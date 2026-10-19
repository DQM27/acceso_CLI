//! Pruebas de caracterización de la salida de un contratista
//! (`AppCore::registrar_salida`).
//!
//! Fijan el comportamiento de hoy para poder mover la salida a una
//! arquitectura hexagonal (unidad de trabajo, puertos en `application`, lote
//! de salidas) sin cambiarlo a escondidas. Sólo pasan por la fachada pública
//! y miran la base con una segunda conexión, así que siguen valiendo aunque
//! por dentro cambie quién abre la transacción o dónde viven los traits.
//!
//! Lo que fijan: la salida normal sella hora y operador y encola el aviso a
//! la nube; una segunda salida falla sin pisar la primera; un id que no
//! existe falla; el gafete queda libre para otro y sólo cuando la salida se
//! confirmó; y toda salida rechazada (reloj retrocedido, operador inactivo)
//! no deja escritura alguna, ni siquiera en la cola de sincronización.
//!
//! Al final, las del lote (`AppCore::registrar_salidas`): cierra varias
//! salidas en una transacción, una rechazada no frena a las demás, y los
//! errores globales (reloj, operador) no cierran ninguna.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, TimeZone, Utc};
use rusqlite::Connection;

use control_acceso::application::AppCore;
use control_acceso::database::schema::initialize_database;
use control_acceso::models::medio_ingreso::MedioIngreso;
use control_acceso::models::usuario::RolUsuario;
use control_acceso::services::autenticacion_service::UsuarioSesion;
use control_acceso::services::error::RegistroIngresoServiceError;
use control_acceso::tiempo::Reloj;

struct RelojControlado {
    instante: Mutex<DateTime<Utc>>,
}

impl RelojControlado {
    fn new(instante: DateTime<Utc>) -> Arc<Self> {
        Arc::new(Self {
            instante: Mutex::new(instante),
        })
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

fn instante(hora: u32, minuto: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 17, hora, minuto, 0).unwrap()
}

/// Un escenario con base en archivo (para mirarla con otra conexión): dos
/// operadores activos, uno inactivo, dos contratistas PRAIND vigentes y un
/// gafete de contratista, el 25.
struct Escenario {
    ruta: PathBuf,
    reloj: Arc<RelojControlado>,
    core: AppCore,
    inspector: Connection,
}

impl Escenario {
    fn nuevo(nombre: &str) -> Self {
        let ruta = std::env::temp_dir().join(format!(
            "caracterizacion_salida_{nombre}_{}.db",
            std::process::id()
        ));
        limpiar(&ruta);
        let connection = Connection::open(&ruta).unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute_batch(
                "INSERT INTO empresas(id,nombre) VALUES (1,'Empresa');
                 INSERT INTO usuarios(id,cedula,nombre,password_hash,rol,activo) VALUES
                    (1,'U1','Operador Uno','hash','OPERADOR',1),
                    (2,'U2','Operador Dos','hash','OPERADOR',1),
                    (3,'U3','Operador Inactivo','hash','OPERADOR',0);
                 INSERT INTO contratistas(
                    id,cedula,nombre,empresa_id,tipo_ingreso,fecha_vencimiento_praind,
                    es_personal_ruta,tiene_acceso
                 ) VALUES
                    (1,'C1','Persona Uno',1,'PRAIND','2027-01-01',0,1),
                    (2,'C2','Persona Dos',1,'PRAIND','2027-01-01',0,1),
                    (3,'C3','Persona Tres',1,'PRAIND','2027-01-01',0,1),
                    (4,'C4','Persona Cuatro',1,'PRAIND','2027-01-01',0,1);
                 INSERT INTO gafetes(numero,tipo,estado) VALUES (25,'CONTRATISTA','DISPONIBLE');",
            )
            .unwrap();
        let reloj = RelojControlado::new(instante(14, 0));
        let core = AppCore::con_reloj(connection, reloj.clone());
        let inspector = Connection::open(&ruta).unwrap();
        Self {
            ruta,
            reloj,
            core,
            inspector,
        }
    }

    /// Entrada de `contratista` con el gafete 25 y el reloj en `hora:minuto`.
    fn entrar(&self, actor: &UsuarioSesion, contratista: i64, hora: u32, minuto: u32) -> i64 {
        self.reloj.establecer(instante(hora, minuto));
        self.core
            .registrar_ingreso(actor, contratista, MedioIngreso::Caminando, Some(25), None)
            .unwrap()
            .registro_id
    }

    /// Entrada de `contratista` sin gafete (S/G) con el reloj en `hora:minuto`.
    fn entrar_sin_gafete(
        &self,
        actor: &UsuarioSesion,
        contratista: i64,
        hora: u32,
        minuto: u32,
    ) -> i64 {
        self.reloj.establecer(instante(hora, minuto));
        self.core
            .registrar_ingreso(actor, contratista, MedioIngreso::Caminando, None, None)
            .unwrap()
            .registro_id
    }

    fn salida_de(&self, id: i64) -> (Option<String>, Option<i64>) {
        self.inspector
            .query_row(
                "SELECT fecha_hora_salida, usuario_salida_id FROM registro_ingresos WHERE id = ?1",
                [id],
                |fila| Ok((fila.get(0)?, fila.get(1)?)),
            )
            .unwrap()
    }

    fn avisos_de_cierre(&self) -> i64 {
        self.inspector
            .query_row(
                "SELECT COUNT(*) FROM cola_salida WHERE entidad = 'ingreso' AND operacion = 'cerrar'",
                [],
                |fila| fila.get(0),
            )
            .unwrap()
    }
}

impl Drop for Escenario {
    fn drop(&mut self) {
        limpiar(&self.ruta);
    }
}

fn limpiar(ruta: &Path) {
    for extension in ["db", "db-journal", "db-wal", "db-shm"] {
        let _ = fs::remove_file(ruta.with_extension(extension));
    }
}

fn operador(id: i64) -> UsuarioSesion {
    UsuarioSesion {
        id,
        cedula: format!("U{id}"),
        nombre: format!("Operador {id}"),
        rol: RolUsuario::Operador,
    }
}

#[test]
fn la_salida_normal_sella_hora_y_operador_y_encola_el_aviso() {
    let escenario = Escenario::nuevo("normal");
    let id = escenario.entrar(&operador(1), 1, 14, 0);
    assert_eq!(escenario.salida_de(id), (None, None));

    escenario.reloj.establecer(instante(15, 30));
    escenario.core.registrar_salida(&operador(2), id).unwrap();

    let (hora, usuario) = escenario.salida_de(id);
    let hora = hora.expect("la salida debe quedar sellada");
    assert!(
        hora.starts_with("2026-10-17T15:30:00"),
        "se sella con el reloj del núcleo, no con el de la base: {hora}"
    );
    assert_eq!(usuario, Some(2), "la salida la firma quien la registra");
    assert_eq!(escenario.avisos_de_cierre(), 1);
}

#[test]
fn una_segunda_salida_falla_y_no_pisa_la_primera() {
    let escenario = Escenario::nuevo("doble");
    let id = escenario.entrar(&operador(1), 1, 14, 0);
    escenario.reloj.establecer(instante(15, 0));
    escenario.core.registrar_salida(&operador(1), id).unwrap();
    let primera = escenario.salida_de(id);

    escenario.reloj.establecer(instante(16, 0));
    let segunda = escenario.core.registrar_salida(&operador(2), id);

    assert!(matches!(
        segunda,
        Err(RegistroIngresoServiceError::RegistroNoActivo)
    ));
    assert_eq!(
        escenario.salida_de(id),
        primera,
        "la hora y el operador de la primera salida no se tocan"
    );
    assert_eq!(
        escenario.avisos_de_cierre(),
        1,
        "la segunda salida no encola otro aviso"
    );
}

#[test]
fn la_salida_de_un_ingreso_que_no_existe_falla_sin_escribir() {
    let escenario = Escenario::nuevo("inexistente");

    let resultado = escenario.core.registrar_salida(&operador(1), 9_999);

    assert!(matches!(
        resultado,
        Err(RegistroIngresoServiceError::RegistroNoActivo)
    ));
    assert_eq!(escenario.avisos_de_cierre(), 0);
}

#[test]
fn la_salida_libera_el_gafete_para_otro_contratista() {
    let escenario = Escenario::nuevo("libera_gafete");
    let id = escenario.entrar(&operador(1), 1, 14, 0);

    escenario.reloj.establecer(instante(14, 10));
    let mientras_esta_adentro =
        escenario
            .core
            .registrar_ingreso(&operador(1), 2, MedioIngreso::Caminando, Some(25), None);
    assert!(
        matches!(
            mientras_esta_adentro,
            Err(RegistroIngresoServiceError::GafeteOcupado)
        ),
        "con el primero adentro, el gafete 25 está ocupado"
    );

    escenario.reloj.establecer(instante(15, 0));
    escenario.core.registrar_salida(&operador(1), id).unwrap();

    escenario.reloj.establecer(instante(15, 5));
    escenario
        .core
        .registrar_ingreso(&operador(1), 2, MedioIngreso::Caminando, Some(25), None)
        .expect("tras la salida, el gafete 25 queda libre");
}

#[test]
fn con_el_reloj_retrocedido_no_se_escribe_nada_y_el_gafete_sigue_ocupado() {
    let escenario = Escenario::nuevo("reloj");
    let id = escenario.entrar(&operador(1), 1, 14, 0);

    escenario.reloj.establecer(instante(13, 59));
    let resultado = escenario.core.registrar_salida(&operador(1), id);

    assert!(matches!(
        resultado,
        Err(RegistroIngresoServiceError::RelojRetrocedido)
    ));
    assert_eq!(escenario.salida_de(id), (None, None));
    assert_eq!(escenario.avisos_de_cierre(), 0);

    // Y el gafete 25 sigue en manos del primero.
    escenario.reloj.establecer(instante(14, 30));
    assert!(matches!(
        escenario
            .core
            .registrar_ingreso(&operador(1), 2, MedioIngreso::Caminando, Some(25), None),
        Err(RegistroIngresoServiceError::GafeteOcupado)
    ));
}

#[test]
fn un_operador_inactivo_no_cierra_nada_ni_libera_el_gafete() {
    let escenario = Escenario::nuevo("inactivo");
    let id = escenario.entrar(&operador(1), 1, 14, 0);

    escenario.reloj.establecer(instante(15, 0));
    let resultado = escenario.core.registrar_salida(&operador(3), id);

    assert!(matches!(
        resultado,
        Err(RegistroIngresoServiceError::OperadorNoAutorizado)
    ));
    assert_eq!(escenario.salida_de(id), (None, None));
    assert_eq!(escenario.avisos_de_cierre(), 0);
    assert!(matches!(
        escenario
            .core
            .registrar_ingreso(&operador(1), 2, MedioIngreso::Caminando, Some(25), None),
        Err(RegistroIngresoServiceError::GafeteOcupado)
    ));
}

#[test]
fn una_salida_rechazada_no_impide_la_siguiente_y_cada_una_se_confirma_sola() {
    let escenario = Escenario::nuevo("independientes");
    let primero = escenario.entrar(&operador(1), 1, 14, 0);
    escenario.reloj.establecer(instante(14, 5));
    escenario
        .core
        .registrar_ingreso(&operador(1), 2, MedioIngreso::Caminando, None, None)
        .unwrap();

    escenario.reloj.establecer(instante(15, 0));
    assert!(
        escenario
            .core
            .registrar_salida(&operador(1), 9_999)
            .is_err()
    );
    escenario
        .core
        .registrar_salida(&operador(1), primero)
        .expect("un fallo anterior no deja la conexión en una transacción abierta");

    assert!(escenario.salida_de(primero).0.is_some());
    assert_eq!(escenario.avisos_de_cierre(), 1);
}

// ---- Lote: `registrar_salidas` ----

/// Tres contratistas adentro (el primero con el gafete 25) a las 14:00,
/// 14:01 y 14:02.
fn tres_adentro(escenario: &Escenario) -> [i64; 3] {
    [
        escenario.entrar(&operador(1), 1, 14, 0),
        escenario.entrar_sin_gafete(&operador(1), 2, 14, 1),
        escenario.entrar_sin_gafete(&operador(1), 3, 14, 2),
    ]
}

#[test]
fn el_lote_cierra_todas_con_la_misma_hora_y_el_mismo_operador() {
    let escenario = Escenario::nuevo("lote_todas");
    let ids = tres_adentro(&escenario);

    escenario.reloj.establecer(instante(16, 0));
    let resultado = escenario
        .core
        .registrar_salidas(&operador(2), &ids)
        .unwrap();

    assert_eq!(resultado.cerrados, ids, "en el orden en que llegaron");
    assert!(resultado.rechazados.is_empty());
    for id in ids {
        let (hora, usuario) = escenario.salida_de(id);
        assert!(hora.unwrap().starts_with("2026-10-17T16:00:00"));
        assert_eq!(usuario, Some(2));
    }
    assert_eq!(escenario.avisos_de_cierre(), 3, "un aviso por salida");
}

#[test]
fn el_lote_libera_el_gafete_de_quien_lo_tenia() {
    let escenario = Escenario::nuevo("lote_gafete");
    let ids = tres_adentro(&escenario);

    escenario.reloj.establecer(instante(16, 0));
    escenario
        .core
        .registrar_salidas(&operador(1), &ids)
        .unwrap();

    escenario.reloj.establecer(instante(16, 5));
    escenario
        .core
        .registrar_ingreso(&operador(1), 4, MedioIngreso::Caminando, Some(25), None)
        .expect("el gafete 25 quedó libre");
}

#[test]
fn una_rechazada_no_frena_a_las_demas_y_trae_su_mensaje() {
    let escenario = Escenario::nuevo("lote_rechazada");
    let [primero, segundo, tercero] = tres_adentro(&escenario);

    escenario.reloj.establecer(instante(16, 0));
    let resultado = escenario
        .core
        .registrar_salidas(&operador(1), &[primero, 9_999, segundo])
        .unwrap();

    assert_eq!(resultado.cerrados, [primero, segundo]);
    assert_eq!(resultado.rechazados.len(), 1);
    assert_eq!(resultado.rechazados[0].id, 9_999);
    assert_eq!(
        resultado.rechazados[0].mensaje,
        "El ingreso ya no está activo"
    );
    assert!(escenario.salida_de(primero).0.is_some());
    assert!(escenario.salida_de(segundo).0.is_some());
    assert_eq!(
        escenario.salida_de(tercero),
        (None, None),
        "el que no iba en el lote sigue adentro"
    );
    assert_eq!(escenario.avisos_de_cierre(), 2);
}

#[test]
fn en_el_lote_un_ingreso_ya_cerrado_se_rechaza_sin_pisar_su_salida() {
    let escenario = Escenario::nuevo("lote_ya_cerrado");
    let [primero, segundo, _] = tres_adentro(&escenario);
    escenario.reloj.establecer(instante(15, 0));
    escenario
        .core
        .registrar_salida(&operador(1), primero)
        .unwrap();
    let original = escenario.salida_de(primero);

    escenario.reloj.establecer(instante(16, 0));
    let resultado = escenario
        .core
        .registrar_salidas(&operador(2), &[primero, segundo])
        .unwrap();

    assert_eq!(resultado.cerrados, [segundo]);
    assert_eq!(resultado.rechazados[0].id, primero);
    assert_eq!(escenario.salida_de(primero), original);
    assert_eq!(
        escenario.avisos_de_cierre(),
        2,
        "la del primero y la del segundo"
    );
}

#[test]
fn en_el_lote_un_id_repetido_cierra_una_vez_y_rechaza_la_repetida() {
    let escenario = Escenario::nuevo("lote_repetido");
    let [primero, ..] = tres_adentro(&escenario);

    escenario.reloj.establecer(instante(16, 0));
    let resultado = escenario
        .core
        .registrar_salidas(&operador(1), &[primero, primero])
        .unwrap();

    assert_eq!(resultado.cerrados, [primero]);
    assert_eq!(resultado.rechazados.len(), 1);
    assert_eq!(escenario.avisos_de_cierre(), 1);
}

#[test]
fn el_lote_con_el_reloj_retrocedido_no_cierra_ninguna() {
    let escenario = Escenario::nuevo("lote_reloj");
    let ids = tres_adentro(&escenario);

    escenario.reloj.establecer(instante(13, 0));
    let resultado = escenario.core.registrar_salidas(&operador(1), &ids);

    assert!(matches!(
        resultado,
        Err(RegistroIngresoServiceError::RelojRetrocedido)
    ));
    for id in ids {
        assert_eq!(escenario.salida_de(id), (None, None));
    }
    assert_eq!(escenario.avisos_de_cierre(), 0);
}

#[test]
fn el_lote_con_un_operador_inactivo_no_cierra_ninguna() {
    let escenario = Escenario::nuevo("lote_inactivo");
    let ids = tres_adentro(&escenario);

    escenario.reloj.establecer(instante(16, 0));
    let resultado = escenario.core.registrar_salidas(&operador(3), &ids);

    assert!(matches!(
        resultado,
        Err(RegistroIngresoServiceError::OperadorNoAutorizado)
    ));
    for id in ids {
        assert_eq!(escenario.salida_de(id), (None, None));
    }
    assert_eq!(escenario.avisos_de_cierre(), 0);
}

#[test]
fn un_lote_vacio_no_cierra_nada_y_no_falla() {
    let escenario = Escenario::nuevo("lote_vacio");
    tres_adentro(&escenario);

    let resultado = escenario.core.registrar_salidas(&operador(1), &[]).unwrap();

    assert!(resultado.cerrados.is_empty());
    assert!(resultado.rechazados.is_empty());
    assert_eq!(escenario.avisos_de_cierre(), 0);
}

#[test]
fn el_lote_da_lo_mismo_que_cerrar_una_por_una() {
    let en_lote = Escenario::nuevo("equivalencia_lote");
    let una_por_una = Escenario::nuevo("equivalencia_una_por_una");
    let ids_lote = tres_adentro(&en_lote);
    let ids_una = tres_adentro(&una_por_una);

    en_lote.reloj.establecer(instante(16, 0));
    en_lote
        .core
        .registrar_salidas(&operador(2), &ids_lote)
        .unwrap();
    una_por_una.reloj.establecer(instante(16, 0));
    for id in ids_una {
        una_por_una.core.registrar_salida(&operador(2), id).unwrap();
    }

    for (lote, una) in ids_lote.into_iter().zip(ids_una) {
        assert_eq!(en_lote.salida_de(lote), una_por_una.salida_de(una));
    }
    assert_eq!(en_lote.avisos_de_cierre(), una_por_una.avisos_de_cierre());
}
