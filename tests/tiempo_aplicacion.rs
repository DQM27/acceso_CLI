use std::sync::{Arc, Mutex};

use chrono::{DateTime, TimeZone, Utc};
use rusqlite::Connection;

use control_acceso::application::AppCore;
use control_acceso::database::queries::ingresos::FiltroIngresosActivos;
use control_acceso::database::schema::initialize_database;
use control_acceso::domain::resultado_acceso::ResultadoAcceso;
use control_acceso::models::medio_ingreso::MedioIngreso;
use control_acceso::models::usuario::RolUsuario;
use control_acceso::services::autenticacion_service::UsuarioSesion;
use control_acceso::services::error::RegistroIngresoServiceError;
use control_acceso::tiempo::Reloj;

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

#[test]
fn appcore_usa_costa_rica_para_reglas_utc_para_persistencia_y_detecta_retrocesos() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute_batch(
            "INSERT INTO empresas(id,nombre) VALUES (1,'Empresa');
             INSERT INTO usuarios(id,cedula,nombre,password_hash,rol,activo)
             VALUES (1,'U1','Operador','hash','OPERADOR',1);
             INSERT INTO contratistas(
                 id,cedula,nombre,empresa_id,tipo_ingreso,fecha_vencimiento_praind,
                 es_personal_ruta,tiene_acceso
             ) VALUES (1,'C1','Persona',1,'IN_HOUSE','2026-08-15',0,1);",
        )
        .unwrap();

    // En UTC ya es 16 de agosto, pero en Costa Rica todavía es 15 de agosto.
    let ingreso = Utc.with_ymd_and_hms(2026, 8, 16, 3, 30, 0).unwrap();
    let reloj = Arc::new(RelojControlado::new(ingreso));
    let core = AppCore::con_reloj(connection, reloj.clone());
    let actor = UsuarioSesion {
        id: 1,
        cedula: "U1".into(),
        nombre: "Operador".into(),
        rol: RolUsuario::Operador,
    };

    let resultado = core
        .registrar_ingreso(&actor, 1, MedioIngreso::Caminando, None)
        .unwrap();
    assert_eq!(
        resultado.resultado_acceso,
        ResultadoAcceso::PermitidoConAdvertencia
    );
    let activos = core
        .listar_ingresos_activos(&FiltroIngresosActivos::default())
        .unwrap();
    assert_eq!(activos.items[0].fecha_hora_ingreso, ingreso);

    reloj.establecer(Utc.with_ymd_and_hms(2026, 8, 16, 3, 29, 59).unwrap());
    assert!(matches!(
        core.registrar_salida(&actor, resultado.registro_id),
        Err(RegistroIngresoServiceError::RelojRetrocedido)
    ));
    assert_eq!(
        core.listar_ingresos_activos(&FiltroIngresosActivos::default())
            .unwrap()
            .items
            .len(),
        1
    );

    reloj.establecer(Utc.with_ymd_and_hms(2026, 8, 16, 3, 31, 0).unwrap());
    core.registrar_salida(&actor, resultado.registro_id)
        .unwrap();
}
