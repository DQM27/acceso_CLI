use std::sync::{Arc, Mutex};

use chrono::{DateTime, TimeZone, Utc};
use rusqlite::Connection;

use control_acceso::application::AppCore;
use control_acceso::database::schema::initialize_database;
use control_acceso::domain::cita::MotivoDenegacionVisita;
use control_acceso::models::usuario::RolUsuario;
use control_acceso::services::autenticacion_service::UsuarioSesion;
use control_acceso::services::error::CitaServiceError;
use control_acceso::tiempo::Reloj;

/// Mismo criterio que `tests/operador_activo.rs` (hallazgo #2 de
/// `docs/auditoria-dominio-2026-08-20.md`) pero para visitas: verifica que
/// `AppCore::registrar_entrada_visita`/`registrar_salida_visita` heredan la
/// misma protección contra una sesión desactivada, aunque `CitaService` en
/// sí no sepa nada de usuarios/roles.
fn base() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute_batch(
            "INSERT INTO usuarios(id,cedula,nombre,password_hash,rol,activo)
             VALUES
                (1,'U1','Operador Activo','hash','OPERADOR',1),
                (2,'U2','Operador Desactivado','hash','OPERADOR',0);
             INSERT INTO citas (id, uuid, fecha_desde, fecha_hasta, anfitrion_nombre,
                anfitrion_correo, estado, creado_en)
             VALUES (1, 'uuid-cita-1', '2026-08-10', '2026-08-15', 'Anfitrión',
                'anfitrion@ejemplo.com', 'VIGENTE', '2026-08-01T00:00:00Z');
             INSERT INTO cita_visitantes (id, uuid, cita_id, cedula, nombre)
             VALUES (1, 'uuid-visitante-1', 1, '1-2345', 'Visitante');",
        )
        .unwrap();
    connection
}

fn actor(id: i64) -> UsuarioSesion {
    UsuarioSesion {
        id,
        cedula: format!("U{id}"),
        nombre: "Operador".into(),
        rol: RolUsuario::Operador,
    }
}

struct RelojControlado {
    instante: Mutex<DateTime<Utc>>,
}

impl RelojControlado {
    fn new(instante: DateTime<Utc>) -> Self {
        Self {
            instante: Mutex::new(instante),
        }
    }

    fn establecer(&self, instante: DateTime<Utc>) {
        *self.instante.lock().unwrap() = instante;
    }
}

impl Reloj for RelojControlado {
    fn ahora_utc(&self) -> DateTime<Utc> {
        *self.instante.lock().unwrap()
    }
}

fn en_vigencia() -> DateTime<Utc> {
    // Dentro del rango 2026-08-10..2026-08-15 de la cita sembrada en `base()`.
    Utc.with_ymd_and_hms(2026, 8, 12, 12, 0, 0).unwrap()
}

#[test]
fn registrar_entrada_visita_rechaza_un_usuario_desactivado() {
    let core = AppCore::con_reloj(base(), Arc::new(RelojControlado::new(en_vigencia())));

    let resultado = core.registrar_entrada_visita(&actor(2), "1-2345", None);

    assert!(matches!(
        resultado,
        Err(CitaServiceError::OperadorNoAutorizado)
    ));
}

#[test]
fn registrar_entrada_visita_rechaza_un_usuario_inexistente() {
    let core = AppCore::con_reloj(base(), Arc::new(RelojControlado::new(en_vigencia())));

    let resultado = core.registrar_entrada_visita(&actor(999), "1-2345", None);

    assert!(matches!(
        resultado,
        Err(CitaServiceError::OperadorNoAutorizado)
    ));
}

#[test]
fn registrar_salida_visita_rechaza_un_usuario_desactivado_aunque_el_movimiento_sea_valido() {
    let core = AppCore::con_reloj(base(), Arc::new(RelojControlado::new(en_vigencia())));
    let movimiento_id = core
        .registrar_entrada_visita(&actor(1), "1-2345", None)
        .unwrap();

    let resultado = core.registrar_salida_visita(&actor(2), movimiento_id);

    assert!(matches!(
        resultado,
        Err(CitaServiceError::OperadorNoAutorizado)
    ));
    // La salida no se aplicó -- sigue pudiendo cerrarse con un operador válido.
    core.registrar_salida_visita(&actor(1), movimiento_id)
        .unwrap();
}

#[test]
fn registrar_entrada_y_salida_visita_funcionan_con_un_operador_activo() {
    let core = AppCore::con_reloj(base(), Arc::new(RelojControlado::new(en_vigencia())));

    let movimiento_id = core
        .registrar_entrada_visita(&actor(1), "1-2345", Some(7))
        .unwrap();
    core.registrar_salida_visita(&actor(1), movimiento_id)
        .unwrap();
}

#[test]
fn registrar_entrada_visita_propaga_el_motivo_de_denegacion_del_servicio() {
    let connection = base();
    connection
        .execute("UPDATE citas SET estado = 'CANCELADA' WHERE id = 1", [])
        .unwrap();
    let core = AppCore::con_reloj(connection, Arc::new(RelojControlado::new(en_vigencia())));

    let resultado = core.registrar_entrada_visita(&actor(1), "1-2345", None);

    assert!(matches!(
        resultado,
        Err(CitaServiceError::SinCitaVigente(
            MotivoDenegacionVisita::CitaCancelada
        ))
    ));
}

#[test]
fn registrar_salida_visita_detecta_un_reloj_retrocedido() {
    let reloj = Arc::new(RelojControlado::new(en_vigencia()));
    let core = AppCore::con_reloj(base(), reloj.clone());
    let movimiento_id = core
        .registrar_entrada_visita(&actor(1), "1-2345", None)
        .unwrap();

    reloj.establecer(en_vigencia() - chrono::Duration::minutes(1));

    assert!(matches!(
        core.registrar_salida_visita(&actor(1), movimiento_id),
        Err(CitaServiceError::RelojRetrocedido)
    ));
}
