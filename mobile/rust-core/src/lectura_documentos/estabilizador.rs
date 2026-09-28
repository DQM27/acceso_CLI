//! Decide, frame a frame, si ya hay lectura suficiente de un documento
//! para aceptarla. Portado de `EstabilizadorLectura.kt` (A-1).
//!
//! **Votación por carácter.** Cada frame vota posición por posición
//! ([`VotadorPorPosicion`]) con un peso según su nitidez, y se confirma
//! cuando, en TODAS las posiciones, el carácter ganador tiene respaldo de
//! al menos `frames_requeridos` frames (descontando [`TOLERANCIA_SOPORTE`])
//! y le saca a la segunda opción al menos [`MARGEN_MINIMO`]. Con frames de
//! peso 1 nunca es más permisivo que exigir N lecturas iguales, incluido el
//! modo gafete, que dispara una salida sin revisión humana. Una lectura que
//! difiere en más de un par de caracteres es OTRO documento y se empieza de
//! cero: la votación corrige errores, nunca mezcla dos personas.
//!
//! El MRZ se vota igual (sus líneas completas) y el consenso se valida con
//! sus dígitos verificadores. El PDF417 confirma en una sola lectura.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use super::identidad::{
    DocumentoLeido, FuenteDatos, TipoDocumento, clasificar, documento_desde_mrz, leer_documento,
    nombre_legible_tipo_documento, reclasificar_por_edad,
};
use super::mrz_texto::{LecturaMrz, lectura_mrz_de_textos};
use super::texto::{FechaOcr, en_blanco, largo};
use crate::mrz::{CorreccionAplicada, leer_mrz};
use crate::pdf417_cedula::DatosPdf417Cedula;
use crate::votacion::{ConsensoVotacion, VotadorPorPosicion};

/// Estado que alimenta los esquineros del visor y el mensaje de la cámara.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum EstadoEscaneo {
    Buscando,
    Invalido,
    Confirmado,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ModoEscaneoDocumento {
    /// Documento del contratista: abre un formulario que el operador revisa.
    DocumentoContratista,
    /// Gafete de contratista: confirmar registra una salida sin revisión.
    GafeteContratista,
}

/// Orientación del recuadro guía.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum OrientacionEncuadre {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct ResultadoEstabilizacion {
    pub estado: EstadoEscaneo,
    pub documento: Option<DocumentoLeido>,
    pub mensaje: String,
    /// Sólo con estado confirmado y fecha de vencimiento disponible;
    /// `false` no significa "vigente confirmado".
    pub vencido: bool,
    /// 0..1: respaldo del candidato actual sobre el necesario (barra de
    /// avance del recuadro: "ya casi, no lo mueva").
    pub progreso: f32,
    /// Orientación del recuadro que pide lo leído (`None` = no opina).
    pub orientacion_sugerida: Option<OrientacionEncuadre>,
    /// Hay un MRZ en cuadro: el lector de códigos no tiene nada que buscar.
    pub hay_mrz: bool,
    /// Correcciones de confusables del MRZ confirmado (para Sentry); vacía
    /// en cualquier otro caso.
    pub correcciones_mrz: Vec<CorreccionAplicada>,
}

impl ResultadoEstabilizacion {
    fn nuevo(estado: EstadoEscaneo, mensaje: impl Into<String>) -> Self {
        Self {
            estado,
            documento: None,
            mensaje: mensaje.into(),
            vencido: false,
            progreso: 0.0,
            orientacion_sugerida: None,
            hay_mrz: false,
            correcciones_mrz: Vec::new(),
        }
    }

    fn con_orientacion(mut self, orientacion: Option<OrientacionEncuadre>) -> Self {
        self.orientacion_sugerida = orientacion;
        self
    }

    fn con_mrz(mut self) -> Self {
        self.hay_mrz = true;
        self
    }
}

/// Parámetros de una sesión de escaneo (los valores por defecto los pone
/// quien crea el estabilizador; ver `EstabilizadorLectura.kt`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct ConfiguracionEstabilizador {
    pub modo: ModoEscaneoDocumento,
    /// Frames (completos) que deben respaldar la lectura.
    pub frames_requeridos: u32,
    /// Frames recientes que se consideran (tolera alguno malo salteado).
    pub ventana: u32,
    /// Con el número de la cédula leído del frente pero sin nombre, cuántos
    /// frames se espera a que se muestre el reverso antes de confirmar sólo
    /// con el número (confirmar cierra la cámara).
    pub frames_espera_reverso: u32,
    /// Frames SEGUIDOS no reconocibles antes de marcar inválido (con 1,
    /// cualquier frame de transición hacía parpadear el rojo).
    pub frames_para_invalido: u32,
    /// Frames con el tipo reconocido pero sin datos antes de sugerir
    /// inclinar el documento.
    pub frames_para_sugerir_reflejo: u32,
}

const VENTANA_LECTURAS_MRZ: u32 = 6;
const LECTURAS_MRZ_COINCIDENTES: u32 = 2;
const SEPARADOR_LINEAS_MRZ: &str = "\n";
/// Un frame apenas menos nítido que el mejor pesa algo menos de 1: sin esta
/// tolerancia 2 frames buenos (1,0 + 0,9) no alcanzarían "2".
const TOLERANCIA_SOPORTE: f32 = 0.5;
/// Ventaja mínima del ganador sobre la segunda opción: un empate nunca
/// confirma.
const MARGEN_MINIMO: f32 = 0.5;
/// Fracción de píxeles saturados a partir de la cual el reflejo tapa texto,
/// y cuántos frames seguidos antes de decirlo.
const UMBRAL_REFLEJO: f32 = 0.03;
const FRAMES_REFLEJO_PARA_AVISAR: u32 = 3;
/// Menos que esto no es texto de un documento todavía.
const LARGO_MINIMO_TEXTO: usize = 10;

const MENSAJE_ACERQUE: &str = "Acerque el documento";
const MENSAJE_BUSCANDO: &str = "Buscando un documento…";
const MENSAJE_LEYENDO_REVERSO: &str = "Leyendo el reverso — mantenga firme";
const MENSAJE_FALTA_REVERSO: &str = "Ya tengo el número — muéstreme la otra cara para el nombre";
const MENSAJE_REFLEJO_MEDIDO: &str = "Hay reflejo — incline un poco el documento";

/// ¿El consenso alcanza para confirmar con `requeridos` frames?
#[uniffi::export]
pub fn consenso_alcanza(consenso: ConsensoVotacion, requeridos: u32) -> bool {
    alcanza(&consenso, requeridos)
}

fn alcanza(consenso: &ConsensoVotacion, requeridos: u32) -> bool {
    let requeridos = f32::from(u16::try_from(requeridos).unwrap_or(u16::MAX));
    consenso.soporte_minimo >= requeridos - TOLERANCIA_SOPORTE
        && consenso.margen_minimo >= MARGEN_MINIMO
}

fn diferencias(a: &str, b: &str) -> usize {
    a.chars().zip(b.chars()).filter(|(x, y)| x != y).count()
}

/// ¿`nueva` es la lectura de OTRO documento y no el mismo con algún
/// carácter mal leído? Distinto largo, o más diferencias de las que explica
/// un error de lectura (un par; en números cortos, como un gafete, una).
#[uniffi::export]
pub fn es_otra_lectura(consenso: String, nueva: String) -> bool {
    largo(&consenso) != largo(&nueva)
        || diferencias(&consenso, &nueva) > (largo(&consenso) / 4).max(1)
}

/// Mismo criterio para las líneas del MRZ (72-92 caracteres): otra persona
/// difiere en número, fechas y nombres a la vez.
fn es_otra_lectura_mrz(consenso: &str, nueva: &str) -> bool {
    largo(consenso) != largo(nueva) || diferencias(consenso, nueva) > largo(consenso) / 5
}

/// Orientación que pide cada tipo: el carnet PRAIND, el gafete In House y
/// el gafete CRC se sostienen VERTICALES; el resto son tarjetas
/// horizontales. `None` para lo que no se reconoce.
#[uniffi::export]
pub fn orientacion_encuadre_de_tipo(tipo: TipoDocumento) -> Option<OrientacionEncuadre> {
    match tipo {
        TipoDocumento::CarnetInduccionPraind
        | TipoDocumento::CarnetInHouse
        | TipoDocumento::GafeteContratista => Some(OrientacionEncuadre::Vertical),
        TipoDocumento::Desconocido => None,
        _ => Some(OrientacionEncuadre::Horizontal),
    }
}

/// Orientación que pide un texto de la pantalla de documentos: el MRZ no se
/// clasifica por palabras clave, pero es claramente horizontal.
#[uniffi::export]
pub fn orientacion_de_texto_documento(texto: String) -> Option<OrientacionEncuadre> {
    if texto.contains("<<") {
        Some(OrientacionEncuadre::Horizontal)
    } else {
        orientacion_encuadre_de_tipo(clasificar(&texto))
    }
}

fn es_valido_para_modo(tipo: TipoDocumento, modo: ModoEscaneoDocumento) -> bool {
    match modo {
        ModoEscaneoDocumento::DocumentoContratista => !matches!(
            tipo,
            TipoDocumento::GafeteContratista | TipoDocumento::Desconocido
        ),
        ModoEscaneoDocumento::GafeteContratista => tipo == TipoDocumento::GafeteContratista,
    }
}

/// Cédula leída sólo del frente, todavía sin nombre: el nombre está en el
/// reverso, por eso se espera a que la persona la voltee.
fn falta_nombre_por_frente(documento: &DocumentoLeido) -> bool {
    documento.tipo == TipoDocumento::CedulaNacional
        && documento.fuente_datos == FuenteDatos::OcrFrente
        && documento.nombre.is_none()
}

/// Lo que se vota: el número (o, en el gafete In House sin cédula, el
/// nombre con que se busca).
fn valor_votado(documento: &DocumentoLeido) -> String {
    documento
        .texto_busqueda
        .as_deref()
        .unwrap_or(&documento.numero_documento)
        .trim()
        .to_owned()
}

fn con_valor_votado(mut documento: DocumentoLeido, valor: &str) -> DocumentoLeido {
    match &documento.texto_busqueda {
        Some(busqueda) => {
            if documento.numero_documento.trim() == busqueda.trim() {
                valor.clone_into(&mut documento.numero_documento);
            }
            documento.texto_busqueda = Some(valor.to_owned());
        }
        None => valor.clone_into(&mut documento.numero_documento),
    }
    documento
}

/// El documento acumulado de frames anteriores (del MISMO documento) con la
/// lectura de un frame nuevo: cada campo opcional se actualiza sólo si el
/// nuevo trae valor. En la cédula "Nombre:" y los apellidos son bloques que
/// ML Kit rara vez lee juntos en un frame.
fn fusionar(acumulado: DocumentoLeido, nuevo: DocumentoLeido) -> DocumentoLeido {
    DocumentoLeido {
        texto_busqueda: nuevo.texto_busqueda.or(acumulado.texto_busqueda),
        nombre: nuevo.nombre.or(acumulado.nombre),
        apellidos: nuevo.apellidos.or(acumulado.apellidos),
        nacionalidad: nuevo.nacionalidad.or(acumulado.nacionalidad),
        empresa: nuevo.empresa.or(acumulado.empresa),
        vencimiento: nuevo.vencimiento.or(acumulado.vencimiento),
        fecha_nacimiento: nuevo.fecha_nacimiento.or(acumulado.fecha_nacimiento),
        sexo: nuevo.sexo.or(acumulado.sexo),
        ..nuevo
    }
}

/// Qué se sacó del texto de un frame (entre sus versiones, ver
/// `textos_de_frame`).
enum LecturaDeTexto {
    /// Un tipo válido para el modo y su documento.
    Documento(TipoDocumento, DocumentoLeido),
    /// Un tipo válido para el modo, pero sus datos todavía no se leen.
    SinDatos(TipoDocumento),
    /// Un tipo reconocido que no corresponde a esta pantalla.
    TipoEquivocado(TipoDocumento),
    Desconocido,
}

fn leer_textos(textos: &[&str], modo: ModoEscaneoDocumento) -> LecturaDeTexto {
    let tipos: Vec<(&str, TipoDocumento)> = textos.iter().map(|t| (*t, clasificar(t))).collect();
    let validos = || {
        tipos
            .iter()
            .filter(|(_, tipo)| es_valido_para_modo(*tipo, modo))
    };
    if let Some((tipo, documento)) =
        validos().find_map(|(t, tipo)| leer_documento(t, *tipo).map(|d| (*tipo, d)))
    {
        return LecturaDeTexto::Documento(tipo, documento);
    }
    if let Some((_, tipo)) = validos().next() {
        return LecturaDeTexto::SinDatos(*tipo);
    }
    tipos
        .iter()
        .find(|(_, tipo)| *tipo != TipoDocumento::Desconocido)
        .map_or(LecturaDeTexto::Desconocido, |(_, tipo)| {
            LecturaDeTexto::TipoEquivocado(*tipo)
        })
}

struct Estado {
    configuracion: ConfiguracionEstabilizador,
    votador_numero: Arc<VotadorPorPosicion>,
    votador_mrz: Arc<VotadorPorPosicion>,
    tipo_acumulado: Option<TipoDocumento>,
    documento_acumulado: Option<DocumentoLeido>,
    /// `None` = no se está esperando el reverso.
    frames_esperando_reverso: Option<u32>,
    frames_desconocidos_seguidos: u32,
    frames_tipo_sin_datos_seguidos: u32,
    frames_con_reflejo_seguidos: u32,
}

/// Una instancia por sesión de escaneo; segura entre hilos (el analizador
/// de la cámara la alimenta y la pantalla la reinicia).
#[derive(uniffi::Object)]
pub struct EstabilizadorDocumento {
    estado: Mutex<Estado>,
}

#[uniffi::export]
impl EstabilizadorDocumento {
    #[uniffi::constructor]
    pub fn new(configuracion: ConfiguracionEstabilizador) -> Arc<Self> {
        let configuracion = ConfiguracionEstabilizador {
            frames_requeridos: configuracion.frames_requeridos.max(1),
            ventana: configuracion
                .ventana
                .max(configuracion.frames_requeridos.max(1)),
            ..configuracion
        };
        Arc::new(Self {
            estado: Mutex::new(Estado {
                configuracion,
                votador_numero: VotadorPorPosicion::new(configuracion.ventana),
                votador_mrz: VotadorPorPosicion::new(VENTANA_LECTURAS_MRZ),
                tipo_acumulado: None,
                documento_acumulado: None,
                frames_esperando_reverso: None,
                frames_desconocidos_seguidos: 0,
                frames_tipo_sin_datos_seguidos: 0,
                frames_con_reflejo_seguidos: 0,
            }),
        })
    }

    /// `textos`: las versiones del texto del frame (ver `textos_de_frame`).
    /// `peso`: calidad relativa del frame en (0, 1]. `fraccion_reflejo`:
    /// medición del recorte, para avisar de un reflejo real.
    pub fn procesar_frame(
        &self,
        textos: Vec<String>,
        peso: f32,
        fraccion_reflejo: Option<f32>,
        hoy: FechaOcr,
    ) -> ResultadoEstabilizacion {
        self.bloquear()
            .procesar_frame(&textos, peso, fraccion_reflejo, hoy)
    }

    /// Cédula anterior leída de su PDF417 (sólo cédula y nombre, ya
    /// validados). Confirma en el acto.
    pub fn procesar_pdf417(
        &self,
        datos: DatosPdf417Cedula,
        hoy: FechaOcr,
    ) -> ResultadoEstabilizacion {
        self.bloquear().procesar_pdf417(datos, hoy)
    }

    pub fn reiniciar(&self) {
        self.bloquear().reiniciar();
    }
}

impl EstabilizadorDocumento {
    fn bloquear(&self) -> MutexGuard<'_, Estado> {
        // Cada operación deja el estado consistente antes de soltar el
        // lock; tras un pánico se puede seguir usando.
        self.estado.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Estado {
    fn modo(&self) -> ModoEscaneoDocumento {
        self.configuracion.modo
    }

    fn reiniciar(&mut self) {
        self.votador_numero.reiniciar();
        self.votador_mrz.reiniciar();
        self.tipo_acumulado = None;
        self.documento_acumulado = None;
        self.frames_esperando_reverso = None;
        self.frames_desconocidos_seguidos = 0;
        self.frames_tipo_sin_datos_seguidos = 0;
        self.frames_con_reflejo_seguidos = 0;
    }

    fn procesar_frame(
        &mut self,
        textos: &[String],
        peso: f32,
        fraccion_reflejo: Option<f32>,
        hoy: FechaOcr,
    ) -> ResultadoEstabilizacion {
        // Cuenta TODO frame (también los vacíos mientras se voltea la
        // tarjeta) para que la espera del reverso tenga un fin real.
        if let Some(frames) = self.frames_esperando_reverso.as_mut() {
            *frames += 1;
        }
        let espera_agotada = self
            .frames_esperando_reverso
            .is_some_and(|f| f >= self.configuracion.frames_espera_reverso);
        self.frames_con_reflejo_seguidos = if fraccion_reflejo.is_some_and(|f| f >= UMBRAL_REFLEJO)
        {
            self.frames_con_reflejo_seguidos + 1
        } else {
            0
        };

        let utiles: Vec<&str> = textos
            .iter()
            .map(String::as_str)
            .filter(|t| !en_blanco(t) && largo(t.trim()) >= LARGO_MINIMO_TEXTO)
            .collect();
        if utiles.is_empty() {
            self.frames_desconocidos_seguidos = 0;
            self.votador_numero.agregar_vacio();
            self.votador_mrz.agregar_vacio();
            if espera_agotada && let Some(r) = self.confirmar_sin_reverso(hoy) {
                return r;
            }
            return ResultadoEstabilizacion::nuevo(EstadoEscaneo::Buscando, MENSAJE_ACERQUE);
        }

        if self.modo() == ModoEscaneoDocumento::DocumentoContratista {
            let propios: Vec<String> = utiles.iter().map(|t| (*t).to_owned()).collect();
            if let Some(lectura) = lectura_mrz_de_textos(&propios, hoy.anio) {
                return self.procesar_lectura_mrz(lectura, peso, hoy, espera_agotada);
            }
        }
        self.votador_mrz.agregar_vacio();
        if espera_agotada && let Some(r) = self.confirmar_sin_reverso(hoy) {
            return r;
        }
        let orientacion_mrz = utiles
            .iter()
            .any(|t| t.contains("<<"))
            .then_some(OrientacionEncuadre::Horizontal);
        self.procesar_lectura_de_texto(
            leer_textos(&utiles, self.modo()),
            orientacion_mrz,
            peso,
            hoy,
        )
    }

    fn procesar_lectura_de_texto(
        &mut self,
        lectura: LecturaDeTexto,
        orientacion_mrz: Option<OrientacionEncuadre>,
        peso: f32,
        hoy: FechaOcr,
    ) -> ResultadoEstabilizacion {
        let orientacion_de = |tipo| orientacion_mrz.or_else(|| orientacion_encuadre_de_tipo(tipo));
        match lectura {
            LecturaDeTexto::Desconocido => {
                self.votador_numero.agregar_vacio();
                self.frames_desconocidos_seguidos += 1;
                let resultado = if self.frames_desconocidos_seguidos
                    >= self.configuracion.frames_para_invalido
                {
                    ResultadoEstabilizacion::nuevo(
                        EstadoEscaneo::Invalido,
                        self.mensaje_no_reconocido(),
                    )
                } else {
                    ResultadoEstabilizacion::nuevo(EstadoEscaneo::Buscando, MENSAJE_BUSCANDO)
                };
                resultado.con_orientacion(orientacion_mrz)
            }
            LecturaDeTexto::TipoEquivocado(tipo) => {
                self.frames_desconocidos_seguidos = 0;
                self.votador_numero.agregar_vacio();
                ResultadoEstabilizacion::nuevo(
                    EstadoEscaneo::Buscando,
                    self.mensaje_tipo_equivocado(tipo),
                )
                .con_orientacion(orientacion_de(tipo))
            }
            LecturaDeTexto::SinDatos(tipo) => {
                // Tipo reconocible, pero el número todavía no sale: lectura
                // parcial (reflejo, ángulo, foco).
                self.frames_desconocidos_seguidos = 0;
                self.votador_numero.agregar_vacio();
                self.frames_tipo_sin_datos_seguidos += 1;
                let sugerir = self.frames_tipo_sin_datos_seguidos
                    >= self.configuracion.frames_para_sugerir_reflejo;
                let base = format!("{} — no lo mueva", nombre_legible_tipo_documento(tipo));
                ResultadoEstabilizacion::nuevo(
                    EstadoEscaneo::Buscando,
                    self.mensaje_lectura_parcial(&base, Some(tipo), sugerir),
                )
                .con_orientacion(orientacion_de(tipo))
            }
            LecturaDeTexto::Documento(tipo, documento) => {
                self.frames_desconocidos_seguidos = 0;
                self.frames_tipo_sin_datos_seguidos = 0;
                self.votar_numero(documento, peso, hoy, orientacion_de(tipo))
            }
        }
    }

    fn votar_numero(
        &mut self,
        documento_de_este_frame: DocumentoLeido,
        peso: f32,
        hoy: FechaOcr,
        orientacion: Option<OrientacionEncuadre>,
    ) -> ResultadoEstabilizacion {
        let valor = valor_votado(&documento_de_este_frame);
        let otro_documento = Some(documento_de_este_frame.tipo) != self.tipo_acumulado
            || self
                .votador_numero
                .consenso()
                .is_some_and(|c| es_otra_lectura(c.texto, valor.clone()));
        if otro_documento {
            self.votador_numero.reiniciar();
            self.documento_acumulado = None;
            self.frames_esperando_reverso = None;
        }
        self.tipo_acumulado = Some(documento_de_este_frame.tipo);
        let acumulado = match self.documento_acumulado.take() {
            Some(previo) => fusionar(previo, documento_de_este_frame),
            None => documento_de_este_frame,
        };
        self.votador_numero.agregar(valor, peso);

        let Some(consenso) = self.votador_numero.consenso() else {
            self.documento_acumulado = Some(acumulado);
            return ResultadoEstabilizacion::nuevo(EstadoEscaneo::Buscando, MENSAJE_BUSCANDO);
        };
        let documento = con_valor_votado(acumulado, &consenso.texto);
        self.documento_acumulado = Some(documento.clone());
        let requeridos = self.configuracion.frames_requeridos;
        let listo = alcanza(&consenso, requeridos);

        if listo && falta_nombre_por_frente(&documento) {
            let esperando = *self.frames_esperando_reverso.get_or_insert(0);
            if esperando < self.configuracion.frames_espera_reverso {
                return ResultadoEstabilizacion::nuevo(
                    EstadoEscaneo::Buscando,
                    MENSAJE_FALTA_REVERSO,
                )
                .con_orientacion(orientacion);
            }
        }
        if listo {
            self.frames_esperando_reverso = None;
            return confirmado(documento, hoy).con_orientacion(orientacion);
        }
        let base = format!(
            "{} — no lo mueva",
            nombre_legible_tipo_documento(documento.tipo)
        );
        let mut resultado = ResultadoEstabilizacion::nuevo(
            EstadoEscaneo::Buscando,
            self.mensaje_lectura_parcial(&base, Some(documento.tipo), false),
        )
        .con_orientacion(orientacion);
        resultado.progreso = (consenso.soporte_minimo
            / f32::from(u16::try_from(requeridos).unwrap_or(u16::MAX)))
        .clamp(0.0, 1.0);
        resultado
    }

    fn procesar_lectura_mrz(
        &mut self,
        lectura: LecturaMrz,
        peso: f32,
        hoy: FechaOcr,
        espera_agotada: bool,
    ) -> ResultadoEstabilizacion {
        let registro = &lectura.registro;
        if registro.numero_documento_extendido_sin_soporte {
            return ResultadoEstabilizacion::nuevo(
                EstadoEscaneo::Invalido,
                "Documento no reconocido",
            )
            .con_mrz();
        }
        // Un MRZ válido de un tipo no soportado es definitivo.
        if registro.checksums_validos
            && !es_valido_para_modo(
                reclasificar_por_edad(documento_desde_mrz(registro.clone()), hoy).tipo,
                self.modo(),
            )
        {
            return ResultadoEstabilizacion::nuevo(
                EstadoEscaneo::Invalido,
                "Documento no soportado",
            )
            .con_mrz();
        }
        let lineas = lectura.lineas.join(SEPARADOR_LINEAS_MRZ);
        if self
            .votador_mrz
            .consenso()
            .is_some_and(|c| es_otra_lectura_mrz(&c.texto, &lineas))
        {
            self.votador_mrz.reiniciar();
        }
        self.votador_mrz.agregar(lineas, peso);
        if let Some(r) = self.confirmar_por_consenso_mrz(hoy) {
            return r;
        }
        if espera_agotada && let Some(r) = self.confirmar_sin_reverso(hoy) {
            return r;
        }
        // MRZ en cuadro sin consenso válido todavía: lectura parcial, no un
        // documento inválido (sin rojo ni vibración, sin borrar el frente).
        ResultadoEstabilizacion::nuevo(
            EstadoEscaneo::Buscando,
            self.mensaje_lectura_parcial(MENSAJE_LEYENDO_REVERSO, None, false),
        )
        .con_orientacion(Some(OrientacionEncuadre::Horizontal))
        .con_mrz()
    }

    fn confirmar_por_consenso_mrz(&mut self, hoy: FechaOcr) -> Option<ResultadoEstabilizacion> {
        let consenso = self.votador_mrz.consenso()?;
        if !alcanza(&consenso, LECTURAS_MRZ_COINCIDENTES) {
            return None;
        }
        let lineas = consenso
            .texto
            .split(SEPARADOR_LINEAS_MRZ)
            .map(str::to_owned)
            .collect();
        let registro = leer_mrz(lineas, hoy.anio);
        if !registro.formato_reconocido
            || !registro.checksums_validos
            || registro.numero_documento_extendido_sin_soporte
        {
            return None;
        }
        let correcciones = registro.correcciones.clone();
        let documento = reclasificar_por_edad(documento_desde_mrz(registro), hoy);
        if !es_valido_para_modo(documento.tipo, self.modo()) {
            return Some(
                ResultadoEstabilizacion::nuevo(EstadoEscaneo::Invalido, "Documento no soportado")
                    .con_mrz(),
            );
        }
        self.reiniciar();
        let mut resultado = confirmado(documento, hoy)
            .con_orientacion(Some(OrientacionEncuadre::Horizontal))
            .con_mrz();
        resultado.correcciones_mrz = correcciones;
        Some(resultado)
    }

    /// Se agotó la espera del reverso sin un MRZ válido: se confirma la
    /// cédula con lo leído del frente.
    fn confirmar_sin_reverso(&mut self, hoy: FechaOcr) -> Option<ResultadoEstabilizacion> {
        let documento = self.documento_acumulado.clone()?;
        self.frames_esperando_reverso = None;
        Some(confirmado(documento, hoy))
    }

    fn procesar_pdf417(
        &mut self,
        datos: DatosPdf417Cedula,
        hoy: FechaOcr,
    ) -> ResultadoEstabilizacion {
        if !es_valido_para_modo(TipoDocumento::CedulaNacional, self.modo()) {
            return ResultadoEstabilizacion::nuevo(
                EstadoEscaneo::Buscando,
                self.mensaje_tipo_equivocado(TipoDocumento::CedulaNacional),
            );
        }
        let mut documento = DocumentoLeido::nuevo(TipoDocumento::CedulaNacional, datos.cedula);
        documento.nombre = Some(datos.nombre);
        documento.apellidos = Some(datos.apellidos);
        documento.fuente_datos = FuenteDatos::Pdf417;
        documento.checksum_valido = Some(true);
        self.reiniciar();
        confirmado(documento, hoy)
    }

    /// Mientras se lee: si la medición ve reflejo sostenido, lo dice.
    fn mensaje_lectura_parcial(
        &self,
        base: &str,
        tipo: Option<TipoDocumento>,
        sugerir_por_tiempo: bool,
    ) -> String {
        if self.frames_con_reflejo_seguidos >= FRAMES_REFLEJO_PARA_AVISAR {
            return MENSAJE_REFLEJO_MEDIDO.to_owned();
        }
        match tipo {
            Some(tipo) if sugerir_por_tiempo => format!(
                "{} — incline un poco para quitar el reflejo",
                nombre_legible_tipo_documento(tipo)
            ),
            _ => base.to_owned(),
        }
    }

    /// Dice QUÉ vio la cámara: así quien opera sabe que tiene en la mano el
    /// objeto equivocado y no que la cámara "no lee".
    fn mensaje_tipo_equivocado(&self, tipo: TipoDocumento) -> String {
        let esperado = match self.modo() {
            ModoEscaneoDocumento::DocumentoContratista => "el documento del contratista",
            ModoEscaneoDocumento::GafeteContratista => "el gafete de contratista",
        };
        format!(
            "Detecté {} — aquí va {esperado}",
            nombre_legible_tipo_documento(tipo)
        )
    }

    fn mensaje_no_reconocido(&self) -> &'static str {
        match self.modo() {
            ModoEscaneoDocumento::DocumentoContratista => "Documento no reconocido",
            ModoEscaneoDocumento::GafeteContratista => "Gafete no reconocido",
        }
    }
}

/// Confirmación, con el vencimiento anunciado en el mismo mensaje: un
/// documento vencido igual se identificó bien (por eso es confirmado y no
/// inválido), pero quien opera lo sabe de inmediato. "Listo: <tipo>" en vez
/// de "<tipo> confirmado": la mayoría de los tipos son femeninos.
fn confirmado(documento: DocumentoLeido, hoy: FechaOcr) -> ResultadoEstabilizacion {
    let nombre_tipo = nombre_legible_tipo_documento(documento.tipo);
    let vencido = documento.vencimiento.filter(|v| v.esta_vencida(hoy));
    let mensaje = match vencido {
        Some(v) => format!("Listo: {nombre_tipo} — VENCIDO el {}", v.a_texto()),
        None if falta_nombre_por_frente(&documento) => {
            format!("Listo: {nombre_tipo} — sin nombre, complételo a mano")
        }
        None => format!("Listo: {nombre_tipo}"),
    };
    let mut resultado = ResultadoEstabilizacion::nuevo(EstadoEscaneo::Confirmado, mensaje);
    resultado.vencido = vencido.is_some();
    resultado.documento = Some(documento);
    resultado
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOY: FechaOcr = FechaOcr {
        dia: 1,
        mes: 1,
        anio: 2026,
    };

    fn estabilizador(
        modo: ModoEscaneoDocumento,
        frames_requeridos: u32,
    ) -> Arc<EstabilizadorDocumento> {
        EstabilizadorDocumento::new(ConfiguracionEstabilizador {
            modo,
            frames_requeridos,
            ventana: frames_requeridos + 2,
            frames_espera_reverso: 20,
            frames_para_invalido: 3,
            frames_para_sugerir_reflejo: 6,
        })
    }

    fn frame(e: &EstabilizadorDocumento, texto: &str) -> ResultadoEstabilizacion {
        e.procesar_frame(vec![texto.to_owned()], 1.0, None, HOY)
    }

    const LICENCIA: &str = "Licencia de Conducir\nNº: 112340567\nVencimiento 03-04-2030";
    const TD1: &str = "C<CRI9998887774<<<<<<<<<<<<<<<\n9001011F3001019NIC<<<<<<<<<<<8\nPEREZ<<MARIA<JOSE<<<<<<<<<<<<<";

    #[test]
    fn licencia_confirma_con_dos_frames() {
        let e = estabilizador(ModoEscaneoDocumento::DocumentoContratista, 2);
        assert_eq!(frame(&e, LICENCIA).estado, EstadoEscaneo::Buscando);
        let r = frame(&e, LICENCIA);
        assert_eq!(r.estado, EstadoEscaneo::Confirmado);
        assert_eq!(r.documento.unwrap().numero_documento, "112340567");
        assert_eq!(r.mensaje, "Listo: Licencia de conducir");
    }

    #[test]
    fn un_texto_que_no_lee_cede_al_siguiente_del_mismo_frame() {
        let e = estabilizador(ModoEscaneoDocumento::DocumentoContratista, 1);
        let r = e.procesar_frame(
            vec!["ruido sin nada útil acá".into(), LICENCIA.into()],
            1.0,
            None,
            HOY,
        );
        assert_eq!(r.estado, EstadoEscaneo::Confirmado);
    }

    #[test]
    fn mrz_confirma_con_dos_frames_y_da_el_nombre() {
        let e = estabilizador(ModoEscaneoDocumento::DocumentoContratista, 2);
        assert!(frame(&e, TD1).hay_mrz);
        let r = frame(&e, TD1);
        assert_eq!(r.estado, EstadoEscaneo::Confirmado);
        let d = r.documento.unwrap();
        assert_eq!(d.tipo, TipoDocumento::CedulaResidencia);
        assert_eq!(d.nombre.as_deref(), Some("MARIA JOSE"));
    }

    #[test]
    fn gafete_exige_tres_frames_y_rechaza_documentos() {
        let e = estabilizador(ModoEscaneoDocumento::GafeteContratista, 3);
        let gafete = "CARNÉ PROVISIONAL\nCRC - 12\nCONTRATISTAS";
        assert_eq!(frame(&e, gafete).estado, EstadoEscaneo::Buscando);
        assert_eq!(frame(&e, gafete).estado, EstadoEscaneo::Buscando);
        assert_eq!(frame(&e, gafete).estado, EstadoEscaneo::Confirmado);
        let r = frame(&e, LICENCIA);
        assert_eq!(
            r.mensaje,
            "Detecté Licencia de conducir — aquí va el gafete de contratista"
        );
    }

    #[test]
    fn vencido_se_anuncia_al_confirmar() {
        let e = estabilizador(ModoEscaneoDocumento::DocumentoContratista, 1);
        let r = frame(
            &e,
            "Licencia de Conducir\nNº: 112340567\nVencimiento 03-04-2020",
        );
        assert!(r.vencido);
        assert_eq!(
            r.mensaje,
            "Listo: Licencia de conducir — VENCIDO el 03-04-2020"
        );
    }

    /// Línea sin palabras (la altura sale de su caja).
    fn linea(texto: &str, x: f32, y: f32) -> crate::LineaOcr {
        crate::LineaOcr {
            texto: texto.to_owned(),
            izquierda: x,
            arriba: y,
            derecha: x + 150.0,
            abajo: y + 20.0,
            palabras: Vec::new(),
        }
    }

    #[test]
    fn los_renglones_visuales_emparejan_etiquetas_y_valores_de_la_cedula() {
        // ML Kit entregó la columna de etiquetas y DESPUÉS la de valores, y
        // ésta en otro orden: con el texto original el "bloque de valores"
        // toma GOMEZ como nombre. Con la geometría cada valor queda junto a
        // su etiqueta.
        let lineas = vec![
            linea("TRIBUNAL SUPREMO DE ELECCIONES", 100.0, 0.0),
            linea("1 2345 6789", 100.0, 50.0),
            linea("Nombre:", 100.0, 100.0),
            linea("1° Apellido:", 100.0, 140.0),
            linea("2° Apellido:", 100.0, 180.0),
            linea("GOMEZ", 300.0, 141.0),
            linea("VARGAS", 300.0, 181.0),
            linea("JUAN CARLOS", 300.0, 99.0),
        ];
        let original = lineas
            .iter()
            .map(|l| l.texto.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let con_original = leer_documento(&original, TipoDocumento::CedulaNacional).unwrap();
        assert_eq!(
            con_original.nombre.as_deref(),
            Some("GOMEZ"),
            "el defecto que corrige E-3"
        );

        let textos = crate::textos_de_frame(original, lineas);
        let e = EstabilizadorDocumento::new(ConfiguracionEstabilizador {
            modo: ModoEscaneoDocumento::DocumentoContratista,
            frames_requeridos: 1,
            ventana: 3,
            frames_espera_reverso: 0,
            frames_para_invalido: 3,
            frames_para_sugerir_reflejo: 6,
        });
        let d = e.procesar_frame(textos, 1.0, None, HOY).documento.unwrap();
        assert_eq!(d.numero_documento, "123456789");
        assert_eq!(d.nombre.as_deref(), Some("JUAN CARLOS"));
        assert_eq!(d.apellidos.as_deref(), Some("GOMEZ VARGAS"));
    }

    #[test]
    fn otra_lectura_segun_largo_y_diferencias() {
        assert!(es_otra_lectura("12".into(), "123".into()));
        assert!(!es_otra_lectura("112340567".into(), "112340561".into()));
        assert!(es_otra_lectura("112340567".into(), "998877567".into()));
        assert!(es_otra_lectura("12".into(), "34".into()));
    }
}
