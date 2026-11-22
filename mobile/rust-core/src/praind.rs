//! Carnés PRAIND en el teléfono (fase 6 de
//! `docs/features-futuras/plan-fotos-praind.md`). La lógica es la misma del
//! escritorio (`control_acceso::nube::praind`): el carné se baja recién al
//! pedirlo y queda guardado en la base cifrada; después se ve sin internet.

use control_acceso::nube::praind::{self, ErrorPraind};

use crate::{Nucleo, NucleoError};

/// Estado del carné de un contratista en este teléfono.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct EstadoCarnePraind {
    /// Tiene carné cargado (si no, no se ofrece "Ver carné").
    pub tiene_carne: bool,
    /// Ya está guardado en el teléfono: se ve al instante y sin internet.
    pub guardado: bool,
}

/// El carné listo para mostrar.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct CarnePraind {
    pub tipo_mime: String,
    pub bytes: Vec<u8>,
}

/// Lo que el teléfono puede hacer con los carnés, según la nube.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct PermisosCarnePraind {
    pub puede_cargar: bool,
    pub sin_conexion: bool,
}

/// Resultado de cargar (o previsualizar) un carné por el QR.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct CargaCarnePraind {
    /// `None` en la previsualización (no se guardó nada).
    pub clave: Option<String>,
    /// Nombre de la persona según la página del QR.
    pub nombre_en_origen: Option<String>,
    /// Si ese nombre coincide con el del contratista (regla compartida).
    pub nombre_coincide: bool,
}

/// Vista previa del gafete del QR con su imagen, sin guardar nada: el
/// teléfono la lee con OCR y la revisa contra la ficha antes de guardar.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PreviaGafetePraind {
    pub nombre_en_origen: Option<String>,
    pub nombre_coincide: bool,
    /// La imagen del gafete (PNG o JPEG) para el OCR.
    pub imagen: Vec<u8>,
}

/// Cómo se ve una línea de la revisión.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum NivelRevisionGafete {
    Bien,
    Aviso,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct LineaRevisionGafete {
    pub nivel: NivelRevisionGafete,
    pub texto: String,
}

/// El gafete leído con OCR contra la ficha: las líneas para mostrar (cédula,
/// nombre, empresa, vencimiento), si se puede guardar y, si el gafete es
/// más nuevo, la fecha que se deja en la ficha al guardar (`AAAA-MM-DD`).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RevisionGafetePraind {
    pub lineas: Vec<LineaRevisionGafete>,
    pub bloquea_guardar: bool,
    pub fecha_para_ficha: Option<String>,
    /// Si el nombre difiere: como quedaría la ficha si el portero elige
    /// corregirla con el del gafete.
    pub nombre_para_ficha: Option<String>,
    /// Si la empresa difiere y la del gafete es una del catálogo.
    pub empresa_para_ficha: Option<EmpresaParaFicha>,
    /// Cómo se guarda: lo que dice el botón
    /// (`domain::gafete_praind::RevisionGafetePraind::modo_guardado`).
    pub modo_guardado: ModoGuardadoGafete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ModoGuardadoGafete {
    /// Cédula distinta: no se puede guardar.
    NoSePuede,
    /// Vencido: sólo como imagen ilustrativa.
    SoloImagen,
    /// Algún dato no coincide o no se leyó: se guarda igual.
    ConAvisos,
    Normal,
}

/// El gafete quedó guardado. `aviso` es lo que se muestra: con
/// `aviso_es_error`, el gafete se guardó pero la ficha no se pudo corregir.
/// Con `ficha_cambio` hay un cambio local para subir.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct GafeteGuardado {
    pub aviso: String,
    pub aviso_es_error: bool,
    pub ficha_cambio: bool,
}

/// El alta quedó guardada; `aviso` dice si el gafete del QR se cargó o por
/// qué no (`None` si no había gafete del QR).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct AltaConGafete {
    pub contratista_id: i64,
    pub aviso: Option<String>,
    pub aviso_es_error: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct EmpresaParaFicha {
    pub id: i64,
    pub nombre: String,
}

/// Vista previa del gafete del QR para dar de alta a un contratista nuevo:
/// todavía no hay ficha con qué compararlo.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PreviaGafeteParaAlta {
    pub nombre_en_origen: Option<String>,
    /// La imagen del gafete (PNG o JPEG) para el OCR.
    pub imagen: Vec<u8>,
}

/// Lo que el gafete leído aporta al formulario de alta, ya como lo guarda
/// la ficha (`control_acceso::domain::gafete_praind::datos_alta_desde_gafete`).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct DatosAltaDesdeGafete {
    pub cedula: Option<String>,
    pub nombre: Option<String>,
    /// La empresa tal como la dice el gafete.
    pub empresa: Option<String>,
    /// La del catálogo que le corresponde, si hay exactamente una.
    pub empresa_catalogo: Option<EmpresaParaFicha>,
    /// `AAAA-MM-DD`; `None` si no se leyó o la lectura no es coherente.
    pub vencimiento: Option<String>,
    pub vencido: bool,
    /// Esa cédula ya tiene ficha: se ofrece actualizarla con el gafete en
    /// vez de darla de alta otra vez.
    pub ya_registrado: Option<ContratistaYaRegistrado>,
    /// Lo que dice el formulario después de leerlo
    /// (`mensajes::mensaje_alta_desde_gafete`), y si es un error.
    pub mensaje: String,
    pub mensaje_es_error: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ContratistaYaRegistrado {
    pub id: i64,
    pub nombre: String,
}

fn fecha_de_ocr(fecha: crate::lectura_documentos::FechaOcr) -> Option<chrono::NaiveDate> {
    chrono::NaiveDate::from_ymd_opt(fecha.anio, u32::from(fecha.mes), u32::from(fecha.dia))
}

/// El texto del OCR, partido en los datos del gafete.
fn gafete_leido(texto_ocr: &str) -> control_acceso::domain::gafete_praind::DatosGafeteLeido {
    let leido = crate::lectura_documentos::leer_gafete_praind(texto_ocr);
    control_acceso::domain::gafete_praind::DatosGafeteLeido {
        cedula: leido.cedula,
        nombre: leido.nombre,
        empresa: leido.empresa,
        induccion: leido.induccion.and_then(fecha_de_ocr),
        vencimiento: leido.vencimiento.and_then(fecha_de_ocr),
    }
}

fn rechazo(mensaje: String) -> NucleoError {
    NucleoError::Rechazado { mensaje }
}

fn nombre_contratista(conexion: &rusqlite::Connection, id: i64) -> Result<String, NucleoError> {
    conexion
        .query_row(
            "SELECT nombre FROM contratistas WHERE id = ?1",
            [id],
            |fila| fila.get(0),
        )
        .map_err(|_| rechazo("El contratista no existe.".to_string()))
}

use praind::nombres_coinciden;

impl Nucleo {
    fn token_praind(&self) -> Result<String, ErrorPraind> {
        self.autenticar_con_cache()
            .map(|token| token.access_token)
            .map_err(|error| {
                if error.es_falta_de_conexion() {
                    ErrorPraind::SinConexion
                } else {
                    ErrorPraind::Rechazo {
                        estado: 401,
                        mensaje: control_acceso::mensajes::mensaje_nube(error),
                    }
                }
            })
    }

    /// Guarda el gafete del QR de un contratista recién dado de alta. Si
    /// todavía no llegó a la nube, primero sube los cambios pendientes de
    /// este teléfono (el alta) y recién después carga el gafete.
    fn cargar_gafete_praind_de_alta(
        &self,
        contratista_id: i64,
        enlace: String,
    ) -> Result<CargaCarnePraind, NucleoError> {
        self.actor_autenticado()?;
        let sin_subir = praind::estado_carne(&self.conexion_secundaria()?, contratista_id)
            .map_err(rechazo)?
            .uuid
            .is_none();
        if sin_subir {
            // Si falla, la carga de abajo avisa que todavía no llegó a la nube.
            let _ = self.enviar_cambios();
        }
        self.cargar_carne_praind_desde_qr(contratista_id, enlace, false)
    }
}

#[uniffi::export]
impl Nucleo {
    /// Si el contratista tiene carné y si ya está en el teléfono. Sin red.
    pub fn estado_carne_praind(
        &self,
        contratista_id: i64,
    ) -> Result<EstadoCarnePraind, NucleoError> {
        self.actor_autenticado()?;
        let carne =
            praind::estado_carne(&self.conexion_secundaria()?, contratista_id).map_err(rechazo)?;
        Ok(EstadoCarnePraind {
            tiene_carne: carne.clave.is_some(),
            guardado: carne.guardado,
        })
    }

    /// El carné: del teléfono si ya se vio; si no, de la nube (y queda
    /// guardado). Sin internet y sin haberlo visto antes, avisa que hace
    /// falta conexión.
    pub fn ver_carne_praind(&self, contratista_id: i64) -> Result<CarnePraind, NucleoError> {
        self.actor_autenticado()?;
        let carne = praind::ver_carne(&self.conexion_secundaria()?, contratista_id, || {
            self.token_praind()
        })
        .map_err(rechazo)?;
        Ok(CarnePraind {
            tipo_mime: carne.tipo_mime,
            bytes: carne.bytes,
        })
    }

    /// Si la nube deja cargar carnés desde este teléfono (lo decide el panel).
    pub fn permisos_carne_praind(&self) -> PermisosCarnePraind {
        let resultado = self.token_praind().and_then(|token| {
            praind::permisos(
                control_acceso::nube::base_url(),
                control_acceso::nube::apikey(),
                &token,
            )
        });
        match resultado {
            Ok(permisos) => PermisosCarnePraind {
                puede_cargar: permisos.puede_cargar,
                sin_conexion: false,
            },
            Err(error) => PermisosCarnePraind {
                puede_cargar: false,
                sin_conexion: error.es_sin_conexion(),
            },
        }
    }

    /// Carga el carné con el enlace leído del QR. Con `solo_previsualizar`
    /// no guarda nada: devuelve de quién es para confirmarlo antes.
    pub fn cargar_carne_praind_desde_qr(
        &self,
        contratista_id: i64,
        enlace: String,
        solo_previsualizar: bool,
    ) -> Result<CargaCarnePraind, NucleoError> {
        self.actor_autenticado()?;
        let conexion = self.conexion_secundaria()?;
        let nombre = nombre_contratista(&conexion, contratista_id)?;
        let carga = praind::cargar_carne_desde_qr(
            &conexion,
            contratista_id,
            &enlace,
            solo_previsualizar,
            || self.token_praind(),
        )
        .map_err(rechazo)?;
        Ok(CargaCarnePraind {
            nombre_coincide: nombres_coinciden(carga.nombre_en_origen.as_deref(), &nombre),
            clave: carga.clave,
            nombre_en_origen: carga.nombre_en_origen,
        })
    }

    /// Vista previa del gafete del QR **con su imagen** (para el OCR), sin
    /// guardar nada.
    pub fn previsualizar_carne_praind_con_imagen(
        &self,
        contratista_id: i64,
        enlace: String,
    ) -> Result<PreviaGafetePraind, NucleoError> {
        self.actor_autenticado()?;
        let conexion = self.conexion_secundaria()?;
        let nombre = nombre_contratista(&conexion, contratista_id)?;
        let (carga, imagen) =
            praind::previsualizar_carne_con_imagen(&conexion, contratista_id, &enlace, || {
                self.token_praind()
            })
            .map_err(rechazo)?;
        Ok(PreviaGafetePraind {
            nombre_coincide: nombres_coinciden(carga.nombre_en_origen.as_deref(), &nombre),
            nombre_en_origen: carga.nombre_en_origen,
            imagen,
        })
    }

    /// Vista previa del gafete del QR con su imagen para dar de alta a un
    /// contratista nuevo (todavía no existe). No guarda nada.
    pub fn previsualizar_gafete_praind_para_alta(
        &self,
        enlace: String,
    ) -> Result<PreviaGafeteParaAlta, NucleoError> {
        self.actor_autenticado()?;
        let (carga, imagen) =
            praind::previsualizar_gafete_para_alta(&enlace, || self.token_praind())
                .map_err(rechazo)?;
        Ok(PreviaGafeteParaAlta {
            nombre_en_origen: carga.nombre_en_origen,
            imagen,
        })
    }

    /// Los datos del gafete leído con OCR para rellenar el formulario de
    /// alta. Si el OCR no encontró el nombre, sirve el de la página del QR.
    /// Sólo lee.
    pub fn datos_alta_desde_gafete_praind(
        &self,
        texto_ocr: String,
        nombre_en_origen: Option<String>,
    ) -> Result<DatosAltaDesdeGafete, NucleoError> {
        let datos = self
            .core_lock()
            .datos_alta_desde_gafete(&gafete_leido(&texto_ocr), nombre_en_origen.as_deref())?;
        let (mensaje, mensaje_es_error) =
            control_acceso::mensajes::mensaje_alta_desde_gafete(&datos);
        Ok(DatosAltaDesdeGafete {
            mensaje,
            mensaje_es_error,
            cedula: datos.cedula,
            nombre: datos.nombre,
            empresa: datos.empresa,
            empresa_catalogo: datos.empresa_catalogo.map(|empresa| EmpresaParaFicha {
                id: empresa.id,
                nombre: empresa.nombre,
            }),
            vencimiento: datos
                .vencimiento
                .map(|fecha| fecha.format("%Y-%m-%d").to_string()),
            vencido: datos.vencido,
            ya_registrado: datos
                .ya_registrado
                .map(|registrado| ContratistaYaRegistrado {
                    id: registrado.id,
                    nombre: registrado.nombre,
                }),
        })
    }

    /// Da de alta al contratista y, si se leyó el QR de su gafete, lo carga.
    /// El gafete sólo se carga si es de esa cédula
    /// (`domain::gafete_praind::gafete_es_de_cedula`): si la cambiaron a
    /// mano por otra, sería de otra persona. Si la carga falla, el alta
    /// queda igual y el aviso lo dice (se carga desde su ingreso).
    pub fn crear_contratista_con_gafete(
        &self,
        datos: crate::DatosContratista,
        enlace: Option<String>,
        cedula_gafete: Option<String>,
    ) -> Result<AltaConGafete, NucleoError> {
        use control_acceso::domain::gafete_praind::gafete_es_de_cedula;
        use control_acceso::mensajes::{
            MENSAJE_GAFETE_DE_OTRA_CEDULA, mensaje_alta_sin_gafete, mensaje_gafete_guardado,
        };

        let cedula = datos.cedula.clone();
        let contratista_id = self.crear_contratista(datos)?;
        let Some(enlace) = enlace else {
            return Ok(AltaConGafete {
                contratista_id,
                aviso: None,
                aviso_es_error: false,
            });
        };
        let (aviso, aviso_es_error) = if gafete_es_de_cedula(cedula_gafete.as_deref(), &cedula) {
            match self.cargar_gafete_praind_de_alta(contratista_id, enlace) {
                Ok(_) => {
                    let nombre = nombre_contratista(&self.conexion_secundaria()?, contratista_id)?;
                    (mensaje_gafete_guardado(&nombre, None, false), false)
                }
                Err(error) => (mensaje_alta_sin_gafete(Some(&error.to_string())), true),
            }
        } else {
            (MENSAJE_GAFETE_DE_OTRA_CEDULA.to_string(), true)
        };
        Ok(AltaConGafete {
            contratista_id,
            aviso: Some(aviso),
            aviso_es_error,
        })
    }

    /// Guarda el gafete del QR y corrige la ficha con él, en este orden:
    /// primero el gafete (si no se puede, la ficha no cambia y se puede
    /// reintentar) y después la ficha, en una sola edición: el vencimiento
    /// si el gafete es más nuevo (lo decide la revisión) y, si se pidió, el
    /// nombre y la empresa del gafete. Vuelve a revisar el gafete contra la
    /// ficha de este momento: si la cédula no coincide no guarda nada. La
    /// usan "Cargar gafete PRAIND" (con las casillas) y la actualización
    /// desde el alta (con todo).
    #[allow(clippy::fn_params_excessive_bools)]
    pub fn guardar_gafete_praind_revisado(
        &self,
        contratista_id: i64,
        enlace: String,
        texto_ocr: String,
        nombre_en_origen: Option<String>,
        corregir_nombre: bool,
        corregir_empresa: bool,
    ) -> Result<GafeteGuardado, NucleoError> {
        use control_acceso::mensajes::{
            MENSAJE_GAFETE_NO_ES_DE_LA_FICHA, mensaje_gafete_guardado,
            mensaje_gafete_guardado_ficha_sin_corregir,
        };

        let actor = self.actor_autenticado()?;
        let leido = gafete_leido(&texto_ocr).con_nombre_de_pagina(nombre_en_origen.as_deref());
        let revision = self
            .core_lock()
            .revisar_gafete_praind(contratista_id, &leido)?;
        if revision.bloquea_guardar() {
            return Err(rechazo(MENSAJE_GAFETE_NO_ES_DE_LA_FICHA.to_string()));
        }
        self.cargar_gafete_praind_de_alta(contratista_id, enlace)?;

        let correccion = control_acceso::application::CorreccionDesdeGafete {
            vencimiento: revision.fecha_para_ficha,
            nombre: revision.nombre_para_ficha.filter(|_| corregir_nombre),
            empresa_id: revision
                .empresa_para_ficha
                .map(|empresa| empresa.id)
                .filter(|_| corregir_empresa),
        };
        let vencimiento = correccion.vencimiento;
        let corrige_datos = correccion.nombre.is_some() || correccion.empresa_id.is_some();
        let ficha_cambio = vencimiento.is_some() || corrige_datos;
        if ficha_cambio {
            let resultado =
                self.core_lock()
                    .corregir_ficha_desde_gafete(&actor, contratista_id, correccion);
            if let Err(error) = resultado {
                return Ok(GafeteGuardado {
                    aviso: mensaje_gafete_guardado_ficha_sin_corregir(
                        &NucleoError::from(error).to_string(),
                    ),
                    aviso_es_error: true,
                    ficha_cambio: false,
                });
            }
        }
        let nombre = nombre_contratista(&self.conexion_secundaria()?, contratista_id)?;
        Ok(GafeteGuardado {
            aviso: mensaje_gafete_guardado(&nombre, vencimiento, corrige_datos),
            aviso_es_error: false,
            ficha_cambio,
        })
    }

    /// Revisa el texto que el OCR leyó del gafete contra la ficha del
    /// contratista (`control_acceso::domain::gafete_praind`). El nombre que
    /// se compara es el de la página del QR si la trajo (más confiable que
    /// el OCR). Sólo lee.
    pub fn revisar_gafete_praind(
        &self,
        contratista_id: i64,
        texto_ocr: String,
        nombre_en_origen: Option<String>,
    ) -> Result<RevisionGafetePraind, NucleoError> {
        use control_acceso::domain::gafete_praind::ModoGuardadoGafete as Modo;
        use control_acceso::mensajes::{
            NivelRevisionGafete as Nivel, lineas_revision_gafete_praind,
        };

        let leido = gafete_leido(&texto_ocr).con_nombre_de_pagina(nombre_en_origen.as_deref());
        let revision = self
            .core_lock()
            .revisar_gafete_praind(contratista_id, &leido)?;
        let modo_guardado = match revision.modo_guardado() {
            Modo::NoSePuede => ModoGuardadoGafete::NoSePuede,
            Modo::SoloImagen => ModoGuardadoGafete::SoloImagen,
            Modo::ConAvisos => ModoGuardadoGafete::ConAvisos,
            Modo::Normal => ModoGuardadoGafete::Normal,
        };
        Ok(RevisionGafetePraind {
            lineas: lineas_revision_gafete_praind(&revision)
                .into_iter()
                .map(|linea| LineaRevisionGafete {
                    nivel: match linea.nivel {
                        Nivel::Bien => NivelRevisionGafete::Bien,
                        Nivel::Aviso => NivelRevisionGafete::Aviso,
                        Nivel::Error => NivelRevisionGafete::Error,
                    },
                    texto: linea.texto,
                })
                .collect(),
            bloquea_guardar: revision.bloquea_guardar(),
            fecha_para_ficha: revision
                .fecha_para_ficha
                .map(|fecha| fecha.format("%Y-%m-%d").to_string()),
            nombre_para_ficha: revision.nombre_para_ficha,
            modo_guardado,
            empresa_para_ficha: revision.empresa_para_ficha.map(|empresa| EmpresaParaFicha {
                id: empresa.id,
                nombre: empresa.nombre,
            }),
        })
    }
}

/// Si el gafete leído es de esta cédula (sólo cuentan los dígitos; sin
/// cédula en el gafete no hay con qué contrastar). La regla es del dominio
/// (`domain::gafete_praind::gafete_es_de_cedula`): el formulario de alta la
/// usa para saber si la cédula escrita sigue siendo la del gafete.
#[uniffi::export]
pub fn gafete_es_de_cedula(cedula_gafete: Option<String>, cedula: String) -> bool {
    control_acceso::domain::gafete_praind::gafete_es_de_cedula(cedula_gafete.as_deref(), &cedula)
}
