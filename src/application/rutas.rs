//! Control de rutas (`docs/planes-implementados/plan-control-rutas.md`) --
//! fachada sobre `RutaService` para la parte operativa (salida/retorno),
//! mismo armazón que `citas.rs` (transacción `Immediate`, reloj validado,
//! operador activo confirmado dentro de la misma transacción). Los
//! catálogos (`vehiculos_ruta`/`encargados_ruta`) siguen el molde más
//! simple de `catalogos.rs` (empresas): sólo actor activo, sin reloj --
//! son datos de referencia, no un ciclo con apertura/cierre.

use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::database::error::DatabaseError;
use crate::database::repositories::documento_ruta_repository::SqliteDocumentoRutaRepository;
use crate::database::repositories::encargado_ruta_repository::{
    EncargadoRutaRepository, SqliteEncargadoRutaRepository,
};
use crate::database::repositories::ruta_repository::SqliteRutaRepository;
use crate::database::repositories::salida_ruta_documento_repository::SqliteSalidaRutaDocumentoRepository;
use crate::database::repositories::salida_ruta_repository::{
    SqliteSalidaRutaRepository, ultimo_instante_salida_ruta,
};
use crate::database::repositories::vehiculo_ruta_repository::{
    SqliteVehiculoRutaRepository, VehiculoRutaRepository,
};
use crate::database::repositories::viaje_ruta_repository::SqliteViajeRutaRepository;
use crate::domain::viaje_ruta::DecisionRetornoViaje;
use crate::models::encargado_ruta::EncargadoRuta;
use crate::models::ruta::Ruta;
use crate::models::salida_ruta::{SalidaRuta, SalidaRutaActivaResumen};
use crate::models::vehiculo_ruta::VehiculoRuta;
use crate::models::viaje_ruta::ViajeRuta;
use crate::services::autenticacion_service::UsuarioSesion;
use crate::services::encargado_ruta_service::EncargadoRutaService;
use crate::services::error::{
    EncargadoRutaServiceError, RutaCatalogoServiceError, RutaServiceError, VehiculoRutaServiceError,
};
use crate::services::ruta_service::{
    ResultadoRegistroSalidaRuta, RutaCatalogoService, RutaService, SolicitudSalidaRuta,
};
use crate::services::vehiculo_ruta_service::VehiculoRutaService;

use super::{AppCore, verificar_actor_activo};

impl AppCore {
    // ---- Catálogo: vehículos ----

    /// Sin `actor`, mismo criterio que `AppCore::listar_empresas`: es una
    /// lectura, no una operación que autorizar. Para la grilla de
    /// administración -- trae activos e inactivos.
    /// `listar_vehiculos_ruta_seleccionables` es la contraparte para el
    /// selector de salida de ruta, donde un vehículo desactivado nunca es
    /// una opción válida -- la decisión vive en `VehiculoRutaService`.
    pub fn listar_vehiculos_ruta(&self) -> Result<Vec<VehiculoRuta>, DatabaseError> {
        let repositorio = SqliteVehiculoRutaRepository::new(&self.connection);
        VehiculoRutaService::new(&repositorio).listar()
    }

    pub fn listar_vehiculos_ruta_seleccionables(&self) -> Result<Vec<VehiculoRuta>, DatabaseError> {
        let repositorio = SqliteVehiculoRutaRepository::new(&self.connection);
        VehiculoRutaService::new(&repositorio).listar_seleccionables()
    }

    /// Sin `actor`, mismo criterio que `listar_vehiculos_ruta` -- lectura,
    /// no autoriza nada. Buscador de la grilla de administración (placa o
    /// número de unidad). `buscar_vehiculos_ruta_seleccionables` es la
    /// contraparte para el selector del checklist mobile, ver el
    /// doc-comment del trait.
    pub fn buscar_vehiculos_ruta(&self, texto: &str) -> Result<Vec<VehiculoRuta>, DatabaseError> {
        let repositorio = SqliteVehiculoRutaRepository::new(&self.connection);
        VehiculoRutaService::new(&repositorio).buscar(texto)
    }

    pub fn buscar_vehiculos_ruta_seleccionables(
        &self,
        texto: &str,
    ) -> Result<Vec<VehiculoRuta>, DatabaseError> {
        let repositorio = SqliteVehiculoRutaRepository::new(&self.connection);
        VehiculoRutaService::new(&repositorio).buscar_seleccionables(texto)
    }

    pub fn crear_vehiculo_ruta(
        &self,
        actor: &UsuarioSesion,
        vehiculo: &VehiculoRuta,
    ) -> Result<i64, VehiculoRutaServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        verificar_actor_activo(&transaction, actor)
            .map_err(VehiculoRutaServiceError::Database)?
            .ok_or(VehiculoRutaServiceError::OperacionNoAutorizada)?;
        let id = SqliteVehiculoRutaRepository::new(&transaction).crear(vehiculo)?;
        transaction
            .commit()
            .map_err(DatabaseError::from)
            .map_err(VehiculoRutaServiceError::Database)?;
        Ok(id)
    }

    pub fn actualizar_vehiculo_ruta(
        &self,
        actor: &UsuarioSesion,
        vehiculo: &VehiculoRuta,
    ) -> Result<(), VehiculoRutaServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        verificar_actor_activo(&transaction, actor)
            .map_err(VehiculoRutaServiceError::Database)?
            .ok_or(VehiculoRutaServiceError::OperacionNoAutorizada)?;
        SqliteVehiculoRutaRepository::new(&transaction).actualizar(vehiculo)?;
        transaction
            .commit()
            .map_err(DatabaseError::from)
            .map_err(VehiculoRutaServiceError::Database)
    }

    // ---- Catálogo: encargados (personal KOF) ----

    /// Para la grilla de administración -- trae activos e inactivos.
    /// `listar_encargados_ruta_seleccionables` es la contraparte para un
    /// selector de wizard -- la decisión vive en `EncargadoRutaService`.
    pub fn listar_encargados_ruta(&self) -> Result<Vec<EncargadoRuta>, DatabaseError> {
        let repositorio = SqliteEncargadoRutaRepository::new(&self.connection);
        EncargadoRutaService::new(&repositorio).listar()
    }

    pub fn listar_encargados_ruta_seleccionables(
        &self,
    ) -> Result<Vec<EncargadoRuta>, DatabaseError> {
        let repositorio = SqliteEncargadoRutaRepository::new(&self.connection);
        EncargadoRutaService::new(&repositorio).listar_seleccionables()
    }

    /// Sin `actor`, mismo criterio que `listar_encargados_ruta` -- lectura,
    /// no autoriza nada. Buscador de la grilla de administración (nombre o
    /// código de empleado). `buscar_encargados_ruta_seleccionables` es la
    /// contraparte para el selector del checklist mobile/gafete
    /// provisional, ver el doc-comment del trait.
    pub fn buscar_encargados_ruta(&self, texto: &str) -> Result<Vec<EncargadoRuta>, DatabaseError> {
        let repositorio = SqliteEncargadoRutaRepository::new(&self.connection);
        EncargadoRutaService::new(&repositorio).buscar(texto)
    }

    pub fn buscar_encargados_ruta_seleccionables(
        &self,
        texto: &str,
    ) -> Result<Vec<EncargadoRuta>, DatabaseError> {
        let repositorio = SqliteEncargadoRutaRepository::new(&self.connection);
        EncargadoRutaService::new(&repositorio).buscar_seleccionables(texto)
    }

    pub fn crear_encargado_ruta(
        &self,
        actor: &UsuarioSesion,
        encargado: &EncargadoRuta,
    ) -> Result<i64, EncargadoRutaServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        verificar_actor_activo(&transaction, actor)
            .map_err(EncargadoRutaServiceError::Database)?
            .ok_or(EncargadoRutaServiceError::OperacionNoAutorizada)?;
        let id = SqliteEncargadoRutaRepository::new(&transaction).crear(encargado)?;
        transaction
            .commit()
            .map_err(DatabaseError::from)
            .map_err(EncargadoRutaServiceError::Database)?;
        Ok(id)
    }

    pub fn actualizar_encargado_ruta(
        &self,
        actor: &UsuarioSesion,
        encargado: &EncargadoRuta,
    ) -> Result<(), EncargadoRutaServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        verificar_actor_activo(&transaction, actor)
            .map_err(EncargadoRutaServiceError::Database)?
            .ok_or(EncargadoRutaServiceError::OperacionNoAutorizada)?;
        SqliteEncargadoRutaRepository::new(&transaction).actualizar(encargado)?;
        transaction
            .commit()
            .map_err(DatabaseError::from)
            .map_err(EncargadoRutaServiceError::Database)
    }

    // ---- Catálogo: números de ruta (bloqueante, ver ruta_service.rs) ----

    /// Para la grilla de administración -- trae activas y dadas de baja.
    /// `listar_rutas_seleccionables` es la contraparte para el checklist
    /// mobile -- la decisión vive en `RutaCatalogoService`.
    pub fn listar_rutas(&self) -> Result<Vec<Ruta>, RutaCatalogoServiceError> {
        let repositorio = SqliteRutaRepository::new(&self.connection);
        RutaCatalogoService::new(&repositorio).listar()
    }

    pub fn listar_rutas_seleccionables(&self) -> Result<Vec<Ruta>, RutaCatalogoServiceError> {
        let repositorio = SqliteRutaRepository::new(&self.connection);
        RutaCatalogoService::new(&repositorio).listar_seleccionables()
    }

    /// Sin `actor`, mismo criterio que `listar_rutas` -- lectura. Buscador
    /// de la grilla de administración (número parcial).
    /// `buscar_rutas_seleccionables` es la contraparte para el checklist
    /// mobile.
    pub fn buscar_rutas(&self, texto: &str) -> Result<Vec<Ruta>, RutaCatalogoServiceError> {
        let repositorio = SqliteRutaRepository::new(&self.connection);
        RutaCatalogoService::new(&repositorio).buscar(texto)
    }

    pub fn buscar_rutas_seleccionables(
        &self,
        texto: &str,
    ) -> Result<Vec<Ruta>, RutaCatalogoServiceError> {
        let repositorio = SqliteRutaRepository::new(&self.connection);
        RutaCatalogoService::new(&repositorio).buscar_seleccionables(texto)
    }

    pub fn crear_ruta(
        &self,
        actor: &UsuarioSesion,
        numero: i64,
    ) -> Result<i64, RutaCatalogoServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        verificar_actor_activo(&transaction, actor)
            .map_err(RutaCatalogoServiceError::Database)?
            .ok_or(RutaCatalogoServiceError::OperacionNoAutorizada)?;
        let rutas = SqliteRutaRepository::new(&transaction);
        let id = RutaCatalogoService::new(&rutas).crear_uno(numero)?;
        transaction
            .commit()
            .map_err(DatabaseError::from)
            .map_err(RutaCatalogoServiceError::Database)?;
        Ok(id)
    }

    pub fn crear_rutas_rango(
        &self,
        actor: &UsuarioSesion,
        desde: i64,
        hasta: i64,
    ) -> Result<Vec<i64>, RutaCatalogoServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        verificar_actor_activo(&transaction, actor)
            .map_err(RutaCatalogoServiceError::Database)?
            .ok_or(RutaCatalogoServiceError::OperacionNoAutorizada)?;
        let rutas = SqliteRutaRepository::new(&transaction);
        let ids = RutaCatalogoService::new(&rutas).crear_rango(desde, hasta)?;
        transaction
            .commit()
            .map_err(DatabaseError::from)
            .map_err(RutaCatalogoServiceError::Database)?;
        Ok(ids)
    }

    pub fn dar_de_baja_ruta(
        &self,
        actor: &UsuarioSesion,
        id: i64,
    ) -> Result<(), RutaCatalogoServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        verificar_actor_activo(&transaction, actor)
            .map_err(RutaCatalogoServiceError::Database)?
            .ok_or(RutaCatalogoServiceError::OperacionNoAutorizada)?;
        let rutas = SqliteRutaRepository::new(&transaction);
        let vinculos = SqliteSalidaRutaDocumentoRepository::new(&transaction);
        RutaCatalogoService::new(&rutas).dar_de_baja(&vinculos, id)?;
        transaction
            .commit()
            .map_err(DatabaseError::from)
            .map_err(RutaCatalogoServiceError::Database)
    }

    pub fn reactivar_ruta(
        &self,
        actor: &UsuarioSesion,
        id: i64,
    ) -> Result<(), RutaCatalogoServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        verificar_actor_activo(&transaction, actor)
            .map_err(RutaCatalogoServiceError::Database)?
            .ok_or(RutaCatalogoServiceError::OperacionNoAutorizada)?;
        let rutas = SqliteRutaRepository::new(&transaction);
        RutaCatalogoService::new(&rutas).reactivar(id)?;
        transaction
            .commit()
            .map_err(DatabaseError::from)
            .map_err(RutaCatalogoServiceError::Database)
    }

    // ---- Operación: salida / retorno ----

    /// Mismo armazón que `AppCore::en_transaccion_con_reloj_validado_visita`
    /// (`citas.rs`), duplicado en vez de generalizado a propósito -- mismo
    /// motivo que ese: los dominios devuelven tipos de error distintos, sin
    /// un motivo de negocio real para unificarlos.
    fn en_transaccion_con_reloj_validado_rutas<T>(
        &self,
        actor: &UsuarioSesion,
        operar: impl FnOnce(
            &Transaction<'_>,
            chrono::DateTime<chrono::Utc>,
        ) -> Result<T, RutaServiceError>,
    ) -> Result<T, RutaServiceError> {
        let ahora = self.reloj.ahora_utc();
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        validar_reloj(&transaction, ahora)?;
        verificar_operador_activo(&transaction, actor)?;
        let resultado = operar(&transaction, ahora)?;
        transaction.commit().map_err(DatabaseError::from)?;
        Ok(resultado)
    }

    /// `solicitud.usuario_salida_id`/`solicitud.fecha_hora_salida` llegan
    /// ignorados -- se pisan acá con el actor y el reloj ya validados de
    /// esta transacción, nunca con lo que traiga el llamador (mismo
    /// criterio que `fecha_hora_entrada` en `registrar_entrada_visita`).
    /// Reutiliza `SolicitudSalidaRuta` de `RutaService` en vez de un tipo
    /// paralelo en el núcleo -- el DTO de escritorio
    /// (`desktop/src-tauri/src/dto/rutas.rs`) es la frontera que de verdad
    /// necesita una forma sin esos dos campos, mismo criterio que
    /// `DatosContratistaEntrada`/`DatosContratista`.
    pub fn registrar_salida_ruta(
        &self,
        actor: &UsuarioSesion,
        mut solicitud: SolicitudSalidaRuta,
    ) -> Result<ResultadoRegistroSalidaRuta, RutaServiceError> {
        self.en_transaccion_con_reloj_validado_rutas(actor, |transaction, ahora| {
            solicitud.usuario_salida_id = actor.id;
            solicitud.fecha_hora_salida = ahora;
            let salidas = SqliteSalidaRutaRepository::new(transaction);
            let vehiculos = SqliteVehiculoRutaRepository::new(transaction);
            let encargados = SqliteEncargadoRutaRepository::new(transaction);
            let rutas = SqliteRutaRepository::new(transaction);
            let documentos = SqliteDocumentoRutaRepository::new(transaction);
            let viajes = SqliteViajeRutaRepository::new(transaction);
            let vinculos = SqliteSalidaRutaDocumentoRepository::new(transaction);
            RutaService::new(
                &salidas,
                &vehiculos,
                &encargados,
                &rutas,
                &documentos,
                &viajes,
                &vinculos,
            )
            .registrar_salida(&solicitud)
        })
    }

    pub fn registrar_retorno_ruta(
        &self,
        actor: &UsuarioSesion,
        salida_id: i64,
        decision: DecisionRetornoViaje,
    ) -> Result<(), RutaServiceError> {
        self.en_transaccion_con_reloj_validado_rutas(actor, |transaction, ahora| {
            let salidas = SqliteSalidaRutaRepository::new(transaction);
            let vehiculos = SqliteVehiculoRutaRepository::new(transaction);
            let encargados = SqliteEncargadoRutaRepository::new(transaction);
            let rutas = SqliteRutaRepository::new(transaction);
            let documentos = SqliteDocumentoRutaRepository::new(transaction);
            let viajes = SqliteViajeRutaRepository::new(transaction);
            let vinculos = SqliteSalidaRutaDocumentoRepository::new(transaction);
            RutaService::new(
                &salidas,
                &vehiculos,
                &encargados,
                &rutas,
                &documentos,
                &viajes,
                &vinculos,
            )
            .registrar_retorno(salida_id, decision, ahora, actor.id)
        })
    }

    /// Sin `actor`, mismo criterio que `listar_visitas_activas`: es una
    /// lectura, no una operación que autorizar.
    pub fn listar_rutas_activas(&self) -> Result<Vec<SalidaRutaActivaResumen>, RutaServiceError> {
        let salidas = SqliteSalidaRutaRepository::new(&self.connection);
        let vehiculos = SqliteVehiculoRutaRepository::new(&self.connection);
        let encargados = SqliteEncargadoRutaRepository::new(&self.connection);
        let rutas = SqliteRutaRepository::new(&self.connection);
        let documentos = SqliteDocumentoRutaRepository::new(&self.connection);
        let viajes = SqliteViajeRutaRepository::new(&self.connection);
        let vinculos = SqliteSalidaRutaDocumentoRepository::new(&self.connection);
        RutaService::new(
            &salidas,
            &vehiculos,
            &encargados,
            &rutas,
            &documentos,
            &viajes,
            &vinculos,
        )
        .listar_activas()
    }

    pub fn buscar_salida_ruta(&self, id: i64) -> Result<Option<SalidaRuta>, RutaServiceError> {
        let salidas = SqliteSalidaRutaRepository::new(&self.connection);
        let vehiculos = SqliteVehiculoRutaRepository::new(&self.connection);
        let encargados = SqliteEncargadoRutaRepository::new(&self.connection);
        let rutas = SqliteRutaRepository::new(&self.connection);
        let documentos = SqliteDocumentoRutaRepository::new(&self.connection);
        let viajes = SqliteViajeRutaRepository::new(&self.connection);
        let vinculos = SqliteSalidaRutaDocumentoRepository::new(&self.connection);
        RutaService::new(
            &salidas,
            &vehiculos,
            &encargados,
            &rutas,
            &documentos,
            &viajes,
            &vinculos,
        )
        .buscar_por_id(id)
    }

    /// Sin `actor`, mismo criterio que `listar_rutas_activas` -- lectura.
    /// La UI la consulta para ofrecer "+ Nuevo tramo" en vez del checklist
    /// completo cuando una unidad ya anduvo hoy.
    pub fn buscar_viaje_ruta_abierto_por_placa(
        &self,
        placa: &str,
    ) -> Result<Option<ViajeRuta>, RutaServiceError> {
        let salidas = SqliteSalidaRutaRepository::new(&self.connection);
        let vehiculos = SqliteVehiculoRutaRepository::new(&self.connection);
        let encargados = SqliteEncargadoRutaRepository::new(&self.connection);
        let rutas = SqliteRutaRepository::new(&self.connection);
        let documentos = SqliteDocumentoRutaRepository::new(&self.connection);
        let viajes = SqliteViajeRutaRepository::new(&self.connection);
        let vinculos = SqliteSalidaRutaDocumentoRepository::new(&self.connection);
        RutaService::new(
            &salidas,
            &vehiculos,
            &encargados,
            &rutas,
            &documentos,
            &viajes,
            &vinculos,
        )
        .buscar_viaje_abierto_por_placa(placa)
    }
}

/// Mismo criterio que `citas::validar_reloj`, pero tomando el máximo entre
/// los TRES dominios operativos (ingresos, visitas, rutas) -- un sitio que
/// sólo tuvo actividad de rutas hoy (sin ingresos ni visitas todavía)
/// también queda protegido contra un reloj retrocedido.
fn validar_reloj(
    connection: &Connection,
    ahora: chrono::DateTime<chrono::Utc>,
) -> Result<(), RutaServiceError> {
    let ultimo_ingreso =
        crate::database::queries::ingresos::ultimo_instante_movimiento(connection)?;
    let ultima_visita =
        crate::database::repositories::movimiento_visita_repository::ultimo_instante_movimiento_visita(
            connection,
        )?;
    let ultima_ruta = ultimo_instante_salida_ruta(connection)?;
    let Some(ultimo) = [ultimo_ingreso, ultima_visita, ultima_ruta]
        .into_iter()
        .flatten()
        .max()
    else {
        return Ok(());
    };
    if ahora < ultimo {
        return Err(RutaServiceError::RelojRetrocedido);
    }
    Ok(())
}

fn verificar_operador_activo(
    connection: &Connection,
    actor: &UsuarioSesion,
) -> Result<(), RutaServiceError> {
    if verificar_actor_activo(connection, actor)?.is_some() {
        Ok(())
    } else {
        Err(RutaServiceError::OperadorNoAutorizado)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::repositories::ruta_repository::RutaRepository;
    use crate::database::schema::initialize_database;
    use crate::services::autenticacion_service::UsuarioSesion;
    use crate::tiempo::RelojFijo;
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    fn nucleo_con_usuario() -> (AppCore, UsuarioSesion) {
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO usuarios (id, cedula, nombre, password_hash, rol, activo)
                 VALUES (1, '1001', 'Operador', 'hash', 'OPERADOR', 1)",
                [],
            )
            .unwrap();
        SqliteRutaRepository::new(&connection).crear(79).unwrap();
        let reloj = Arc::new(RelojFijo::new(
            Utc.with_ymd_and_hms(2026, 9, 15, 12, 0, 0).unwrap(),
        ));
        let core = AppCore::con_reloj(connection, reloj);
        let sesion = UsuarioSesion {
            id: 1,
            cedula: "1001".to_string(),
            nombre: "Operador".to_string(),
            rol: crate::models::usuario::RolUsuario::Operador,
        };
        (core, sesion)
    }

    fn solicitud(placa: &str, numero_documento: &str) -> SolicitudSalidaRuta {
        use crate::services::ruta_service::SolicitudDocumentoRuta;
        SolicitudSalidaRuta {
            vehiculo_placa: placa.to_string(),
            vehiculo_numero_unidad: Some("22906".to_string()),
            encargado_nombre: "Carlos Balmaceda".to_string(),
            encargado_codigo_empleado: None,
            documentos: vec![SolicitudDocumentoRuta {
                numero_documento: numero_documento.to_string(),
                numero_ruta: Some(79),
                sub_numero: 1,
                fecha_documento: "2026-09-15".parse().unwrap(),
            }],
            continuar_viaje_id: None,
            tiene_correo_autorizacion: false,
            // Ignorados por `AppCore::registrar_salida_ruta` -- se pisan
            // con el actor/reloj reales de la transacción.
            usuario_salida_id: 0,
            fecha_hora_salida: Utc::now(),
        }
    }

    #[test]
    fn registrar_salida_y_retorno_de_ruta_redondea_el_viaje() {
        let (core, actor) = nucleo_con_usuario();

        let resultado = core
            .registrar_salida_ruta(&actor, solicitud("C12345", "700101452"))
            .unwrap();
        core.registrar_retorno_ruta(
            &actor,
            resultado.salida_id,
            DecisionRetornoViaje::NoVuelveASalir,
        )
        .unwrap();

        let salida = core
            .buscar_salida_ruta(resultado.salida_id)
            .unwrap()
            .unwrap();
        assert!(salida.retorno.is_some());
    }

    #[test]
    fn listar_rutas_activas_omite_las_ya_retornadas() {
        let (core, actor) = nucleo_con_usuario();
        let resultado = core
            .registrar_salida_ruta(&actor, solicitud("C12345", "700101452"))
            .unwrap();
        core.registrar_salida_ruta(&actor, solicitud("C99999", "700101453"))
            .unwrap();
        core.registrar_retorno_ruta(
            &actor,
            resultado.salida_id,
            DecisionRetornoViaje::NoVuelveASalir,
        )
        .unwrap();

        let activas = core.listar_rutas_activas().unwrap();

        assert_eq!(activas.len(), 1);
        assert_eq!(activas[0].vehiculo_placa, "C99999");
    }

    #[test]
    fn crear_y_listar_vehiculo_ruta_redondea_el_viaje() {
        let (core, actor) = nucleo_con_usuario();

        core.crear_vehiculo_ruta(
            &actor,
            &VehiculoRuta {
                id: 0,
                numero_unidad: Some("22906".to_string()),
                placa: "C12345".to_string(),
                activo: true,
            },
        )
        .unwrap();

        let vehiculos = core.listar_vehiculos_ruta().unwrap();
        assert_eq!(vehiculos.len(), 1);
        assert_eq!(vehiculos[0].placa, "C12345");
    }

    #[test]
    fn crear_y_listar_encargado_ruta_redondea_el_viaje() {
        let (core, actor) = nucleo_con_usuario();

        core.crear_encargado_ruta(
            &actor,
            &EncargadoRuta {
                id: 0,
                codigo_empleado: "5040017".to_string(),
                nombre: "Michael Araya Retana".to_string(),
                cedula: None,
                activo: true,
            },
        )
        .unwrap();

        let encargados = core.listar_encargados_ruta().unwrap();
        assert_eq!(encargados.len(), 1);
        assert_eq!(encargados[0].codigo_empleado, "5040017");
    }

    #[test]
    fn buscar_encargados_ruta_encuentra_por_nombre_o_codigo() {
        let (core, actor) = nucleo_con_usuario();
        core.crear_encargado_ruta(
            &actor,
            &EncargadoRuta {
                id: 0,
                codigo_empleado: "5040017".to_string(),
                nombre: "Michael Araya Retana".to_string(),
                cedula: None,
                activo: true,
            },
        )
        .unwrap();

        assert_eq!(core.buscar_encargados_ruta("araya").unwrap().len(), 1);
        assert_eq!(core.buscar_encargados_ruta("5040").unwrap().len(), 1);
        assert!(core.buscar_encargados_ruta("nadie").unwrap().is_empty());
    }

    #[test]
    fn buscar_rutas_encuentra_coincidencia_parcial() {
        let (core, _actor) = nucleo_con_usuario();

        // La 79 ya existe (fixture).
        assert_eq!(core.buscar_rutas("79").unwrap().len(), 1);
        assert!(core.buscar_rutas("222").unwrap().is_empty());
    }

    #[test]
    fn reloj_retrocedido_bloquea_una_nueva_salida_de_ruta() {
        let (core, actor) = nucleo_con_usuario();
        core.registrar_salida_ruta(&actor, solicitud("C12345", "700101452"))
            .unwrap();

        // Mismo motivo que `validar_reloj` no se prueba reabriendo un
        // `AppCore` nuevo en `citas.rs`/`accesos.rs` (no hay ese patrón
        // ahí tampoco): `AppCore` implementa `Drop`, así que no se puede
        // mover su `Connection` a un núcleo nuevo. Se prueba la función
        // directamente contra la misma conexión, con un instante anterior
        // a la salida ya registrada.
        let error = validar_reloj(
            &core.connection,
            Utc.with_ymd_and_hms(2026, 9, 15, 11, 0, 0).unwrap(),
        )
        .unwrap_err();

        assert!(matches!(error, RutaServiceError::RelojRetrocedido));
    }

    #[test]
    fn crear_y_listar_ruta_redondea_el_viaje() {
        let (core, actor) = nucleo_con_usuario();

        core.crear_ruta(&actor, 120).unwrap();

        let rutas = core.listar_rutas().unwrap();
        assert_eq!(rutas.len(), 2, "79 (del fixture) + 120");
        assert!(rutas.iter().any(|r| r.numero == 120));
    }

    #[test]
    fn crear_rutas_rango_revierte_todo_si_una_falla() {
        let (core, actor) = nucleo_con_usuario();

        // La 79 ya existe (fixture) -- el rango 78..=80 choca con ella. A
        // diferencia del servicio puro sin transacción
        // (`ruta_service::tests::crear_rango_con_un_numero_ya_existente_falla_en_esa_posicion`),
        // acá SÍ hay una transacción real (`AppCore::crear_rutas_rango`):
        // como la función nunca llega a `transaction.commit()` cuando
        // devuelve `Err`, el `Drop` de `Transaction` revierte todo --
        // tampoco queda la 78.
        let error = core.crear_rutas_rango(&actor, 78, 80).unwrap_err();

        assert!(matches!(error, RutaCatalogoServiceError::NumeroDuplicado));
        let rutas = core.listar_rutas().unwrap();
        assert_eq!(
            rutas.iter().map(|r| r.numero).collect::<Vec<_>>(),
            vec![79],
            "el rollback deja sólo la ruta del fixture, ni la 78 queda creada"
        );
    }

    #[test]
    fn crear_rutas_rango_sin_conflicto_crea_todas() {
        let (core, actor) = nucleo_con_usuario();

        let ids = core.crear_rutas_rango(&actor, 100, 102).unwrap();

        assert_eq!(ids.len(), 3);
        assert_eq!(core.listar_rutas().unwrap().len(), 4, "79 (fixture) + 3");
    }

    #[test]
    fn dar_de_baja_y_reactivar_ruta_redondean_el_viaje() {
        let (core, actor) = nucleo_con_usuario();
        let ruta_id = core
            .listar_rutas()
            .unwrap()
            .into_iter()
            .find(|r| r.numero == 79)
            .unwrap()
            .id;

        core.dar_de_baja_ruta(&actor, ruta_id).unwrap();
        assert!(!core.listar_rutas().unwrap()[0].activo);

        core.reactivar_ruta(&actor, ruta_id).unwrap();
        assert!(core.listar_rutas().unwrap()[0].activo);
    }

    #[test]
    fn dar_de_baja_una_ruta_con_salida_activa_se_bloquea() {
        let (core, actor) = nucleo_con_usuario();
        core.registrar_salida_ruta(&actor, solicitud("C12345", "700101452"))
            .unwrap();
        let ruta_id = core.listar_rutas().unwrap()[0].id;

        let error = core.dar_de_baja_ruta(&actor, ruta_id).unwrap_err();

        assert!(matches!(
            error,
            RutaCatalogoServiceError::RutaConSalidaActiva
        ));
    }
}
