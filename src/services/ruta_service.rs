//! Orquestación del ciclo salida/retorno de rutas
//! (`docs/planes-implementados/plan-control-rutas.md`, sección
//! "Rediseño del núcleo de rutas -- documento/tramo/viaje", 2026-09-19/20).
//!
//! Tres entidades separadas, donde antes había una sola fila plana:
//! - **Documento** (`documentos_ruta`) -- el comprobante real. `ruta_id`
//!   es `Option` a propósito: el número de ruta es propiedad del
//!   documento (va impreso en cada comprobante), no del camión -- un
//!   tercero puede llevar documentos de varias rutas a la vez, o de
//!   ninguna. Bloqueante *cuando viene* (mismo criterio que antes), pero
//!   nunca obligatorio.
//! - **Salida/tramo** (`salidas_ruta`) -- un cruce físico de portón.
//!   Vehículo/encargado son consultivos, nunca bloqueantes (mismo
//!   criterio de siempre).
//! - **Viaje** (`viajes_ruta`) -- agrupa 1+ tramos de la misma
//!   unidad+encargado el mismo día, con su propio cierre explícito.
//!
//! El vínculo `salida_ruta_documentos` es la pieza que resuelve a la vez
//! "una salida con varios documentos" y "un documento repartido en 2+
//! salidas" (recarga) -- y es donde vive el veredicto de fecha vencida
//! (`resultado`/`motivo_resultado`), evaluado por cada uso, no fijo en
//! el documento (un documento puede reusarse en un tramo de OTRO día).

use chrono::{DateTime, NaiveDate, Utc};

use crate::database::repositories::documento_ruta_repository::DocumentoRutaRepository;
use crate::database::repositories::encargado_ruta_repository::EncargadoRutaRepository;
use crate::database::repositories::ruta_repository::RutaRepository;
use crate::database::repositories::salida_ruta_documento_repository::SalidaRutaDocumentoRepository;
use crate::database::repositories::salida_ruta_repository::SalidaRutaRepository;
use crate::database::repositories::vehiculo_ruta_repository::VehiculoRutaRepository;
use crate::database::repositories::viaje_ruta_repository::ViajeRutaRepository;
use crate::domain::resultado_salida_ruta::verificar_fecha_documento;
use crate::domain::viaje_ruta::DecisionRetornoViaje;
use crate::models::documento_ruta::NuevoDocumentoRuta;
use crate::models::salida_ruta::{NuevaSalidaRuta, SalidaRuta, SalidaRutaActivaResumen};
use crate::models::viaje_ruta::{EstadoViaje, NuevoViajeRuta, ViajeRuta};
use crate::tiempo::fecha_costa_rica;

use super::error::{RutaCatalogoServiceError, RutaServiceError};

/// Un documento declarado como parte de una solicitud de salida -- 1+
/// por solicitud (confirmado: todos se declaran juntos, al momento de
/// la salida). `numero_ruta` es `Option`: `None` para un documento de
/// tercero sin ruta de catálogo válida detrás de la carga.
#[derive(Debug, Clone)]
pub struct SolicitudDocumentoRuta {
    pub numero_documento: String,
    pub numero_ruta: Option<i64>,
    pub sub_numero: i64,
    pub fecha_documento: NaiveDate,
}

/// Datos crudos de una salida (checklist mobile o formulario de
/// escritorio) -- ver el doc-comment del módulo.
///
/// `continuar_viaje_id`: `Some` cuando el guardia declaró "sí, misma
/// ruta" al confirmar un retorno anterior -- el tramo nuevo se abre
/// dentro de ESE viaje en vez de crear uno. `None` abre un viaje nuevo
/// (primera salida del día para esta unidad, o "otra ruta"/tercero).
#[derive(Debug, Clone)]
pub struct SolicitudSalidaRuta {
    pub vehiculo_placa: String,
    pub vehiculo_numero_unidad: Option<String>,
    pub encargado_nombre: String,
    pub encargado_codigo_empleado: Option<String>,
    pub documentos: Vec<SolicitudDocumentoRuta>,
    pub continuar_viaje_id: Option<i64>,
    /// Declaración del guardia (checkbox en pantalla) -- aplica a todos
    /// los documentos de esta solicitud, mismo criterio que antes.
    pub tiene_correo_autorizacion: bool,
    pub usuario_salida_id: i64,
    pub fecha_hora_salida: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResultadoRegistroSalidaRuta {
    pub salida_id: i64,
    pub viaje_id: i64,
}

pub struct RutaService<'a, S, V, E, R, D, VJ, SD>
where
    S: SalidaRutaRepository + ?Sized,
    V: VehiculoRutaRepository + ?Sized,
    E: EncargadoRutaRepository + ?Sized,
    R: RutaRepository + ?Sized,
    D: DocumentoRutaRepository + ?Sized,
    VJ: ViajeRutaRepository + ?Sized,
    SD: SalidaRutaDocumentoRepository + ?Sized,
{
    salidas: &'a S,
    vehiculos: &'a V,
    encargados: &'a E,
    rutas: &'a R,
    documentos: &'a D,
    viajes: &'a VJ,
    vinculos: &'a SD,
}

impl<'a, S, V, E, R, D, VJ, SD> RutaService<'a, S, V, E, R, D, VJ, SD>
where
    S: SalidaRutaRepository + ?Sized,
    V: VehiculoRutaRepository + ?Sized,
    E: EncargadoRutaRepository + ?Sized,
    R: RutaRepository + ?Sized,
    D: DocumentoRutaRepository + ?Sized,
    VJ: ViajeRutaRepository + ?Sized,
    SD: SalidaRutaDocumentoRepository + ?Sized,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        salidas: &'a S,
        vehiculos: &'a V,
        encargados: &'a E,
        rutas: &'a R,
        documentos: &'a D,
        viajes: &'a VJ,
        vinculos: &'a SD,
    ) -> Self {
        Self {
            salidas,
            vehiculos,
            encargados,
            rutas,
            documentos,
            viajes,
            vinculos,
        }
    }

    /// Este servicio es independiente de `SQLite` y no abre transacciones --
    /// mismo contrato que `RegistroIngresoService::registrar_entrada`: el
    /// adaptador productivo debe construir los siete repositorios sobre la
    /// misma unidad de trabajo `IMMEDIATE`.
    pub fn registrar_salida(
        &self,
        solicitud: &SolicitudSalidaRuta,
    ) -> Result<ResultadoRegistroSalidaRuta, RutaServiceError> {
        let vehiculo_placa = solicitud.vehiculo_placa.trim();
        if vehiculo_placa.is_empty() {
            return Err(RutaServiceError::PlacaVacia);
        }
        let encargado_nombre = solicitud.encargado_nombre.trim();
        if encargado_nombre.is_empty() {
            return Err(RutaServiceError::EncargadoVacio);
        }
        if solicitud.documentos.is_empty() {
            return Err(RutaServiceError::SinDocumentos);
        }

        if self
            .salidas
            .buscar_activa_por_placa(vehiculo_placa)?
            .is_some()
        {
            return Err(RutaServiceError::VehiculoYaEnRuta);
        }

        // Match de catálogo -- consultivo, nunca bloquea. Vehículo por
        // placa (identificador único y confiable); encargado sólo por
        // código de empleado, nunca por nombre.
        let vehiculo_id = self
            .vehiculos
            .buscar_por_placa(vehiculo_placa)?
            .map(|vehiculo| vehiculo.id);
        let encargado_id = match &solicitud.encargado_codigo_empleado {
            Some(codigo) => self
                .encargados
                .buscar_por_codigo_empleado(codigo)?
                .map(|encargado| encargado.id),
            None => None,
        };

        let viaje_id = self.resolver_viaje(
            solicitud,
            vehiculo_placa,
            encargado_nombre,
            vehiculo_id,
            encargado_id,
        )?;

        let salida_id = self.salidas.crear(&NuevaSalidaRuta {
            viaje_id,
            vehiculo_id,
            vehiculo_placa: vehiculo_placa.to_string(),
            vehiculo_numero_unidad: solicitud.vehiculo_numero_unidad.clone(),
            encargado_id,
            encargado_nombre: encargado_nombre.to_string(),
            fecha_hora_salida: solicitud.fecha_hora_salida,
            usuario_salida_id: solicitud.usuario_salida_id,
        })?;

        let hoy = fecha_costa_rica(solicitud.fecha_hora_salida);
        for documento_solicitado in &solicitud.documentos {
            self.vincular_documento(salida_id, hoy, solicitud, documento_solicitado)?;
        }

        Ok(ResultadoRegistroSalidaRuta {
            salida_id,
            viaje_id,
        })
    }

    /// `Some(id)` (guardia declaró "sí, misma ruta") valida que ese
    /// viaje siga abierto y que placa/encargado coincidan exactamente;
    /// `None` abre un viaje nuevo (primera salida del día, "otra ruta",
    /// o tercero).
    fn resolver_viaje(
        &self,
        solicitud: &SolicitudSalidaRuta,
        vehiculo_placa: &str,
        encargado_nombre: &str,
        vehiculo_id: Option<i64>,
        encargado_id: Option<i64>,
    ) -> Result<i64, RutaServiceError> {
        match solicitud.continuar_viaje_id {
            Some(id) => {
                let viaje = self
                    .viajes
                    .buscar_por_id(id)?
                    .ok_or(RutaServiceError::ViajeNoEncontrado)?;
                if viaje.estado != EstadoViaje::Abierto {
                    return Err(RutaServiceError::ViajeYaCerrado);
                }
                if viaje.vehiculo_placa != vehiculo_placa
                    || viaje.encargado_nombre != encargado_nombre
                {
                    return Err(RutaServiceError::ViajeNoCoincide);
                }
                Ok(id)
            }
            None => Ok(self.viajes.crear(&NuevoViajeRuta {
                vehiculo_id,
                vehiculo_placa: vehiculo_placa.to_string(),
                vehiculo_numero_unidad: solicitud.vehiculo_numero_unidad.clone(),
                encargado_id,
                encargado_nombre: encargado_nombre.to_string(),
                fecha_hora_creacion: solicitud.fecha_hora_salida,
                usuario_creacion_id: solicitud.usuario_salida_id,
            })?),
        }
    }

    /// Resuelve (o crea) el documento y lo vincula al tramo con su
    /// veredicto de fecha ya evaluado -- ver el doc-comment del módulo
    /// sobre por qué el veredicto vive en el vínculo, no en el documento.
    fn vincular_documento(
        &self,
        salida_id: i64,
        hoy: NaiveDate,
        solicitud: &SolicitudSalidaRuta,
        documento_solicitado: &SolicitudDocumentoRuta,
    ) -> Result<(), RutaServiceError> {
        let numero_documento = documento_solicitado.numero_documento.trim();
        if numero_documento.is_empty() {
            return Err(RutaServiceError::NumeroDocumentoVacio);
        }

        // Bloqueante a propósito, *cuando viene* -- ver el doc-comment
        // del módulo. `None` (tercero sin ruta de catálogo) no valida
        // nada.
        let ruta_id = match documento_solicitado.numero_ruta {
            Some(numero_ruta) => {
                let ruta = self
                    .rutas
                    .buscar_por_numero(numero_ruta)?
                    .ok_or(RutaServiceError::RutaNoEncontrada)?;
                if !ruta.activo {
                    return Err(RutaServiceError::RutaInactiva);
                }
                Some(ruta.id)
            }
            None => None,
        };

        // Reuso de un documento ya existente -- el caso real de la
        // recarga: el mismo documento se reparte en 2+ tramos porque la
        // carga no cupo en un solo viaje.
        let documento_id = match self.documentos.buscar_por_numero(numero_documento)? {
            Some(documento) => documento.id,
            None => self.documentos.crear(
                &NuevoDocumentoRuta {
                    numero_documento: numero_documento.to_string(),
                    ruta_id,
                    sub_numero: documento_solicitado.sub_numero,
                    fecha_documento: documento_solicitado.fecha_documento,
                },
                solicitud.fecha_hora_salida,
            )?,
        };

        let resultado = verificar_fecha_documento(
            documento_solicitado.fecha_documento,
            hoy,
            solicitud.tiene_correo_autorizacion,
        )
        .ok_or(RutaServiceError::DocumentoRequiereAutorizacion)?;

        Ok(self.vinculos.vincular(salida_id, documento_id, resultado)?)
    }

    /// `decision`: la respuesta obligatoria del guardia a "¿vuelve a
    /// salir?" en el mismo acto de confirmar este retorno -- ver
    /// `crate::domain::viaje_ruta::DecisionRetornoViaje`.
    pub fn registrar_retorno(
        &self,
        salida_id: i64,
        decision: DecisionRetornoViaje,
        fecha_hora_retorno: DateTime<Utc>,
        usuario_retorno_id: i64,
    ) -> Result<(), RutaServiceError> {
        let salida = self
            .salidas
            .buscar_por_id(salida_id)?
            .ok_or(RutaServiceError::SalidaNoActiva)?;

        if salida.retorno.is_some() {
            return Err(RutaServiceError::SalidaNoActiva);
        }

        if !crate::domain::resultado_salida_ruta::salida_es_cronologicamente_valida(
            salida.fecha_hora_salida,
            fecha_hora_retorno,
        ) {
            return Err(RutaServiceError::RetornoAnteriorASalida);
        }

        self.salidas
            .registrar_retorno(salida_id, fecha_hora_retorno, usuario_retorno_id)?;

        if decision.cierra_el_viaje() {
            self.viajes
                .cerrar(salida.viaje_id, fecha_hora_retorno, usuario_retorno_id)?;
        }

        Ok(())
    }

    pub fn buscar_por_id(&self, salida_id: i64) -> Result<Option<SalidaRuta>, RutaServiceError> {
        Ok(self.salidas.buscar_por_id(salida_id)?)
    }

    pub fn listar_activas(&self) -> Result<Vec<SalidaRutaActivaResumen>, RutaServiceError> {
        Ok(self.salidas.listar_activas()?)
    }

    /// Historial completo de un viaje (todos sus tramos, abiertos y ya
    /// retornados) -- para la tarjeta agrupada de "Rutas activas".
    pub fn listar_tramos_de_viaje(
        &self,
        viaje_id: i64,
    ) -> Result<Vec<SalidaRuta>, RutaServiceError> {
        Ok(self.salidas.listar_por_viaje(viaje_id)?)
    }

    /// Para que la UI sepa si una unidad ya anduvo hoy y ofrecer "+
    /// Nuevo tramo" en vez de arrancar el checklist desde cero.
    pub fn buscar_viaje_abierto_por_placa(
        &self,
        placa: &str,
    ) -> Result<Option<ViajeRuta>, RutaServiceError> {
        Ok(self.viajes.buscar_abierto_por_placa(placa)?)
    }
}

/// Catálogo de números de ruta -- mismo molde que `GafeteService` (alta
/// individual/por rango, dar de baja con resguardo si está en uso), pero
/// sin estados de portador (ver el doc-comment de `RutaCatalogoServiceError`).
pub struct RutaCatalogoService<'a, R: RutaRepository + ?Sized> {
    rutas: &'a R,
}

impl<'a, R: RutaRepository + ?Sized> RutaCatalogoService<'a, R> {
    pub fn new(rutas: &'a R) -> Self {
        Self { rutas }
    }

    fn buscar_por_id(
        &self,
        id: i64,
    ) -> Result<crate::models::ruta::Ruta, RutaCatalogoServiceError> {
        self.rutas
            .buscar_por_id(id)?
            .ok_or(RutaCatalogoServiceError::RutaNoEncontrada)
    }

    pub fn crear_uno(&self, numero: i64) -> Result<i64, RutaCatalogoServiceError> {
        if numero <= 0 {
            return Err(RutaCatalogoServiceError::NumeroInvalido);
        }
        self.rutas.crear(numero).map_err(|error| {
            if error.es_constraint_unique() {
                RutaCatalogoServiceError::NumeroDuplicado
            } else {
                RutaCatalogoServiceError::Database(error)
            }
        })
    }

    /// Si un número del rango falla (típicamente duplicado -- pedido
    /// explícito del usuario, 2026-09-15: "hay unas que se saltan o no
    /// aplican", así que el rango no asume que todos los números
    /// intermedios están libres), el rango completo aborta sin alta
    /// parcial -- el llamador (`AppCore::crear_rutas_rango`) sólo comitea
    /// la transacción cuando esta función devuelve `Ok`.
    pub fn crear_rango(
        &self,
        desde: i64,
        hasta: i64,
    ) -> Result<Vec<i64>, RutaCatalogoServiceError> {
        if desde <= 0 || hasta < desde {
            return Err(RutaCatalogoServiceError::RangoInvalido);
        }
        (desde..=hasta)
            .map(|numero| self.crear_uno(numero))
            .collect()
    }

    /// `vinculos`: reemplaza al viejo `SalidaRutaRepository` -- el número
    /// de ruta ahora vive en `documentos_ruta`, no en `salidas_ruta`, así
    /// que el chequeo "¿esta ruta está en uso?" necesita el join que
    /// expone `SalidaRutaDocumentoRepository::ruta_tiene_documento_en_tramo_activo`.
    pub fn dar_de_baja<SD: SalidaRutaDocumentoRepository + ?Sized>(
        &self,
        vinculos: &SD,
        id: i64,
    ) -> Result<(), RutaCatalogoServiceError> {
        let mut ruta = self.buscar_por_id(id)?;
        if vinculos.ruta_tiene_documento_en_tramo_activo(id)? {
            return Err(RutaCatalogoServiceError::RutaConSalidaActiva);
        }
        ruta.activo = false;
        Ok(self.rutas.actualizar(&ruta)?)
    }

    pub fn reactivar(&self, id: i64) -> Result<(), RutaCatalogoServiceError> {
        let mut ruta = self.buscar_por_id(id)?;
        ruta.activo = true;
        Ok(self.rutas.actualizar(&ruta)?)
    }

    /// Para la grilla de administración -- trae todas, activas e inactivas
    /// (dadas de baja), así se puede reactivar una. `listar_seleccionables`
    /// es la contraparte para el checklist mobile: una ruta dada de baja
    /// nunca es una opción válida para una salida nueva. Mismo criterio que
    /// `EmpresaProveedorService::listar`/`listar_seleccionables`.
    pub fn listar(&self) -> Result<Vec<crate::models::ruta::Ruta>, RutaCatalogoServiceError> {
        Ok(self.rutas.listar(false)?)
    }

    pub fn listar_seleccionables(
        &self,
    ) -> Result<Vec<crate::models::ruta::Ruta>, RutaCatalogoServiceError> {
        Ok(self.rutas.listar(true)?)
    }

    /// Buscador del checklist mobile (número parcial) -- mismo criterio que
    /// `listar`/`listar_seleccionables`.
    pub fn buscar(
        &self,
        texto: &str,
    ) -> Result<Vec<crate::models::ruta::Ruta>, RutaCatalogoServiceError> {
        Ok(self.rutas.buscar(texto, false)?)
    }

    pub fn buscar_seleccionables(
        &self,
        texto: &str,
    ) -> Result<Vec<crate::models::ruta::Ruta>, RutaCatalogoServiceError> {
        Ok(self.rutas.buscar(texto, true)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::repositories::documento_ruta_repository::SqliteDocumentoRutaRepository;
    use crate::database::repositories::encargado_ruta_repository::SqliteEncargadoRutaRepository;
    use crate::database::repositories::ruta_repository::SqliteRutaRepository;
    use crate::database::repositories::salida_ruta_documento_repository::SqliteSalidaRutaDocumentoRepository;
    use crate::database::repositories::salida_ruta_repository::SqliteSalidaRutaRepository;
    use crate::database::repositories::vehiculo_ruta_repository::SqliteVehiculoRutaRepository;
    use crate::database::repositories::viaje_ruta_repository::SqliteViajeRutaRepository;
    use crate::database::schema::initialize_database;
    use crate::domain::resultado_salida_ruta::ResultadoSalidaRuta;
    use crate::models::encargado_ruta::EncargadoRuta;
    use crate::models::vehiculo_ruta::VehiculoRuta;
    use rusqlite::Connection;

    /// La ruta 79 (activa) ya viene creada -- casi todos los tests de este
    /// módulo registran una salida y no les interesa el catálogo de rutas
    /// en sí, sólo necesitan que la validación bloqueante lo encuentre.
    fn conexion() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO usuarios (id, cedula, nombre, password_hash, rol, activo)
                 VALUES (1, '1001', 'Operador', 'hash', 'OPERADOR', 1)",
                [],
            )
            .unwrap();
        SqliteRutaRepository::new(&connection).crear(79).unwrap();
        connection
    }

    fn fecha(texto: &str) -> NaiveDate {
        texto.parse().unwrap()
    }

    fn instante_costa_rica_hoy(fecha_str: &str, hora: u32) -> DateTime<Utc> {
        // Mediodía en Costa Rica (UTC-6) cae siempre el mismo día civil en
        // ambos lados -- evita que un test cerca de medianoche UTC sea
        // frágil según la hora en que corra.
        use chrono::TimeZone;
        crate::tiempo::ZONA_APLICACION
            .with_ymd_and_hms(
                fecha_str[0..4].parse().unwrap(),
                fecha_str[5..7].parse().unwrap(),
                fecha_str[8..10].parse().unwrap(),
                hora,
                0,
                0,
            )
            .unwrap()
            .with_timezone(&Utc)
    }

    fn documento(numero_documento: &str, fecha_documento: NaiveDate) -> SolicitudDocumentoRuta {
        SolicitudDocumentoRuta {
            numero_documento: numero_documento.to_string(),
            numero_ruta: Some(79),
            sub_numero: 1,
            fecha_documento,
        }
    }

    fn solicitud(
        placa: &str,
        numero_documento: &str,
        fecha_documento: NaiveDate,
    ) -> SolicitudSalidaRuta {
        SolicitudSalidaRuta {
            vehiculo_placa: placa.to_string(),
            vehiculo_numero_unidad: Some("22906".to_string()),
            encargado_nombre: "Carlos Balmaceda".to_string(),
            encargado_codigo_empleado: None,
            documentos: vec![documento(numero_documento, fecha_documento)],
            continuar_viaje_id: None,
            tiene_correo_autorizacion: false,
            usuario_salida_id: 1,
            fecha_hora_salida: instante_costa_rica_hoy("2026-09-15", 12),
        }
    }

    #[allow(clippy::type_complexity)]
    fn servicio(
        connection: &Connection,
    ) -> RutaService<
        '_,
        SqliteSalidaRutaRepository<'_>,
        SqliteVehiculoRutaRepository<'_>,
        SqliteEncargadoRutaRepository<'_>,
        SqliteRutaRepository<'_>,
        SqliteDocumentoRutaRepository<'_>,
        SqliteViajeRutaRepository<'_>,
        SqliteSalidaRutaDocumentoRepository<'_>,
    > {
        // `Box::leak` sólo en tests -- evita atar el lifetime de cada
        // repositorio a esta función auxiliar, mismo truco que ya usan
        // otros módulos de test con muchos repositorios de vida corta.
        let salidas = Box::leak(Box::new(SqliteSalidaRutaRepository::new(connection)));
        let vehiculos = Box::leak(Box::new(SqliteVehiculoRutaRepository::new(connection)));
        let encargados = Box::leak(Box::new(SqliteEncargadoRutaRepository::new(connection)));
        let rutas = Box::leak(Box::new(SqliteRutaRepository::new(connection)));
        let documentos = Box::leak(Box::new(SqliteDocumentoRutaRepository::new(connection)));
        let viajes = Box::leak(Box::new(SqliteViajeRutaRepository::new(connection)));
        let vinculos = Box::leak(Box::new(SqliteSalidaRutaDocumentoRepository::new(
            connection,
        )));
        RutaService::new(
            salidas, vehiculos, encargados, rutas, documentos, viajes, vinculos,
        )
    }

    #[test]
    fn registrar_salida_con_documento_de_hoy_permite_sin_autorizacion() {
        let connection = conexion();
        let servicio = servicio(&connection);

        let resultado = servicio
            .registrar_salida(&solicitud("C12345", "700101452", fecha("2026-09-15")))
            .unwrap();

        let salida = servicio
            .buscar_por_id(resultado.salida_id)
            .unwrap()
            .unwrap();
        assert_eq!(salida.vehiculo_placa, "C12345");
        assert_eq!(salida.viaje_id, resultado.viaje_id);
        assert!(
            salida.vehiculo_id.is_none(),
            "sin catálogo cargado, no hay match"
        );
    }

    #[test]
    fn registrar_salida_con_numero_de_ruta_inexistente_falla() {
        let connection = conexion();
        let servicio = servicio(&connection);

        let mut solicitud = solicitud("C12345", "700101452", fecha("2026-09-15"));
        solicitud.documentos[0].numero_ruta = Some(222);

        let error = servicio.registrar_salida(&solicitud).unwrap_err();

        assert!(matches!(error, RutaServiceError::RutaNoEncontrada));
    }

    #[test]
    fn registrar_salida_con_numero_de_ruta_dado_de_baja_falla() {
        let connection = conexion();
        let rutas = SqliteRutaRepository::new(&connection);
        let mut ruta = rutas.buscar_por_numero(79).unwrap().unwrap();
        ruta.activo = false;
        rutas.actualizar(&ruta).unwrap();
        let servicio = servicio(&connection);

        let error = servicio
            .registrar_salida(&solicitud("C12345", "700101452", fecha("2026-09-15")))
            .unwrap_err();

        assert!(matches!(error, RutaServiceError::RutaInactiva));
    }

    #[test]
    fn registrar_salida_sin_ruta_de_catalogo_es_valida_para_terceros() {
        // Caso tercero confirmado explícitamente por el usuario: a veces
        // ni siquiera hay una ruta de catálogo real detrás de la carga.
        let connection = conexion();
        let servicio = servicio(&connection);

        let mut solicitud = solicitud("BPH485", "999888777", fecha("2026-09-15"));
        solicitud.documentos[0].numero_ruta = None;

        let resultado = servicio.registrar_salida(&solicitud).unwrap();

        assert!(
            servicio
                .buscar_por_id(resultado.salida_id)
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn registrar_salida_hace_match_con_el_catalogo_cuando_existe() {
        let connection = conexion();
        let vehiculos = SqliteVehiculoRutaRepository::new(&connection);
        let vehiculo_id = vehiculos
            .crear(&VehiculoRuta {
                id: 0,
                numero_unidad: Some("22906".to_string()),
                placa: "C12345".to_string(),
                activo: true,
            })
            .unwrap();
        let encargados = SqliteEncargadoRutaRepository::new(&connection);
        let encargado_id = encargados
            .crear(&EncargadoRuta {
                id: 0,
                codigo_empleado: "5040017".to_string(),
                nombre: "Michael Araya Retana".to_string(),
                cedula: None,
                activo: true,
            })
            .unwrap();
        let servicio = servicio(&connection);

        let mut solicitud = solicitud("C12345", "700101452", fecha("2026-09-15"));
        solicitud.encargado_codigo_empleado = Some("5040017".to_string());

        let resultado = servicio.registrar_salida(&solicitud).unwrap();

        let salida = servicio
            .buscar_por_id(resultado.salida_id)
            .unwrap()
            .unwrap();
        assert_eq!(salida.vehiculo_id, Some(vehiculo_id));
        assert_eq!(salida.encargado_id, Some(encargado_id));
    }

    #[test]
    fn registrar_salida_con_documento_de_otro_dia_sin_correo_bloquea() {
        let connection = conexion();
        let servicio = servicio(&connection);

        let error = servicio
            .registrar_salida(&solicitud("C12345", "700101452", fecha("2026-09-14")))
            .unwrap_err();

        assert!(matches!(
            error,
            RutaServiceError::DocumentoRequiereAutorizacion
        ));
    }

    #[test]
    fn registrar_salida_con_documento_de_otro_dia_y_correo_permite_con_autorizacion() {
        let connection = conexion();
        let servicio = servicio(&connection);

        let mut solicitud = solicitud("C12345", "700101452", fecha("2026-09-14"));
        solicitud.tiene_correo_autorizacion = true;

        let resultado = servicio.registrar_salida(&solicitud).unwrap();

        let vinculos = SqliteSalidaRutaDocumentoRepository::new(&connection)
            .listar_por_salida(resultado.salida_id)
            .unwrap();
        assert_eq!(
            vinculos[0].resultado,
            ResultadoSalidaRuta::PermitidoConAutorizacion
        );
    }

    #[test]
    fn registrar_salida_con_vehiculo_ya_en_ruta_falla() {
        let connection = conexion();
        let servicio = servicio(&connection);
        servicio
            .registrar_salida(&solicitud("C12345", "700101452", fecha("2026-09-15")))
            .unwrap();

        let error = servicio
            .registrar_salida(&solicitud("C12345", "700101453", fecha("2026-09-15")))
            .unwrap_err();

        assert!(matches!(error, RutaServiceError::VehiculoYaEnRuta));
    }

    #[test]
    fn registrar_salida_con_placa_vacia_falla() {
        let connection = conexion();
        let servicio = servicio(&connection);

        let error = servicio
            .registrar_salida(&solicitud("  ", "700101452", fecha("2026-09-15")))
            .unwrap_err();

        assert!(matches!(error, RutaServiceError::PlacaVacia));
    }

    #[test]
    fn registrar_salida_sin_documentos_falla() {
        let connection = conexion();
        let servicio = servicio(&connection);
        let mut solicitud = solicitud("C12345", "700101452", fecha("2026-09-15"));
        solicitud.documentos.clear();

        let error = servicio.registrar_salida(&solicitud).unwrap_err();

        assert!(matches!(error, RutaServiceError::SinDocumentos));
    }

    #[test]
    fn registrar_salida_con_varios_documentos_a_la_vez() {
        // Caso real: la ruta 125 sale con 4 documentos a la vez, mismo
        // camión, distintos clientes.
        let connection = conexion();
        let servicio = servicio(&connection);
        let mut solicitud = solicitud("C12345", "700101452", fecha("2026-09-15"));
        solicitud
            .documentos
            .push(documento("700101453", fecha("2026-09-15")));
        solicitud
            .documentos
            .push(documento("700101454", fecha("2026-09-15")));

        let resultado = servicio.registrar_salida(&solicitud).unwrap();

        let vinculos = SqliteSalidaRutaDocumentoRepository::new(&connection)
            .listar_por_salida(resultado.salida_id)
            .unwrap();
        assert_eq!(vinculos.len(), 3);
    }

    #[test]
    fn registrar_retorno_no_vuelve_a_salir_cierra_el_viaje() {
        let connection = conexion();
        let servicio = servicio(&connection);
        let resultado = servicio
            .registrar_salida(&solicitud("C12345", "700101452", fecha("2026-09-15")))
            .unwrap();

        servicio
            .registrar_retorno(
                resultado.salida_id,
                DecisionRetornoViaje::NoVuelveASalir,
                Utc::now(),
                1,
            )
            .unwrap();

        let viaje = SqliteViajeRutaRepository::new(&connection)
            .buscar_por_id(resultado.viaje_id)
            .unwrap()
            .unwrap();
        assert_eq!(viaje.estado, EstadoViaje::Cerrado);
    }

    #[test]
    fn recarga_misma_ruta_continua_el_mismo_viaje_con_el_mismo_documento() {
        // Caso real confirmado: la carga no cupo en el primer viaje, el
        // camión vuelve y sale de nuevo a terminar de despachar el MISMO
        // documento -- confirmado: mismo vehículo, mismo encargado, mismo
        // viaje.
        let connection = conexion();
        let servicio = servicio(&connection);
        let resultado_1 = servicio
            .registrar_salida(&solicitud("C12345", "700101452", fecha("2026-09-15")))
            .unwrap();
        servicio
            .registrar_retorno(
                resultado_1.salida_id,
                DecisionRetornoViaje::MismaRuta,
                instante_costa_rica_hoy("2026-09-15", 14),
                1,
            )
            .unwrap();

        let mut segunda_solicitud = solicitud("C12345", "700101452", fecha("2026-09-15"));
        segunda_solicitud.continuar_viaje_id = Some(resultado_1.viaje_id);

        let resultado_2 = servicio.registrar_salida(&segunda_solicitud).unwrap();

        assert_eq!(resultado_2.viaje_id, resultado_1.viaje_id);
        let vinculos_documento = SqliteSalidaRutaDocumentoRepository::new(&connection)
            .listar_por_documento(
                SqliteDocumentoRutaRepository::new(&connection)
                    .buscar_por_numero("700101452")
                    .unwrap()
                    .unwrap()
                    .id,
            )
            .unwrap();
        assert_eq!(
            vinculos_documento.len(),
            2,
            "el mismo documento queda vinculado a los 2 tramos"
        );
    }

    #[test]
    fn continuar_un_viaje_con_placa_distinta_falla() {
        let connection = conexion();
        let servicio = servicio(&connection);
        let resultado_1 = servicio
            .registrar_salida(&solicitud("C12345", "700101452", fecha("2026-09-15")))
            .unwrap();
        servicio
            .registrar_retorno(
                resultado_1.salida_id,
                DecisionRetornoViaje::MismaRuta,
                instante_costa_rica_hoy("2026-09-15", 14),
                1,
            )
            .unwrap();

        let mut otra_solicitud = solicitud("C99999", "700101453", fecha("2026-09-15"));
        otra_solicitud.continuar_viaje_id = Some(resultado_1.viaje_id);

        let error = servicio.registrar_salida(&otra_solicitud).unwrap_err();

        assert!(matches!(error, RutaServiceError::ViajeNoCoincide));
    }

    #[test]
    fn continuar_un_viaje_ya_cerrado_falla() {
        let connection = conexion();
        let servicio = servicio(&connection);
        let resultado_1 = servicio
            .registrar_salida(&solicitud("C12345", "700101452", fecha("2026-09-15")))
            .unwrap();
        servicio
            .registrar_retorno(
                resultado_1.salida_id,
                DecisionRetornoViaje::NoVuelveASalir,
                instante_costa_rica_hoy("2026-09-15", 14),
                1,
            )
            .unwrap();

        let mut otra_solicitud = solicitud("C12345", "700101453", fecha("2026-09-15"));
        otra_solicitud.continuar_viaje_id = Some(resultado_1.viaje_id);

        let error = servicio.registrar_salida(&otra_solicitud).unwrap_err();

        assert!(matches!(error, RutaServiceError::ViajeYaCerrado));
    }

    #[test]
    fn otra_ruta_cierra_el_viaje_y_abre_uno_nuevo() {
        // "Sí, otra ruta" -- mismo camión/encargado, pero es una
        // asignación distinta: cierra el viaje actual (mismo criterio que
        // "no vuelve a salir", ver `DecisionRetornoViaje::cierra_el_viaje`)
        // y, al no mandar `continuar_viaje_id`, la salida siguiente abre
        // uno nuevo.
        let connection = conexion();
        let servicio = servicio(&connection);
        let resultado_1 = servicio
            .registrar_salida(&solicitud("C12345", "700101452", fecha("2026-09-15")))
            .unwrap();
        servicio
            .registrar_retorno(
                resultado_1.salida_id,
                DecisionRetornoViaje::OtraRutaOTercero,
                instante_costa_rica_hoy("2026-09-15", 14),
                1,
            )
            .unwrap();

        let resultado_2 = servicio
            .registrar_salida(&solicitud("C12345", "700101453", fecha("2026-09-15")))
            .unwrap();

        assert_ne!(resultado_2.viaje_id, resultado_1.viaje_id);

        // El viaje anterior quedó cerrado -- nunca es candidato a
        // continuar (mismo resguardo que `continuar_un_viaje_ya_cerrado_falla`).
        // Primero hay que cerrar el tramo de `resultado_2` -- si no, la
        // validación de "vehículo ya en ruta" (por placa) se dispara antes
        // de siquiera llegar a mirar `continuar_viaje_id`.
        servicio
            .registrar_retorno(
                resultado_2.salida_id,
                DecisionRetornoViaje::NoVuelveASalir,
                instante_costa_rica_hoy("2026-09-15", 16),
                1,
            )
            .unwrap();
        let mut otra_solicitud = solicitud("C12345", "700101454", fecha("2026-09-15"));
        otra_solicitud.continuar_viaje_id = Some(resultado_1.viaje_id);
        let error = servicio.registrar_salida(&otra_solicitud).unwrap_err();
        assert!(matches!(error, RutaServiceError::ViajeYaCerrado));
    }

    #[test]
    fn documento_repartido_en_dos_camiones_distintos_es_valido() {
        // Confirmado explícitamente por el usuario: un documento puede
        // cruzar a otra unidad si hace falta.
        let connection = conexion();
        let servicio = servicio(&connection);
        servicio
            .registrar_salida(&solicitud("C12345", "700101452", fecha("2026-09-15")))
            .unwrap();

        let mut otra_unidad = solicitud("C99999", "700101452", fecha("2026-09-15"));
        otra_unidad.continuar_viaje_id = None;
        let resultado = servicio.registrar_salida(&otra_unidad).unwrap();

        assert!(
            servicio
                .buscar_por_id(resultado.salida_id)
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn registrar_retorno_cierra_la_salida() {
        let connection = conexion();
        let servicio = servicio(&connection);
        let resultado = servicio
            .registrar_salida(&solicitud("C12345", "700101452", fecha("2026-09-15")))
            .unwrap();

        servicio
            .registrar_retorno(
                resultado.salida_id,
                DecisionRetornoViaje::NoVuelveASalir,
                Utc::now(),
                1,
            )
            .unwrap();

        let salida = servicio
            .buscar_por_id(resultado.salida_id)
            .unwrap()
            .unwrap();
        assert!(salida.retorno.is_some());
    }

    #[test]
    fn registrar_retorno_anterior_a_la_salida_falla() {
        let connection = conexion();
        let servicio = servicio(&connection);
        let resultado = servicio
            .registrar_salida(&solicitud("C12345", "700101452", fecha("2026-09-15")))
            .unwrap();

        let error = servicio
            .registrar_retorno(
                resultado.salida_id,
                DecisionRetornoViaje::NoVuelveASalir,
                instante_costa_rica_hoy("2026-09-15", 10) - chrono::Duration::hours(3),
                1,
            )
            .unwrap_err();

        assert!(matches!(error, RutaServiceError::RetornoAnteriorASalida));
    }

    #[test]
    fn registrar_retorno_dos_veces_falla_la_segunda() {
        let connection = conexion();
        let servicio = servicio(&connection);
        let resultado = servicio
            .registrar_salida(&solicitud("C12345", "700101452", fecha("2026-09-15")))
            .unwrap();
        servicio
            .registrar_retorno(
                resultado.salida_id,
                DecisionRetornoViaje::NoVuelveASalir,
                Utc::now(),
                1,
            )
            .unwrap();

        let error = servicio
            .registrar_retorno(
                resultado.salida_id,
                DecisionRetornoViaje::NoVuelveASalir,
                Utc::now(),
                1,
            )
            .unwrap_err();

        assert!(matches!(error, RutaServiceError::SalidaNoActiva));
    }

    #[test]
    fn listar_activas_omite_las_ya_retornadas() {
        let connection = conexion();
        let servicio = servicio(&connection);
        let resultado = servicio
            .registrar_salida(&solicitud("C12345", "700101452", fecha("2026-09-15")))
            .unwrap();
        servicio
            .registrar_salida(&solicitud("C99999", "700101453", fecha("2026-09-15")))
            .unwrap();
        servicio
            .registrar_retorno(
                resultado.salida_id,
                DecisionRetornoViaje::NoVuelveASalir,
                Utc::now(),
                1,
            )
            .unwrap();

        let activas = servicio.listar_activas().unwrap();

        assert_eq!(activas.len(), 1);
        assert_eq!(activas[0].vehiculo_placa, "C99999");
    }

    #[test]
    fn buscar_viaje_abierto_por_placa_encuentra_el_viaje_en_curso() {
        let connection = conexion();
        let servicio = servicio(&connection);
        let resultado = servicio
            .registrar_salida(&solicitud("C12345", "700101452", fecha("2026-09-15")))
            .unwrap();

        let viaje = servicio
            .buscar_viaje_abierto_por_placa("C12345")
            .unwrap()
            .unwrap();
        assert_eq!(viaje.id, resultado.viaje_id);
    }

    #[test]
    fn dar_de_baja_una_ruta_con_documento_en_tramo_activo_se_bloquea() {
        let connection = conexion();
        let servicio = servicio(&connection);
        servicio
            .registrar_salida(&solicitud("C12345", "700101452", fecha("2026-09-15")))
            .unwrap();
        let rutas = SqliteRutaRepository::new(&connection);
        let ruta_id = rutas.buscar_por_numero(79).unwrap().unwrap().id;
        let vinculos = SqliteSalidaRutaDocumentoRepository::new(&connection);
        let catalogo = RutaCatalogoService::new(&rutas);

        let error = catalogo.dar_de_baja(&vinculos, ruta_id).unwrap_err();

        assert!(matches!(
            error,
            RutaCatalogoServiceError::RutaConSalidaActiva
        ));
    }

    #[test]
    fn crear_rango_con_un_numero_ya_existente_falla_en_esa_posicion() {
        let connection = conexion();
        let rutas = SqliteRutaRepository::new(&connection);
        let catalogo = RutaCatalogoService::new(&rutas);

        // La 79 ya existe (fixture) -- el rango 78..=80 choca con ella.
        let error = catalogo.crear_rango(78, 80).unwrap_err();

        assert!(matches!(error, RutaCatalogoServiceError::NumeroDuplicado));
    }
}
