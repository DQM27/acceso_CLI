//! Contratistas y Empresas.

use chrono::NaiveDate;
use rusqlite::{Transaction, TransactionBehavior};

use crate::database::error::DatabaseError;
use crate::database::queries::auditoria::{CambioAuditado, FiltroAuditoria, SqliteAuditoria};
use crate::database::queries::contratistas::{
    FiltroContratistas, PaginaContratistas, SqliteContratistasQuery,
};
use crate::database::queries::empresas::{EmpresaResumen, FiltroEmpresas, SqliteEmpresasQuery};
use crate::database::queries::gafetes_incidentes::{
    GafetesIncidentesQuery, IncidenteGafete, SqliteGafetesIncidentes,
};
use crate::database::repositories::contratista_repository::SqliteContratistaRepository;
use crate::database::repositories::empresa_repository::SqliteEmpresaRepository;
use crate::domain::acceso::{aviso_acceso_en_lista, dias_para_vencer_praind, verificar_acceso};
use crate::domain::autorizacion::Operacion;
use crate::domain::contratista::praind_vencido;
use crate::mensajes::mensaje_aviso_acceso_lista;
use crate::models::contratista::Contratista;
use crate::services::autenticacion_service::UsuarioSesion;
use crate::services::contratista_service::{
    ContratistaConsultaService, ContratistaService, DatosActualizacionContratista, DatosContratista,
};
use crate::services::empresa_service::{EmpresaConsultaService, EmpresaService};
use crate::services::error::{ContratistaServiceError, EmpresaServiceError};
use crate::tiempo::fecha_costa_rica;

use super::{AppCore, CargaCompleta, LIMITE_CARGA_COMPLETA_MAXIMO, verificar_actor_activo};

/// Núcleo de [`AppCore::buscar_auditoria`] sobre una `Connection` cualquiera
/// — mismo motivo que `buscar_historial_con_conexion`
/// (`src/application/historial.rs`): un comando Tauri puede abrir su propia
/// conexión en vez de retener el `Mutex<AppCore>` compartido.
pub fn buscar_auditoria_con_conexion(
    connection: &rusqlite::Connection,
    actor: &UsuarioSesion,
    filtro: &FiltroAuditoria,
) -> Result<crate::database::queries::auditoria::PaginaAuditoria, ContratistaServiceError> {
    let actor_actual = verificar_actor_activo(connection, actor)?
        .ok_or(ContratistaServiceError::OperacionNoAutorizada)?;
    if !actor_actual.rol.puede(Operacion::VerAuditoria) {
        return Err(ContratistaServiceError::OperacionNoAutorizada);
    }
    Ok(SqliteAuditoria::new(connection).buscar(filtro)?)
}

/// Núcleo de [`AppCore::buscar_auditoria_completo`] sobre una `Connection`
/// cualquiera — mismo motivo que [`buscar_auditoria_con_conexion`]: evita
/// retener el núcleo compartido durante los ~750ms que puede tardar esta
/// consulta (medido en la auditoría de las tres capas, `docs/pendientes.md`).
pub fn buscar_auditoria_completo_con_conexion(
    connection: &rusqlite::Connection,
    actor: &UsuarioSesion,
) -> Result<CargaCompleta<CambioAuditado>, ContratistaServiceError> {
    let mut consulta = FiltroAuditoria {
        limite: usize::MAX,
        offset: 0,
    };
    let mut todos = Vec::new();
    let mut total;
    loop {
        let pagina = buscar_auditoria_con_conexion(connection, actor, &consulta)?;
        total = pagina.total;
        if pagina.items.is_empty() {
            break;
        }
        todos.extend(pagina.items);
        if todos.len() >= total || todos.len() >= LIMITE_CARGA_COMPLETA_MAXIMO {
            break;
        }
        consulta.offset = todos.len();
    }
    Ok(CargaCompleta {
        truncado: todos.len() < total,
        items: todos,
    })
}

impl AppCore {
    pub fn buscar_contratistas(
        &self,
        filtro: &FiltroContratistas,
    ) -> Result<PaginaContratistas, ContratistaServiceError> {
        let query = SqliteContratistasQuery::new(&self.connection);
        let mut pagina = ContratistaConsultaService::new(&query).buscar_para_tabla(filtro)?;
        let hoy = fecha_costa_rica(self.reloj.ahora_utc());
        for contratista in &mut pagina.items {
            contratista.aviso_acceso = aviso_acceso_en_lista(
                contratista.tiene_acceso,
                contratista.fecha_vencimiento_praind,
                hoy,
            )
            .map(mensaje_aviso_acceso_lista);
            // Las mismas reglas que al dar ingreso, para mostrar el estado
            // completo en la lista (igual que el panel web).
            let modelo = Contratista::reconstruir(
                contratista.id,
                contratista.cedula.clone(),
                contratista.nombre.clone(),
                contratista.empresa_id,
                contratista.tipo_ingreso,
                contratista.fecha_vencimiento_praind,
                contratista.es_personal_ruta,
                contratista.tiene_acceso,
                contratista.empresa_activa,
            );
            contratista.estado_acceso = Some(verificar_acceso(&modelo, hoy).into());
            contratista.dias_para_vencer_praind = dias_para_vencer_praind(&modelo, hoy);
        }
        Ok(pagina)
    }

    /// Auditoría genérica (contratistas, empresas, usuarios — ver
    /// `EntidadAuditada`, `src/database/queries/auditoria.rs`), no sólo de
    /// contratistas. Sigue gateada por `Operacion::VerAuditoria`: da igual
    /// de qué entidad sea el cambio, es la misma información sensible.
    pub fn buscar_auditoria(
        &self,
        actor: &UsuarioSesion,
        filtro: &FiltroAuditoria,
    ) -> Result<crate::database::queries::auditoria::PaginaAuditoria, ContratistaServiceError> {
        buscar_auditoria_con_conexion(&self.connection, actor, filtro)
    }

    /// Todo el conjunto en un solo `Vec`, no sólo una página — mismo
    /// criterio que `buscar_historial_completo`
    /// (`src/application/historial.rs`) para una interfaz que virtualiza del
    /// lado del cliente (AG Grid) en vez de paginar por su cuenta. A
    /// diferencia de historial, `FiltroAuditoria` no tiene un `corte_id` —
    /// `auditoria_cambios` también es append-only, así que un cambio nuevo
    /// insertado justo mientras se pagina podría, en teoría, correr una fila
    /// entre páginas; caso raro (auditoría no se llena tan rápido como los
    /// ingresos) y no hay mecanismo de corte que reutilizar sin agregarlo
    /// primero a la consulta de abajo. Se corta en
    /// [`LIMITE_CARGA_COMPLETA_MAXIMO`] — a diferencia de Historial, esta
    /// pantalla no tiene selector de rango de fechas, así que es la única
    /// barrera real contra un total que crezca sin límite.
    pub fn buscar_auditoria_completo(
        &self,
        actor: &UsuarioSesion,
    ) -> Result<CargaCompleta<CambioAuditado>, ContratistaServiceError> {
        buscar_auditoria_completo_con_conexion(&self.connection, actor)
    }

    /// Incidentes de gafetes (marcar perdido/resolver, `gafetes_incidentes`)
    /// para la pantalla general de Auditoría — mismo gate que
    /// `buscar_auditoria`, aunque el dato viene de una tabla aparte
    /// (`gafetes_incidentes`, no `auditoria_cambios`): es la misma
    /// información sensible sin importar de qué tabla salga. A diferencia
    /// del historial por gafete puntual (`AppCore::historial_gafete`, sin
    /// restricción), acá sí aplica `Operacion::VerAuditoria`.
    pub fn buscar_auditoria_gafetes(
        &self,
        actor: &UsuarioSesion,
    ) -> Result<Vec<IncidenteGafete>, ContratistaServiceError> {
        let actor_actual = verificar_actor_activo(&self.connection, actor)?
            .ok_or(ContratistaServiceError::OperacionNoAutorizada)?;
        if !actor_actual.rol.puede(Operacion::VerAuditoria) {
            return Err(ContratistaServiceError::OperacionNoAutorizada);
        }
        Ok(SqliteGafetesIncidentes::new(&self.connection).historial_completo()?)
    }

    pub fn crear_contratista(
        &self,
        actor: &UsuarioSesion,
        datos: DatosContratista,
    ) -> Result<i64, ContratistaServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        verificar_actor_activo(&transaction, actor)
            .map_err(ContratistaServiceError::Database)?
            .ok_or(ContratistaServiceError::OperacionNoAutorizada)?;
        ContratistaService::con_hoy(
            &SqliteContratistaRepository::new(&transaction),
            &SqliteEmpresaRepository::new(&transaction),
            fecha_costa_rica(self.reloj.ahora_utc()),
        )
        .crear(datos)
        .and_then(|id| {
            transaction
                .commit()
                .map_err(DatabaseError::from)
                .map_err(ContratistaServiceError::Database)?;
            Ok(id)
        })
    }

    /// `domain::contratista::praind_vencido` contra el "hoy" del reloj del
    /// núcleo (corregido con la hora del servidor en escritorio/móvil):
    /// lo usa el formulario para avisar antes de guardar, con la misma
    /// regla y el mismo reloj que [`Self::crear_contratista`].
    pub fn praind_vencido(&self, fecha_vencimiento: NaiveDate) -> bool {
        praind_vencido(
            Some(fecha_vencimiento),
            fecha_costa_rica(self.reloj.ahora_utc()),
        )
    }

    pub fn actualizar_contratista(
        &self,
        actor: &UsuarioSesion,
        id: i64,
        datos: DatosActualizacionContratista,
    ) -> Result<(), ContratistaServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        let actor_actual = verificar_actor_activo(&transaction, actor)
            .map_err(ContratistaServiceError::Database)?
            .ok_or(ContratistaServiceError::OperacionNoAutorizada)?;
        let contratistas = SqliteContratistaRepository::new(&transaction);
        let empresas = SqliteEmpresaRepository::new(&transaction);
        let servicio = ContratistaService::con_hoy(
            &contratistas,
            &empresas,
            fecha_costa_rica(self.reloj.ahora_utc()),
        );
        let actual = servicio.buscar_por_id(id)?;
        let cedula_nueva = crate::domain::cedula::Cedula::normalizar(&datos.cedula).map_or_else(
            |_| datos.cedula.trim().to_string(),
            crate::domain::cedula::Cedula::into_string,
        );
        if actual.cedula != cedula_nueva
            && !actor_actual.rol.puede(Operacion::EditarCedulaContratista)
        {
            return Err(ContratistaServiceError::OperacionNoAutorizada);
        }
        // Quien está adentro conserva su cédula hasta salir (ver
        // `CedulaConIngresoActivo`).
        if actual.cedula != cedula_nueva
            && crate::database::queries::persona_adentro::contratista_adentro(
                &transaction,
                &actual.cedula,
            )?
        {
            return Err(ContratistaServiceError::CedulaConIngresoActivo);
        }
        if actual.tiene_acceso != datos.tiene_acceso
            && !actor_actual
                .rol
                .puede(Operacion::ActivarDesactivarContratista)
        {
            return Err(ContratistaServiceError::OperacionNoAutorizada);
        }
        servicio.actualizar_auditado(
            id,
            datos,
            actor_actual.id,
            &actor_actual.nombre,
            self.reloj.ahora_utc(),
            &SqliteAuditoria::new(&transaction),
        )?;
        transaction
            .commit()
            .map_err(DatabaseError::from)
            .map_err(ContratistaServiceError::Database)
    }

    /// Para la grilla de administración -- trae activas e inactivas.
    /// `listar_empresas_seleccionables` es la contraparte para el selector
    /// de empresa al crear/editar un contratista, donde una empresa inactiva
    /// nunca es una opción válida -- la decisión de cuál pedir vive en
    /// `EmpresaService`, no acá ni en quien llama.
    pub fn listar_empresas(
        &self,
    ) -> Result<Vec<crate::models::empresa::Empresa>, EmpresaServiceError> {
        EmpresaService::new(&SqliteEmpresaRepository::new(&self.connection)).listar()
    }

    pub fn listar_empresas_seleccionables(
        &self,
    ) -> Result<Vec<crate::models::empresa::Empresa>, EmpresaServiceError> {
        EmpresaService::new(&SqliteEmpresaRepository::new(&self.connection)).listar_seleccionables()
    }

    pub fn buscar_empresas(
        &self,
        filtro: &FiltroEmpresas,
    ) -> Result<Vec<EmpresaResumen>, EmpresaServiceError> {
        EmpresaConsultaService::new(&SqliteEmpresasQuery::new(&self.connection))
            .buscar_para_tabla(filtro)
    }

    pub fn crear_empresa(
        &self,
        actor: &UsuarioSesion,
        nombre: &str,
    ) -> Result<i64, EmpresaServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        verificar_actor_activo(&transaction, actor)
            .map_err(EmpresaServiceError::Database)?
            .ok_or(EmpresaServiceError::OperacionNoAutorizada)?;
        let id = EmpresaService::new(&SqliteEmpresaRepository::new(&transaction)).crear(nombre)?;
        transaction
            .commit()
            .map_err(DatabaseError::from)
            .map_err(EmpresaServiceError::Database)?;
        Ok(id)
    }

    pub fn actualizar_empresa(
        &self,
        actor: &UsuarioSesion,
        id: i64,
        nombre: &str,
    ) -> Result<(), EmpresaServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        let actor_actual = verificar_actor_activo(&transaction, actor)
            .map_err(EmpresaServiceError::Database)?
            .ok_or(EmpresaServiceError::OperacionNoAutorizada)?;
        EmpresaService::new(&SqliteEmpresaRepository::new(&transaction)).actualizar_auditado(
            id,
            nombre,
            actor_actual.id,
            &actor_actual.nombre,
            self.reloj.ahora_utc(),
            &SqliteAuditoria::new(&transaction),
        )?;
        transaction
            .commit()
            .map_err(DatabaseError::from)
            .map_err(EmpresaServiceError::Database)
    }

    pub fn activar_empresa(
        &self,
        actor: &UsuarioSesion,
        id: i64,
    ) -> Result<(), EmpresaServiceError> {
        self.establecer_empresa_activa(actor, id, true)
    }

    pub fn desactivar_empresa(
        &self,
        actor: &UsuarioSesion,
        id: i64,
    ) -> Result<(), EmpresaServiceError> {
        self.establecer_empresa_activa(actor, id, false)
    }

    fn establecer_empresa_activa(
        &self,
        actor: &UsuarioSesion,
        id: i64,
        activa: bool,
    ) -> Result<(), EmpresaServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        let actor_actual = verificar_actor_activo(&transaction, actor)
            .map_err(EmpresaServiceError::Database)?
            .ok_or(EmpresaServiceError::OperacionNoAutorizada)?;
        if !actor_actual.rol.puede(Operacion::ActivarDesactivarEmpresa) {
            return Err(EmpresaServiceError::OperacionNoAutorizada);
        }
        let repositorio = SqliteEmpresaRepository::new(&transaction);
        let servicio = EmpresaService::new(&repositorio);
        servicio.establecer_activo_auditado(
            id,
            activa,
            actor_actual.id,
            &actor_actual.nombre,
            self.reloj.ahora_utc(),
            &SqliteAuditoria::new(&transaction),
        )?;
        transaction
            .commit()
            .map_err(DatabaseError::from)
            .map_err(EmpresaServiceError::Database)
    }
}
