//! Catálogo de gafetes. **Sin restricción de rol a propósito** — decisión
//! explícita del usuario: cualquier operador con sesión activa gestiona el
//! catálogo (alta, reportar perdido, pago, recuperar, conteo), a diferencia
//! de Empresas/Usuarios. Sólo se exige que el actor siga siendo un usuario
//! activo (`verificar_actor_activo`): cada reporte queda a su nombre.

use rusqlite::{Transaction, TransactionBehavior, params};

use crate::database::error::DatabaseError;
use crate::database::queries::Igualdad;
use crate::database::queries::gafetes::{FiltroGafetes, GafeteResumen, SqliteGafetesQuery};
use crate::database::queries::gafetes_incidentes::{
    GafetesIncidentesQuery, IncidenteGafete, SqliteGafetesIncidentes,
};
use crate::database::repositories::cita_repository::{CitaRepository, SqliteCitaRepository};
use crate::database::repositories::contratista_repository::{
    ContratistaRepository, SqliteContratistaRepository,
};
use crate::database::repositories::encargado_ruta_repository::{
    EncargadoRutaRepository, SqliteEncargadoRutaRepository,
};
use crate::database::repositories::gafete_repository::SqliteGafeteRepository;
use crate::domain::gafete::{
    GafeteParaConteo, LecturaConteo, ResultadoConteo, comprobar_conteo, leer_numero_conteo,
};
use crate::models::gafete::{PortadorGafete, TipoGafete};
use crate::services::autenticacion_service::UsuarioSesion;
use crate::services::error::GafeteServiceError;
use crate::services::gafete_service::{AutorReporte, GafeteConsultaService, GafeteService};

use super::{AppCore, verificar_actor_activo};

/// Movimientos abiertos de cada tipo de gafete: `(numero, nombre)` de quien
/// lo tiene puesto. Las cachés `*_remotos` son lo abierto por el otro
/// dispositivo del sitio; un registro cerrado acá queda en
/// `remotos_cerrados_aca` hasta que la nube lo confirme.
fn consultas_en_uso(tipo: TipoGafete) -> (&'static [&'static str], &'static [&'static str]) {
    match tipo {
        TipoGafete::Contratista => (
            &[
                "SELECT gafete_numero, contratista_nombre FROM registro_ingresos
               WHERE fecha_hora_salida IS NULL AND gafete_numero IS NOT NULL",
            ],
            &[
                "SELECT gafete_numero, contratista_nombre FROM ingresos_remotos
               WHERE gafete_numero IS NOT NULL
                 AND uuid NOT IN (SELECT uuid FROM remotos_cerrados_aca)",
            ],
        ),
        TipoGafete::Visita => (
            &[
                "SELECT mv.gafete_numero, cv.nombre FROM movimientos_visita mv
                 JOIN cita_visitantes cv ON cv.id = mv.cita_visitante_id
                 WHERE mv.fecha_hora_salida IS NULL AND mv.gafete_numero IS NOT NULL",
                "SELECT gafete_numero, nombre FROM registro_ingresos_correo
                 WHERE fecha_hora_salida IS NULL",
            ],
            &[
                "SELECT gafete_numero, nombre FROM movimientos_visita_remotos
                 WHERE gafete_numero IS NOT NULL
                   AND uuid NOT IN (SELECT uuid FROM remotos_cerrados_aca)",
                "SELECT gafete_numero, nombre FROM ingresos_correo_remotos
                 WHERE uuid NOT IN (SELECT uuid FROM remotos_cerrados_aca)",
            ],
        ),
        TipoGafete::Proveedor => (
            &[
                "SELECT gafete_numero, nombre FROM registro_ingresos_proveedor
               WHERE fecha_hora_salida IS NULL",
            ],
            &[
                "SELECT gafete_numero, nombre FROM ingresos_proveedor_remotos
               WHERE uuid NOT IN (SELECT uuid FROM remotos_cerrados_aca)",
            ],
        ),
        TipoGafete::ProvisionalKof => (
            &[
                "SELECT gafete_numero, encargado_nombre FROM prestamos_gafete_provisional
               WHERE fecha_hora_devolucion IS NULL",
            ],
            &[
                "SELECT gafete_numero, encargado_nombre FROM prestamos_gafete_provisional_remotos
               WHERE uuid NOT IN (SELECT uuid FROM remotos_cerrados_aca)",
            ],
        ),
    }
}

impl AppCore {
    pub fn buscar_gafetes(
        &self,
        filtro: &FiltroGafetes,
    ) -> Result<Vec<GafeteResumen>, GafeteServiceError> {
        let query = SqliteGafetesQuery::new(&self.connection);
        GafeteConsultaService::new(&query).buscar(filtro)
    }

    /// Historial de reportes de un gafete puntual (quién lo reportó perdido,
    /// quién registró el pago o lo recuperó, cuándo y qué escribió) — sin
    /// actor, mismo criterio sin restricción que `buscar_gafetes`.
    pub fn historial_gafete(
        &self,
        gafete_id: i64,
    ) -> Result<Vec<IncidenteGafete>, GafeteServiceError> {
        let incidentes = SqliteGafetesIncidentes::new(&self.connection);
        Ok(incidentes.historial(gafete_id)?)
    }

    pub fn crear_gafete(
        &self,
        actor: &UsuarioSesion,
        numero: i64,
        tipo: TipoGafete,
    ) -> Result<i64, GafeteServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        verificar_actor_activo(&transaction, actor)
            .map_err(GafeteServiceError::Database)?
            .ok_or(GafeteServiceError::OperacionNoAutorizada)?;
        let gafetes = SqliteGafeteRepository::new(&transaction);
        let id = GafeteService::new(&gafetes).crear_uno(numero, tipo)?;
        transaction.commit().map_err(DatabaseError::from)?;
        Ok(id)
    }

    /// Alta por rango (desde-hasta), para cargar p. ej. 01-25 de una vez —
    /// si un número del rango falla (típicamente duplicado), el rango
    /// completo aborta: la transacción nunca comitea, sin alta parcial.
    pub fn crear_gafetes_rango(
        &self,
        actor: &UsuarioSesion,
        desde: i64,
        hasta: i64,
        tipo: TipoGafete,
    ) -> Result<Vec<i64>, GafeteServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        verificar_actor_activo(&transaction, actor)
            .map_err(GafeteServiceError::Database)?
            .ok_or(GafeteServiceError::OperacionNoAutorizada)?;
        let gafetes = SqliteGafeteRepository::new(&transaction);
        let ids = GafeteService::new(&gafetes).crear_rango(desde, hasta, tipo)?;
        transaction.commit().map_err(DatabaseError::from)?;
        Ok(ids)
    }

    /// Reporta perdido un gafete disponible. `responsable`: a quién se le
    /// asignó, si se sabe (se valida que exista, según su tipo); sin él, el
    /// `detalle` es obligatorio. Con o sin responsable, el detalle queda a
    /// nombre de quien reporta.
    pub fn reportar_gafete_perdido(
        &self,
        actor: &UsuarioSesion,
        id: i64,
        responsable: Option<PortadorGafete>,
        detalle: Option<&str>,
    ) -> Result<(), GafeteServiceError> {
        // Se decide antes de abrir la transacción: consulta la sesión de nube.
        let incluir_otro_equipo = self.incluir_otro_equipo(actor);
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        let actor_actual = verificar_actor_activo(&transaction, actor)
            .map_err(GafeteServiceError::Database)?
            .ok_or(GafeteServiceError::OperacionNoAutorizada)?;
        if let Some(responsable) = responsable {
            verificar_responsable(&transaction, responsable)?;
        }
        let gafetes = SqliteGafeteRepository::new(&transaction);
        let incidentes = SqliteGafetesIncidentes::new(&transaction);
        GafeteService::new(&gafetes).reportar_perdido(
            &incidentes,
            |gafete| {
                quien_tiene_puesto(
                    &transaction,
                    gafete.tipo,
                    gafete.numero,
                    incluir_otro_equipo,
                )
            },
            id,
            responsable,
            detalle,
            AutorReporte {
                usuario_id: actor_actual.id,
                ahora: self.reloj.ahora_utc(),
            },
        )?;
        transaction.commit().map_err(DatabaseError::from)?;
        Ok(())
    }

    /// El responsable pagó: se le quita la marca; el gafete sigue perdido
    /// hasta que se recupere.
    pub fn registrar_pago_gafete(
        &self,
        actor: &UsuarioSesion,
        id: i64,
        detalle: Option<&str>,
    ) -> Result<(), GafeteServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        let actor_actual = verificar_actor_activo(&transaction, actor)
            .map_err(GafeteServiceError::Database)?
            .ok_or(GafeteServiceError::OperacionNoAutorizada)?;
        let gafetes = SqliteGafeteRepository::new(&transaction);
        let incidentes = SqliteGafetesIncidentes::new(&transaction);
        GafeteService::new(&gafetes).registrar_pago(
            &incidentes,
            id,
            detalle,
            AutorReporte {
                usuario_id: actor_actual.id,
                ahora: self.reloj.ahora_utc(),
            },
        )?;
        transaction.commit().map_err(DatabaseError::from)?;
        Ok(())
    }

    /// Apareció o se reimprimió: vuelve a estar disponible.
    pub fn recuperar_gafete(
        &self,
        actor: &UsuarioSesion,
        id: i64,
        detalle: &str,
    ) -> Result<(), GafeteServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        let actor_actual = verificar_actor_activo(&transaction, actor)
            .map_err(GafeteServiceError::Database)?
            .ok_or(GafeteServiceError::OperacionNoAutorizada)?;
        let gafetes = SqliteGafeteRepository::new(&transaction);
        let incidentes = SqliteGafetesIncidentes::new(&transaction);
        GafeteService::new(&gafetes).recuperar(
            &incidentes,
            id,
            Some(detalle),
            AutorReporte {
                usuario_id: actor_actual.id,
                ahora: self.reloj.ahora_utc(),
            },
        )?;
        transaction.commit().map_err(DatabaseError::from)?;
        Ok(())
    }

    /// Qué es un número escrito en el conteo físico, en el momento de
    /// escribirlo: esperado, no existe, perdido o puesto en alguien.
    pub fn leer_numero_conteo_gafetes(
        &self,
        actor: &UsuarioSesion,
        tipo: TipoGafete,
        numero: i64,
    ) -> Result<LecturaConteo, GafeteServiceError> {
        let catalogo = self.catalogo_para_conteo(actor, tipo)?;
        Ok(leer_numero_conteo(&catalogo, numero))
    }

    /// Botón "Comprobar" del conteo: lo contado contra lo que debería haber.
    pub fn comprobar_conteo_gafetes(
        &self,
        actor: &UsuarioSesion,
        tipo: TipoGafete,
        numeros: &[i64],
    ) -> Result<ResultadoConteo, GafeteServiceError> {
        let catalogo = self.catalogo_para_conteo(actor, tipo)?;
        Ok(comprobar_conteo(&catalogo, numeros))
    }

    /// El catálogo de un tipo con quién tiene puesto cada gafete: lo abierto
    /// en este equipo y, si la sesión puede usar la nube, lo abierto por el
    /// otro dispositivo del sitio (mismo criterio que `tenedores_del_gafete`).
    fn catalogo_para_conteo(
        &self,
        actor: &UsuarioSesion,
        tipo: TipoGafete,
    ) -> Result<Vec<GafeteParaConteo>, GafeteServiceError> {
        verificar_actor_activo(&self.connection, actor)
            .map_err(GafeteServiceError::Database)?
            .ok_or(GafeteServiceError::OperacionNoAutorizada)?;
        let gafetes = self.buscar_gafetes(&FiltroGafetes {
            tipo: Some(Igualdad::Incluye(tipo)),
            ..FiltroGafetes::default()
        })?;
        let en_uso = gafetes_en_uso(&self.connection, tipo, self.incluir_otro_equipo(actor))?;

        Ok(gafetes
            .into_iter()
            .map(|gafete| GafeteParaConteo {
                numero: gafete.numero,
                estado: gafete.estado,
                en_uso_por: en_uso
                    .iter()
                    .find(|(numero, _)| *numero == gafete.numero)
                    .map(|(_, nombre)| nombre.clone()),
            })
            .collect())
    }

    /// Si la sesión puede usar la nube, "en uso" también mira lo abierto por
    /// el otro equipo del sitio (las cachés `*_remotos`), mismo criterio que
    /// `tenedores_del_gafete`.
    #[cfg_attr(not(feature = "nube"), allow(clippy::unused_self))]
    fn incluir_otro_equipo(&self, actor: &UsuarioSesion) -> bool {
        #[cfg(feature = "nube")]
        {
            self.autorizar_uso_nube(actor).is_ok()
        }
        #[cfg(not(feature = "nube"))]
        {
            let _ = actor;
            false
        }
    }
}

/// Quién tiene puesto cada gafete de **un** tipo: `(numero, nombre)`. Es la
/// única respuesta a "¿alguien tiene este gafete?" para el conteo y para
/// reportar perdido. Cada tipo mira sólo sus propios movimientos
/// ([`consultas_en_uso`]): el 5 de contratista y el 5 de proveedor son dos
/// gafetes distintos y nunca se cruzan.
fn gafetes_en_uso(
    connection: &rusqlite::Connection,
    tipo: TipoGafete,
    incluir_otro_equipo: bool,
) -> Result<Vec<(i64, String)>, DatabaseError> {
    let (locales, remotas) = consultas_en_uso(tipo);
    let consultas = locales
        .iter()
        .chain(remotas.iter().filter(|_| incluir_otro_equipo));
    let mut en_uso: Vec<(i64, String)> = Vec::new();
    for sql in consultas {
        let mut statement = connection.prepare(sql)?;
        let filas = statement
            .query_map(params![], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<(i64, String)>, _>>()?;
        en_uso.extend(filas);
    }
    Ok(en_uso)
}

/// Quién tiene puesto el gafete `numero` de este tipo, si alguien.
fn quien_tiene_puesto(
    connection: &rusqlite::Connection,
    tipo: TipoGafete,
    numero: i64,
    incluir_otro_equipo: bool,
) -> Result<Option<String>, DatabaseError> {
    Ok(gafetes_en_uso(connection, tipo, incluir_otro_equipo)?
        .into_iter()
        .find(|(en_uso, _)| *en_uso == numero)
        .map(|(_, nombre)| nombre))
}

/// El responsable de un gafete perdido tiene que existir, según su tipo.
fn verificar_responsable(
    transaction: &Transaction<'_>,
    responsable: PortadorGafete,
) -> Result<(), GafeteServiceError> {
    match responsable {
        PortadorGafete::Contratista(id) => {
            if SqliteContratistaRepository::new(transaction)
                .buscar_por_id(id)?
                .is_none()
            {
                return Err(GafeteServiceError::ContratistaNoEncontrado);
            }
        }
        PortadorGafete::Visita(id) => {
            if SqliteCitaRepository::new(transaction)
                .buscar_visitante_por_id(id)?
                .is_none()
            {
                return Err(GafeteServiceError::VisitaNoEncontrada);
            }
        }
        PortadorGafete::ProvisionalKof(id) => {
            if SqliteEncargadoRutaRepository::new(transaction)
                .buscar_por_id(id)?
                .is_none()
            {
                return Err(GafeteServiceError::EncargadoRutaNoEncontrado);
            }
        }
        // El colaborador de un proveedor no tiene catálogo propio: la
        // pantalla no ofrece buscarlo, y el núcleo no lo acepta.
        PortadorGafete::Proveedor(_) => return Err(GafeteServiceError::EstadoInvalido),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::puertos::gafetes::GafeteRepository;
    use crate::database::schema::initialize_database;
    use crate::models::encargado_ruta::EncargadoRuta;
    use crate::models::gafete::{EstadoGafete, TipoGafete};
    use crate::tiempo::RelojFijo;
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    fn nucleo_con_usuario_encargado_y_gafete() -> (AppCore, UsuarioSesion, i64) {
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO usuarios (id, cedula, nombre, password_hash, rol, activo)
                 VALUES (1, '1001', 'Operador', 'hash', 'OPERADOR', 1)",
                [],
            )
            .unwrap();
        let encargado_id = SqliteEncargadoRutaRepository::new(&connection)
            .crear(&EncargadoRuta {
                id: 0,
                codigo_empleado: "E1".to_string(),
                nombre: "Encargado Test".to_string(),
                cedula: None,
                activo: true,
            })
            .unwrap();
        let gafetes = SqliteGafeteRepository::new(&connection);
        gafetes.crear(9, TipoGafete::ProvisionalKof).unwrap();
        for numero in 1..=4 {
            gafetes.crear(numero, TipoGafete::ProvisionalKof).unwrap();
        }
        let reloj = Arc::new(RelojFijo::new(
            Utc.with_ymd_and_hms(2026, 9, 19, 12, 0, 0).unwrap(),
        ));
        let core = AppCore::con_reloj(connection, reloj);
        let sesion = UsuarioSesion {
            id: 1,
            cedula: "1001".to_string(),
            nombre: "Operador".to_string(),
            rol: crate::models::usuario::RolUsuario::Operador,
        };
        (core, sesion, encargado_id)
    }

    fn id_del_gafete(core: &AppCore, numero: i64) -> i64 {
        core.buscar_gafetes(&FiltroGafetes::default())
            .unwrap()
            .into_iter()
            .find(|g| g.numero == numero)
            .unwrap()
            .id
    }

    /// Cualquier gafete se puede perder, KOF no es la excepción.
    #[test]
    fn reportar_perdido_provisional_kof_redondea_el_viaje() {
        let (core, actor, encargado_id) = nucleo_con_usuario_encargado_y_gafete();
        let gafete_id = id_del_gafete(&core, 9);

        core.reportar_gafete_perdido(
            &actor,
            gafete_id,
            Some(PortadorGafete::ProvisionalKof(encargado_id)),
            None,
        )
        .unwrap();

        let gafete = core
            .buscar_gafetes(&FiltroGafetes::default())
            .unwrap()
            .into_iter()
            .find(|g| g.id == gafete_id)
            .unwrap();
        assert_eq!(gafete.estado, EstadoGafete::Perdido);
    }

    #[test]
    fn reportar_perdido_con_encargado_inexistente_falla() {
        let (core, actor, _) = nucleo_con_usuario_encargado_y_gafete();
        let gafete_id = id_del_gafete(&core, 9);

        let error = core
            .reportar_gafete_perdido(
                &actor,
                gafete_id,
                Some(PortadorGafete::ProvisionalKof(9999)),
                None,
            )
            .unwrap_err();

        assert!(matches!(
            error,
            GafeteServiceError::EncargadoRutaNoEncontrado
        ));
    }

    #[test]
    fn el_conteo_ve_perdidos_y_prestados() {
        let (core, actor, encargado_id) = nucleo_con_usuario_encargado_y_gafete();
        core.reportar_gafete_perdido(&actor, id_del_gafete(&core, 2), None, Some("no está"))
            .unwrap();
        core.connection
            .execute(
                "INSERT INTO prestamos_gafete_provisional (
                    encargado_id, encargado_nombre, encargado_codigo_empleado, gafete_numero,
                    fecha_hora_entrega, usuario_entrega_id, usuario_entrega_nombre, uuid
                 ) VALUES (?1, 'Encargado Test', 'E1', 3, '2026-09-19T11:00:00Z', 1, 'Operador', 'p1')",
                params![encargado_id],
            )
            .unwrap();

        let resultado = core
            .comprobar_conteo_gafetes(&actor, TipoGafete::ProvisionalKof, &[1, 2, 40])
            .unwrap();

        assert_eq!(resultado.presentes, vec![1]);
        assert_eq!(resultado.faltantes, vec![4, 9]);
        assert_eq!(resultado.perdidos_contados, vec![2]);
        assert_eq!(resultado.no_existen, vec![40]);
        assert_eq!(resultado.en_uso.len(), 1);
        assert_eq!(resultado.en_uso[0].numero, 3);
        assert_eq!(resultado.en_uso[0].nombre, "Encargado Test");

        assert_eq!(
            core.leer_numero_conteo_gafetes(&actor, TipoGafete::ProvisionalKof, 3)
                .unwrap(),
            LecturaConteo::EnUso {
                nombre: "Encargado Test".to_string()
            }
        );
    }

    /// Los tipos no se mezclan: el préstamo KOF del 3 bloquea reportar
    /// perdido el 3 de KOF, pero no el 3 de contratista, que es otro gafete
    /// aunque lleve el mismo número. Y el conteo de contratistas no ve el
    /// préstamo KOF.
    #[test]
    fn el_gafete_en_uso_de_un_tipo_no_afecta_al_mismo_numero_de_otro_tipo() {
        let (core, actor, encargado_id) = nucleo_con_usuario_encargado_y_gafete();
        let gafete_contratista_3 = SqliteGafeteRepository::new(&core.connection)
            .crear(3, TipoGafete::Contratista)
            .unwrap();
        core.connection
            .execute(
                "INSERT INTO prestamos_gafete_provisional (
                    encargado_id, encargado_nombre, encargado_codigo_empleado, gafete_numero,
                    fecha_hora_entrega, usuario_entrega_id, usuario_entrega_nombre, uuid
                 ) VALUES (?1, 'Encargado Test', 'E1', 3, '2026-09-19T11:00:00Z', 1, 'Operador', 'p1')",
                params![encargado_id],
            )
            .unwrap();
        let gafete_kof_3 = SqliteGafeteRepository::new(&core.connection)
            .buscar_por_numero(3, TipoGafete::ProvisionalKof)
            .unwrap()
            .unwrap()
            .id;

        let error = core
            .reportar_gafete_perdido(&actor, gafete_kof_3, None, Some("no está"))
            .unwrap_err();
        assert!(matches!(
            error,
            GafeteServiceError::GafeteAsignado { nombre } if nombre == "Encargado Test"
        ));

        core.reportar_gafete_perdido(&actor, gafete_contratista_3, None, Some("no está"))
            .unwrap();

        let conteo_contratistas = core
            .comprobar_conteo_gafetes(&actor, TipoGafete::Contratista, &[])
            .unwrap();
        assert!(conteo_contratistas.en_uso.is_empty());
        assert_eq!(conteo_contratistas.perdidos, vec![3]);
    }
}
