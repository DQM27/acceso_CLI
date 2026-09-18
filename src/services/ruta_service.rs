//! Orquestación del ciclo salida/retorno de rutas
//! (`docs/planes-implementados/plan-control-rutas.md`) -- mismo reparto de
//! responsabilidades que `RegistroIngresoService` (pedido explícito del
//! usuario: "toma como modelo mejor... ingreso"), con dos tratamientos
//! opuestos del catálogo: vehículo/encargado (`vehiculos_ruta`/
//! `encargados_ruta`) son **consultivos, no bloqueantes** -- la placa/el
//! nombre escaneados siempre alcanzan para registrar la salida, el match
//! de catálogo es sólo un enriquecimiento cuando existe. El número de
//! ruta (`rutas`) es lo opuesto: **bloqueante** (pedido explícito del
//! usuario, 2026-09-15: "sin restricción podrías poner la ruta 222 y no
//! existe, sino un número acotado de rutas") -- debe existir y estar
//! activo en el catálogo, mismo criterio que `contratista_id` en
//! `RegistroIngresoService`.

use chrono::{DateTime, NaiveDate, Utc};

use crate::database::repositories::encargado_ruta_repository::EncargadoRutaRepository;
use crate::database::repositories::ruta_repository::RutaRepository;
use crate::database::repositories::salida_ruta_repository::SalidaRutaRepository;
use crate::database::repositories::vehiculo_ruta_repository::VehiculoRutaRepository;
use crate::domain::resultado_salida_ruta::{
    ResultadoSalidaRuta, salida_es_cronologicamente_valida, verificar_fecha_documento,
};
use crate::models::salida_ruta::{NuevaSalidaRuta, SalidaRuta, SalidaRutaActivaResumen};
use crate::tiempo::fecha_costa_rica;

use super::error::{RutaCatalogoServiceError, RutaServiceError};

/// Datos crudos de un escaneo (o entrada manual) de salida -- agrupados en
/// un struct en vez de una lista larga de parámetros posicionales, mismo
/// motivo que `NuevoRegistroIngreso`/`DatosHistoricosEntrada` en el
/// dominio de ingresos. `encargado_codigo_empleado` es `Option` a
/// propósito: el paso "Gafete KOF" del checklist mobile hoy sólo confirma
/// con el **nombre** (frente del carnet) -- ver
/// `PantallaEscanearCarnetKof.kt` -- el código de empleado (reverso)
/// todavía no tiene campo propio en la UI, así que casi siempre viaja
/// `None`. Cuando sí viaja, es la única vía de match con `encargados_ruta`
/// -- nunca se busca por nombre (dos personas pueden compartir nombre; el
/// código de empleado es la clave real del catálogo).
#[derive(Debug, Clone)]
pub struct SolicitudSalidaRuta {
    pub vehiculo_placa: String,
    pub vehiculo_numero_unidad: Option<String>,
    pub encargado_nombre: String,
    pub encargado_codigo_empleado: Option<String>,
    pub numero_ruta: i64,
    pub sub_numero: i64,
    pub numero_documento: String,
    pub fecha_documento: NaiveDate,
    /// Declaración del guardia (checkbox en pantalla) -- ver doc-comment de
    /// `domain::resultado_salida_ruta::verificar_fecha_documento`.
    pub tiene_correo_autorizacion: bool,
    pub usuario_salida_id: i64,
    pub fecha_hora_salida: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResultadoRegistroSalidaRuta {
    pub salida_id: i64,
    pub resultado: ResultadoSalidaRuta,
}

pub struct RutaService<'a, S, V, E, R>
where
    S: SalidaRutaRepository + ?Sized,
    V: VehiculoRutaRepository + ?Sized,
    E: EncargadoRutaRepository + ?Sized,
    R: RutaRepository + ?Sized,
{
    salidas: &'a S,
    vehiculos: &'a V,
    encargados: &'a E,
    rutas: &'a R,
}

impl<'a, S, V, E, R> RutaService<'a, S, V, E, R>
where
    S: SalidaRutaRepository + ?Sized,
    V: VehiculoRutaRepository + ?Sized,
    E: EncargadoRutaRepository + ?Sized,
    R: RutaRepository + ?Sized,
{
    pub fn new(salidas: &'a S, vehiculos: &'a V, encargados: &'a E, rutas: &'a R) -> Self {
        Self {
            salidas,
            vehiculos,
            encargados,
            rutas,
        }
    }

    /// Este servicio es independiente de `SQLite` y no abre transacciones --
    /// mismo contrato que `RegistroIngresoService::registrar_entrada`: el
    /// adaptador productivo debe construir los cuatro repositorios sobre la
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
        let numero_documento = solicitud.numero_documento.trim();
        if numero_documento.is_empty() {
            return Err(RutaServiceError::NumeroDocumentoVacio);
        }

        // Bloqueante a propósito -- ver el doc-comment del módulo. Un
        // número que no existe en el catálogo, o que existe pero está
        // dado de baja, nunca llega a crear la salida.
        let ruta = self
            .rutas
            .buscar_por_numero(solicitud.numero_ruta)?
            .ok_or(RutaServiceError::RutaNoEncontrada)?;
        if !ruta.activo {
            return Err(RutaServiceError::RutaInactiva);
        }

        if self
            .salidas
            .buscar_activa_por_placa(vehiculo_placa)?
            .is_some()
        {
            return Err(RutaServiceError::VehiculoYaEnRuta);
        }

        if self
            .salidas
            .buscar_por_numero_documento(numero_documento)?
            .is_some()
        {
            return Err(RutaServiceError::DocumentoYaRegistrado);
        }

        let hoy = fecha_costa_rica(solicitud.fecha_hora_salida);
        let resultado = verificar_fecha_documento(
            solicitud.fecha_documento,
            hoy,
            solicitud.tiene_correo_autorizacion,
        )
        .ok_or(RutaServiceError::DocumentoRequiereAutorizacion)?;

        // Match de catálogo -- consultivo, nunca bloquea (ver doc-comment
        // del módulo). Vehículo por placa (identificador único y
        // confiable); encargado sólo por código de empleado, nunca por
        // nombre (ver doc-comment de `SolicitudSalidaRuta`).
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

        let salida_id = self.salidas.crear(&NuevaSalidaRuta {
            vehiculo_id,
            vehiculo_placa: vehiculo_placa.to_string(),
            vehiculo_numero_unidad: solicitud.vehiculo_numero_unidad.clone(),
            encargado_id,
            encargado_nombre: encargado_nombre.to_string(),
            ruta_id: ruta.id,
            numero_ruta: ruta.numero,
            sub_numero: solicitud.sub_numero,
            numero_documento: numero_documento.to_string(),
            fecha_documento: solicitud.fecha_documento,
            resultado,
            fecha_hora_salida: solicitud.fecha_hora_salida,
            usuario_salida_id: solicitud.usuario_salida_id,
        })?;

        Ok(ResultadoRegistroSalidaRuta {
            salida_id,
            resultado,
        })
    }

    pub fn registrar_retorno(
        &self,
        salida_id: i64,
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

        if !salida_es_cronologicamente_valida(salida.fecha_hora_salida, fecha_hora_retorno) {
            return Err(RutaServiceError::RetornoAnteriorASalida);
        }

        Ok(self
            .salidas
            .registrar_retorno(salida_id, fecha_hora_retorno, usuario_retorno_id)?)
    }

    pub fn buscar_por_id(&self, salida_id: i64) -> Result<Option<SalidaRuta>, RutaServiceError> {
        Ok(self.salidas.buscar_por_id(salida_id)?)
    }

    pub fn listar_activas(&self) -> Result<Vec<SalidaRutaActivaResumen>, RutaServiceError> {
        Ok(self.salidas.listar_activas()?)
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

    pub fn dar_de_baja<S: SalidaRutaRepository + ?Sized>(
        &self,
        salidas: &S,
        id: i64,
    ) -> Result<(), RutaCatalogoServiceError> {
        let mut ruta = self.buscar_por_id(id)?;
        if salidas.buscar_activa_por_ruta(id)?.is_some() {
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

    pub fn listar(
        &self,
        solo_activos: bool,
    ) -> Result<Vec<crate::models::ruta::Ruta>, RutaCatalogoServiceError> {
        Ok(self.rutas.listar(solo_activos)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::repositories::encargado_ruta_repository::SqliteEncargadoRutaRepository;
    use crate::database::repositories::ruta_repository::SqliteRutaRepository;
    use crate::database::repositories::salida_ruta_repository::SqliteSalidaRutaRepository;
    use crate::database::repositories::vehiculo_ruta_repository::SqliteVehiculoRutaRepository;
    use crate::database::schema::initialize_database;
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
            numero_ruta: 79,
            sub_numero: 1,
            numero_documento: numero_documento.to_string(),
            fecha_documento,
            tiene_correo_autorizacion: false,
            usuario_salida_id: 1,
            fecha_hora_salida: instante_costa_rica_hoy("2026-09-15", 12),
        }
    }

    #[test]
    fn registrar_salida_con_documento_de_hoy_permite_sin_autorizacion() {
        let connection = conexion();
        let salidas = SqliteSalidaRutaRepository::new(&connection);
        let vehiculos = SqliteVehiculoRutaRepository::new(&connection);
        let encargados = SqliteEncargadoRutaRepository::new(&connection);
        let rutas = SqliteRutaRepository::new(&connection);
        let servicio = RutaService::new(&salidas, &vehiculos, &encargados, &rutas);

        let resultado = servicio
            .registrar_salida(&solicitud("C12345", "700101452", fecha("2026-09-15")))
            .unwrap();

        assert_eq!(resultado.resultado, ResultadoSalidaRuta::Permitido);
        let salida = salidas.buscar_por_id(resultado.salida_id).unwrap().unwrap();
        assert_eq!(salida.vehiculo_placa, "C12345");
        assert_eq!(salida.numero_ruta, 79);
        assert!(
            salida.vehiculo_id.is_none(),
            "sin catálogo cargado, no hay match"
        );
    }

    #[test]
    fn registrar_salida_con_numero_de_ruta_inexistente_falla() {
        let connection = conexion();
        let salidas = SqliteSalidaRutaRepository::new(&connection);
        let vehiculos = SqliteVehiculoRutaRepository::new(&connection);
        let encargados = SqliteEncargadoRutaRepository::new(&connection);
        let rutas = SqliteRutaRepository::new(&connection);
        let servicio = RutaService::new(&salidas, &vehiculos, &encargados, &rutas);

        let mut solicitud = solicitud("C12345", "700101452", fecha("2026-09-15"));
        solicitud.numero_ruta = 222;

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
        let salidas = SqliteSalidaRutaRepository::new(&connection);
        let vehiculos = SqliteVehiculoRutaRepository::new(&connection);
        let encargados = SqliteEncargadoRutaRepository::new(&connection);
        let servicio = RutaService::new(&salidas, &vehiculos, &encargados, &rutas);

        let error = servicio
            .registrar_salida(&solicitud("C12345", "700101452", fecha("2026-09-15")))
            .unwrap_err();

        assert!(matches!(error, RutaServiceError::RutaInactiva));
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
        let salidas = SqliteSalidaRutaRepository::new(&connection);
        let rutas = SqliteRutaRepository::new(&connection);
        let servicio = RutaService::new(&salidas, &vehiculos, &encargados, &rutas);

        let mut solicitud = solicitud("C12345", "700101452", fecha("2026-09-15"));
        solicitud.encargado_codigo_empleado = Some("5040017".to_string());

        let resultado = servicio.registrar_salida(&solicitud).unwrap();

        let salida = salidas.buscar_por_id(resultado.salida_id).unwrap().unwrap();
        assert_eq!(salida.vehiculo_id, Some(vehiculo_id));
        assert_eq!(salida.encargado_id, Some(encargado_id));
    }

    #[test]
    fn registrar_salida_con_documento_de_otro_dia_sin_correo_bloquea() {
        let connection = conexion();
        let salidas = SqliteSalidaRutaRepository::new(&connection);
        let vehiculos = SqliteVehiculoRutaRepository::new(&connection);
        let encargados = SqliteEncargadoRutaRepository::new(&connection);
        let rutas = SqliteRutaRepository::new(&connection);
        let servicio = RutaService::new(&salidas, &vehiculos, &encargados, &rutas);

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
        let salidas = SqliteSalidaRutaRepository::new(&connection);
        let vehiculos = SqliteVehiculoRutaRepository::new(&connection);
        let encargados = SqliteEncargadoRutaRepository::new(&connection);
        let rutas = SqliteRutaRepository::new(&connection);
        let servicio = RutaService::new(&salidas, &vehiculos, &encargados, &rutas);

        let mut solicitud = solicitud("C12345", "700101452", fecha("2026-09-14"));
        solicitud.tiene_correo_autorizacion = true;

        let resultado = servicio.registrar_salida(&solicitud).unwrap();

        assert_eq!(
            resultado.resultado,
            ResultadoSalidaRuta::PermitidoConAutorizacion
        );
    }

    #[test]
    fn registrar_salida_con_vehiculo_ya_en_ruta_falla() {
        let connection = conexion();
        let salidas = SqliteSalidaRutaRepository::new(&connection);
        let vehiculos = SqliteVehiculoRutaRepository::new(&connection);
        let encargados = SqliteEncargadoRutaRepository::new(&connection);
        let rutas = SqliteRutaRepository::new(&connection);
        let servicio = RutaService::new(&salidas, &vehiculos, &encargados, &rutas);
        servicio
            .registrar_salida(&solicitud("C12345", "700101452", fecha("2026-09-15")))
            .unwrap();

        let error = servicio
            .registrar_salida(&solicitud("C12345", "700101453", fecha("2026-09-15")))
            .unwrap_err();

        assert!(matches!(error, RutaServiceError::VehiculoYaEnRuta));
    }

    #[test]
    fn registrar_salida_con_documento_repetido_falla() {
        let connection = conexion();
        let salidas = SqliteSalidaRutaRepository::new(&connection);
        let vehiculos = SqliteVehiculoRutaRepository::new(&connection);
        let encargados = SqliteEncargadoRutaRepository::new(&connection);
        let rutas = SqliteRutaRepository::new(&connection);
        let servicio = RutaService::new(&salidas, &vehiculos, &encargados, &rutas);
        servicio
            .registrar_salida(&solicitud("C12345", "700101452", fecha("2026-09-15")))
            .unwrap();

        let error = servicio
            .registrar_salida(&solicitud("C99999", "700101452", fecha("2026-09-15")))
            .unwrap_err();

        assert!(matches!(error, RutaServiceError::DocumentoYaRegistrado));
    }

    #[test]
    fn registrar_salida_con_placa_vacia_falla() {
        let connection = conexion();
        let salidas = SqliteSalidaRutaRepository::new(&connection);
        let vehiculos = SqliteVehiculoRutaRepository::new(&connection);
        let encargados = SqliteEncargadoRutaRepository::new(&connection);
        let rutas = SqliteRutaRepository::new(&connection);
        let servicio = RutaService::new(&salidas, &vehiculos, &encargados, &rutas);

        let error = servicio
            .registrar_salida(&solicitud("  ", "700101452", fecha("2026-09-15")))
            .unwrap_err();

        assert!(matches!(error, RutaServiceError::PlacaVacia));
    }

    #[test]
    fn registrar_retorno_cierra_la_salida() {
        let connection = conexion();
        let salidas = SqliteSalidaRutaRepository::new(&connection);
        let vehiculos = SqliteVehiculoRutaRepository::new(&connection);
        let encargados = SqliteEncargadoRutaRepository::new(&connection);
        let rutas = SqliteRutaRepository::new(&connection);
        let servicio = RutaService::new(&salidas, &vehiculos, &encargados, &rutas);
        let resultado = servicio
            .registrar_salida(&solicitud("C12345", "700101452", fecha("2026-09-15")))
            .unwrap();

        servicio
            .registrar_retorno(resultado.salida_id, Utc::now(), 1)
            .unwrap();

        let salida = salidas.buscar_por_id(resultado.salida_id).unwrap().unwrap();
        assert!(salida.retorno.is_some());
    }

    #[test]
    fn registrar_retorno_anterior_a_la_salida_falla() {
        let connection = conexion();
        let salidas = SqliteSalidaRutaRepository::new(&connection);
        let vehiculos = SqliteVehiculoRutaRepository::new(&connection);
        let encargados = SqliteEncargadoRutaRepository::new(&connection);
        let rutas = SqliteRutaRepository::new(&connection);
        let servicio = RutaService::new(&salidas, &vehiculos, &encargados, &rutas);
        let resultado = servicio
            .registrar_salida(&solicitud("C12345", "700101452", fecha("2026-09-15")))
            .unwrap();
        let salida_registrada = salidas.buscar_por_id(resultado.salida_id).unwrap().unwrap();

        let error = servicio
            .registrar_retorno(
                resultado.salida_id,
                salida_registrada.fecha_hora_salida - chrono::Duration::hours(1),
                1,
            )
            .unwrap_err();

        assert!(matches!(error, RutaServiceError::RetornoAnteriorASalida));
    }

    #[test]
    fn registrar_retorno_dos_veces_falla_la_segunda() {
        let connection = conexion();
        let salidas = SqliteSalidaRutaRepository::new(&connection);
        let vehiculos = SqliteVehiculoRutaRepository::new(&connection);
        let encargados = SqliteEncargadoRutaRepository::new(&connection);
        let rutas = SqliteRutaRepository::new(&connection);
        let servicio = RutaService::new(&salidas, &vehiculos, &encargados, &rutas);
        let resultado = servicio
            .registrar_salida(&solicitud("C12345", "700101452", fecha("2026-09-15")))
            .unwrap();
        servicio
            .registrar_retorno(resultado.salida_id, Utc::now(), 1)
            .unwrap();

        let error = servicio
            .registrar_retorno(resultado.salida_id, Utc::now(), 1)
            .unwrap_err();

        assert!(matches!(error, RutaServiceError::SalidaNoActiva));
    }

    #[test]
    fn listar_activas_delega_al_repositorio() {
        let connection = conexion();
        let salidas = SqliteSalidaRutaRepository::new(&connection);
        let vehiculos = SqliteVehiculoRutaRepository::new(&connection);
        let encargados = SqliteEncargadoRutaRepository::new(&connection);
        let rutas = SqliteRutaRepository::new(&connection);
        let servicio = RutaService::new(&salidas, &vehiculos, &encargados, &rutas);
        servicio
            .registrar_salida(&solicitud("C12345", "700101452", fecha("2026-09-15")))
            .unwrap();

        let activas = servicio.listar_activas().unwrap();

        assert_eq!(activas.len(), 1);
        assert_eq!(activas[0].vehiculo_placa, "C12345");
    }

    #[test]
    fn crear_ruta_con_numero_cero_o_negativo_es_invalido() {
        let connection = conexion();
        let rutas = SqliteRutaRepository::new(&connection);
        let servicio = RutaCatalogoService::new(&rutas);

        assert!(matches!(
            servicio.crear_uno(0),
            Err(RutaCatalogoServiceError::NumeroInvalido)
        ));
        assert!(matches!(
            servicio.crear_uno(-1),
            Err(RutaCatalogoServiceError::NumeroInvalido)
        ));
    }

    #[test]
    fn crear_ruta_con_numero_duplicado_falla() {
        let connection = conexion();
        let rutas = SqliteRutaRepository::new(&connection);
        let servicio = RutaCatalogoService::new(&rutas);

        assert!(matches!(
            servicio.crear_uno(79),
            Err(RutaCatalogoServiceError::NumeroDuplicado)
        ));
    }

    #[test]
    fn crear_rango_invalido_no_crea_nada() {
        let connection = conexion();
        let rutas = SqliteRutaRepository::new(&connection);
        let servicio = RutaCatalogoService::new(&rutas);

        assert!(matches!(
            servicio.crear_rango(5, 3),
            Err(RutaCatalogoServiceError::RangoInvalido)
        ));
    }

    #[test]
    fn crear_rango_con_un_numero_ya_existente_falla_en_esa_posicion() {
        let connection = conexion();
        let rutas = SqliteRutaRepository::new(&connection);
        let servicio = RutaCatalogoService::new(&rutas);

        // La 79 ya la crea `conexion()` -- el rango 78..=80 choca con ella.
        // Sin transacción propia (ver doc-comment de `crear_rango`), el
        // servicio puro no revierte lo ya insertado -- la 78 (antes de la
        // duplicada) SÍ queda creada; la atomicidad real de "todo o nada"
        // la da `AppCore::crear_rutas_rango`, que envuelve esta llamada en
        // una transacción y nunca comitea si devuelve `Err` (ver
        // `application::rutas::tests::crear_rutas_rango_revierte_todo_si_una_falla`).
        let error = servicio.crear_rango(78, 80).unwrap_err();

        assert!(matches!(error, RutaCatalogoServiceError::NumeroDuplicado));
        assert!(rutas.buscar_por_numero(78).unwrap().is_some());
        assert!(rutas.buscar_por_numero(80).unwrap().is_none());
    }

    #[test]
    fn crear_rango_valido_crea_todos_los_numeros() {
        let connection = conexion();
        let rutas = SqliteRutaRepository::new(&connection);
        let servicio = RutaCatalogoService::new(&rutas);

        let ids = servicio.crear_rango(100, 102).unwrap();

        assert_eq!(ids.len(), 3);
        assert!(rutas.buscar_por_numero(100).unwrap().is_some());
        assert!(rutas.buscar_por_numero(101).unwrap().is_some());
        assert!(rutas.buscar_por_numero(102).unwrap().is_some());
    }

    #[test]
    fn dar_de_baja_una_ruta_con_salida_activa_se_bloquea() {
        let connection = conexion();
        let rutas = SqliteRutaRepository::new(&connection);
        let salidas = SqliteSalidaRutaRepository::new(&connection);
        let vehiculos = SqliteVehiculoRutaRepository::new(&connection);
        let encargados = SqliteEncargadoRutaRepository::new(&connection);
        let ruta_id = rutas.buscar_por_numero(79).unwrap().unwrap().id;
        RutaService::new(&salidas, &vehiculos, &encargados, &rutas)
            .registrar_salida(&solicitud("C12345", "700101452", fecha("2026-09-15")))
            .unwrap();

        let error = RutaCatalogoService::new(&rutas)
            .dar_de_baja(&salidas, ruta_id)
            .unwrap_err();

        assert!(matches!(
            error,
            RutaCatalogoServiceError::RutaConSalidaActiva
        ));
    }

    #[test]
    fn dar_de_baja_y_reactivar_una_ruta_redondea_el_viaje() {
        let connection = conexion();
        let rutas = SqliteRutaRepository::new(&connection);
        let salidas = SqliteSalidaRutaRepository::new(&connection);
        let ruta_id = rutas.buscar_por_numero(79).unwrap().unwrap().id;
        let servicio = RutaCatalogoService::new(&rutas);

        servicio.dar_de_baja(&salidas, ruta_id).unwrap();
        assert!(!rutas.buscar_por_id(ruta_id).unwrap().unwrap().activo);

        servicio.reactivar(ruta_id).unwrap();
        assert!(rutas.buscar_por_id(ruta_id).unwrap().unwrap().activo);
    }

    #[test]
    fn dar_de_baja_una_ruta_inexistente_falla() {
        let connection = conexion();
        let rutas = SqliteRutaRepository::new(&connection);
        let salidas = SqliteSalidaRutaRepository::new(&connection);
        let servicio = RutaCatalogoService::new(&rutas);

        let error = servicio.dar_de_baja(&salidas, 999).unwrap_err();

        assert!(matches!(error, RutaCatalogoServiceError::RutaNoEncontrada));
    }
}
