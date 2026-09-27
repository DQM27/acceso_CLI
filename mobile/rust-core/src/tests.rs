use super::*;

#[test]
fn abre_una_base_de_datos_temporal() {
    let archivo = tempfile::NamedTempFile::new().unwrap();
    let ruta = archivo.path().to_str().unwrap().to_string();

    let resultado = Nucleo::abrir(ruta);

    assert!(resultado.is_ok());
}

// --- abrir_cifrado (MV-03, auditoría 2026-09-24) ---

#[test]
fn abrir_cifrado_en_archivo_nuevo_no_necesita_reconstruccion() {
    let archivo = tempfile::NamedTempFile::new().unwrap();
    let ruta = archivo.path().to_str().unwrap().to_string();

    let resultado = Nucleo::abrir_cifrado(ruta, vec![7u8; 32]);

    assert!(resultado.is_ok());
}

#[test]
fn abrir_cifrado_rechaza_una_clave_que_no_mide_32_bytes() {
    let archivo = tempfile::NamedTempFile::new().unwrap();
    let ruta = archivo.path().to_str().unwrap().to_string();

    let resultado = Nucleo::abrir_cifrado(ruta, vec![1, 2, 3]);

    assert!(matches!(resultado, Err(NucleoError::Interno { .. })));
}

#[test]
fn abrir_cifrado_reconstruye_si_el_archivo_existente_no_es_legible() {
    // Simula el caso real de todo teléfono con la app instalada antes
    // de este cambio (base en texto plano) o un archivo corrupto --
    // decisión explícita del usuario (2026-09-26): en vez de fallar o
    // migrar byte a byte, se descarta y se reconstruye vacía. El
    // catálogo vuelve solo por la sincronización normal.
    let archivo = tempfile::NamedTempFile::new().unwrap();
    let ruta = archivo.path().to_str().unwrap().to_string();
    std::fs::write(&ruta, b"esto no es un archivo SQLite valido").unwrap();

    let resultado = Nucleo::abrir_cifrado(ruta.clone(), vec![7u8; 32]);
    let error = resultado.as_ref().err().map(ToString::to_string);

    assert!(
        resultado.is_ok(),
        "debería reconstruir en vez de fallar: {error:?}"
    );
    let bytes = std::fs::read(&ruta).unwrap();
    assert_ne!(bytes, b"esto no es un archivo SQLite valido".to_vec());
}

// Mismo criterio que `sqlite3mc_cifra_de_verdad_a_traves_de_open_database_cifrada`
// en el crate raíz (`src/database/connection.rs`): confirma que
// `abrir_cifrado` cifra de verdad, no sólo que compila y enlaza. Es
// también la guarda contra un SQLite plano colado por unificación de
// features de Cargo (pasó de verdad el 2026-09-26, ver
// docs/decisiones-tecnicas.md).
#[test]
fn abrir_cifrado_sqlite3mc_cifra_de_verdad() {
    let archivo = tempfile::NamedTempFile::new().unwrap();
    let ruta = archivo.path().to_str().unwrap().to_string();

    let nucleo = Nucleo::abrir_cifrado(ruta.clone(), vec![7u8; 32]).unwrap();
    drop(nucleo);

    let bytes = std::fs::read(&ruta).unwrap();
    assert!(
        !bytes.starts_with(b"SQLite format 3\0"),
        "la base quedó sin cifrar de verdad: se enlazó un SQLite plano en vez de SQLite3MC"
    );
}

// Bug real encontrado en la prueba de fuego en un dispositivo real
// (2026-09-26): `conexion_secundaria()` pasaba `None` siempre, sin
// importar con qué clave se hubiera abierto la conexión principal --
// cualquier flujo que la usara (toda la sincronización) reventaba con
// "file is not a database" apenas tocaba la base ya cifrada.
#[test]
fn conexion_secundaria_usa_la_misma_clave_que_abrir_cifrado() {
    let archivo = tempfile::NamedTempFile::new().unwrap();
    let ruta = archivo.path().to_str().unwrap().to_string();
    let nucleo = Nucleo::abrir_cifrado(ruta, vec![7u8; 32]).unwrap();

    let conexion = nucleo.conexion_secundaria();
    let error = conexion.as_ref().err().map(ToString::to_string);
    assert!(conexion.is_ok(), "conexion_secundaria() falló: {error:?}");

    let cuenta: i64 = conexion
        .unwrap()
        .query_row("SELECT count(*) FROM usuarios", [], |row| row.get(0))
        .unwrap();
    assert_eq!(cuenta, 0);
}

#[test]
fn autenticar_con_credenciales_invalidas_falla() {
    let archivo = tempfile::NamedTempFile::new().unwrap();
    let ruta = archivo.path().to_str().unwrap().to_string();
    let nucleo = Nucleo::abrir(ruta).unwrap();

    let resultado = nucleo.autenticar_con_secreto(
        "000000000".to_string(),
        "loquesea".to_string(),
        String::new(),
    );

    assert!(matches!(resultado, Err(NucleoError::CredencialesInvalidas)));
}

#[test]
fn buscar_contratistas_en_base_vacia_no_falla() {
    let archivo = tempfile::NamedTempFile::new().unwrap();
    let ruta = archivo.path().to_str().unwrap().to_string();
    let nucleo = Nucleo::abrir(ruta).unwrap();

    let resultado = nucleo.buscar_contratistas(String::new());

    assert_eq!(resultado.unwrap(), Vec::new());
}

#[test]
fn preparar_y_registrar_ingreso_sin_gafete() {
    let archivo = tempfile::NamedTempFile::new().unwrap();
    let ruta = archivo.path().to_str().unwrap().to_string();

    // Aplica el esquema real y siembra lo mínimo: SWAT no requiere ni
    // PRAIND ni gafete (domain::contratista), así que es el caso feliz
    // más simple para probar el camino completo.
    let conexion = control_acceso::database::connection::open_database(&ruta).unwrap();
    conexion
        .execute_batch(
            "INSERT INTO empresas (nombre) VALUES ('Empresa Test');
             INSERT INTO contratistas (
                 cedula, nombre, empresa_id, tipo_ingreso, es_personal_ruta, tiene_acceso
             ) VALUES ('111111111', 'Contratista Test', 1, 'SWAT', 0, 1);
             INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo) VALUES (
                 '999999999', 'Actor Test',
                 '$argon2id$v=19$m=19456,t=2,p=1$pO+/qvY8ieaUA97ME2LUPQ$OfE/070ufOj4TtL2SzVyW3sefnJjrMJq32APEHrM/wI',
                 'ROOT', 1
             );",
        )
        .unwrap();
    drop(conexion);

    let nucleo = Nucleo::abrir(ruta).unwrap();
    nucleo
        .autenticar_con_secreto(
            "999999999".to_string(),
            "clave_prueba_123".to_string(),
            String::new(),
        )
        .unwrap();

    let preparacion = nucleo.preparar_ingreso(1).unwrap();
    assert!(!preparacion.requiere_gafete);
    assert_eq!(preparacion.resultado_acceso, ResultadoAcceso::Permitido);

    let resultado = nucleo
        .registrar_ingreso(1, MedioIngreso::Caminando, None, None)
        .unwrap();
    assert_eq!(resultado.resultado_acceso, ResultadoAcceso::Permitido);
}

/// Con `secreto` vacío ninguno de los dos métodos `_con_secreto` debe
/// tocar la red -- `CacheTokenDispositivo` corta antes de intentar
/// nada, así que el resultado tiene que ser idéntico al de las
/// versiones sin secreto. Corre en CI (sin acceso a Internet) sin
/// necesitar mockear HTTP.
#[test]
fn con_secreto_vacio_los_metodos_con_secreto_no_tocan_la_red() {
    let archivo = tempfile::NamedTempFile::new().unwrap();
    let ruta = archivo.path().to_str().unwrap().to_string();

    let conexion = control_acceso::database::connection::open_database(&ruta).unwrap();
    conexion
        .execute_batch(
            "INSERT INTO empresas (nombre) VALUES ('Empresa Test');
             INSERT INTO contratistas (
                 cedula, nombre, empresa_id, tipo_ingreso, es_personal_ruta, tiene_acceso
             ) VALUES ('111111111', 'Contratista Test', 1, 'SWAT', 0, 1);
             INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo) VALUES (
                 '999999999', 'Actor Test',
                 '$argon2id$v=19$m=19456,t=2,p=1$pO+/qvY8ieaUA97ME2LUPQ$OfE/070ufOj4TtL2SzVyW3sefnJjrMJq32APEHrM/wI',
                 'ROOT', 1
             );",
        )
        .unwrap();
    drop(conexion);

    let nucleo = Nucleo::abrir(ruta).unwrap();
    nucleo
        .autenticar_con_secreto(
            "999999999".to_string(),
            "clave_prueba_123".to_string(),
            String::new(),
        )
        .unwrap();

    let preparacion = nucleo
        .preparar_ingreso_con_secreto(1, String::new())
        .unwrap();
    assert_eq!(preparacion.mensaje_bloqueo, None);
    assert_eq!(preparacion.activo_en_otro_sitio, None);

    let resultado = nucleo
        .registrar_ingreso_con_secreto(1, MedioIngreso::Caminando, None, None, String::new())
        .unwrap();
    assert_eq!(resultado.resultado_acceso, ResultadoAcceso::Permitido);
}

/// Cuando el bloqueo YA es local (ingreso activo) `preparar_ingreso_con_secreto`
/// ni siquiera debe intentar el chequeo remoto -- de eso depende que
/// este test pueda correr sin red: si tocara `CacheTokenDispositivo`
/// con un secreto no vacío, fallaría por falta de conexión en CI. El
/// campo que le interesa a Kotlin es `mensaje_bloqueo`: antes tenía
/// que recalcularlo con `puedeContinuar`/`mensajeBloqueo` propios.
#[test]
fn preparar_ingreso_con_secreto_no_toca_la_red_si_ya_hay_bloqueo_local() {
    let archivo = tempfile::NamedTempFile::new().unwrap();
    let ruta = archivo.path().to_str().unwrap().to_string();

    let conexion = control_acceso::database::connection::open_database(&ruta).unwrap();
    conexion
        .execute_batch(
            "INSERT INTO empresas (nombre) VALUES ('Empresa Test');
             INSERT INTO contratistas (
                 cedula, nombre, empresa_id, tipo_ingreso, es_personal_ruta, tiene_acceso
             ) VALUES ('111111111', 'Contratista Test', 1, 'SWAT', 0, 1);
             INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo) VALUES (
                 '999999999', 'Actor Test',
                 '$argon2id$v=19$m=19456,t=2,p=1$pO+/qvY8ieaUA97ME2LUPQ$OfE/070ufOj4TtL2SzVyW3sefnJjrMJq32APEHrM/wI',
                 'ROOT', 1
             );",
        )
        .unwrap();
    drop(conexion);

    let nucleo = Nucleo::abrir(ruta).unwrap();
    nucleo
        .autenticar_con_secreto(
            "999999999".to_string(),
            "clave_prueba_123".to_string(),
            String::new(),
        )
        .unwrap();
    nucleo
        .registrar_ingreso(1, MedioIngreso::Caminando, None, None)
        .unwrap();

    // Secreto NO vacío a propósito -- si el chequeo remoto se
    // intentara igual, este test colgaría o fallaría por falta de red
    // en vez de terminar en microsegundos.
    let preparacion = nucleo
        .preparar_ingreso_con_secreto(1, "secreto-de-prueba".to_string())
        .unwrap();

    assert!(preparacion.tiene_ingreso_activo);
    assert_eq!(
        preparacion.mensaje_bloqueo,
        Some("El contratista ya tiene un ingreso activo.".to_string())
    );
}

/// Secreto vacío = nube sin configurar: se registra con las reglas
/// locales sin tocar la red. Con secreto, la verificación en vivo es
/// estricta (si no se puede verificar, no se registra); eso lo prueban
/// los tests de `application::con_nube` contra un servidor falso, nunca
/// contra la nube real.
#[test]
fn registrar_ingreso_con_secreto_vacio_no_toca_la_red() {
    let archivo = tempfile::NamedTempFile::new().unwrap();
    let ruta = archivo.path().to_str().unwrap().to_string();

    let conexion = control_acceso::database::connection::open_database(&ruta).unwrap();
    conexion
        .execute_batch(
            "INSERT INTO empresas (nombre) VALUES ('Empresa Test');
             INSERT INTO contratistas (
                 cedula, nombre, empresa_id, tipo_ingreso, es_personal_ruta, tiene_acceso
             ) VALUES ('111111111', 'Contratista Test', 1, 'SWAT', 0, 1);
             INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo) VALUES (
                 '999999999', 'Actor Test',
                 '$argon2id$v=19$m=19456,t=2,p=1$pO+/qvY8ieaUA97ME2LUPQ$OfE/070ufOj4TtL2SzVyW3sefnJjrMJq32APEHrM/wI',
                 'ROOT', 1
             );",
        )
        .unwrap();
    drop(conexion);

    let nucleo = Nucleo::abrir(ruta).unwrap();
    nucleo
        .autenticar_con_secreto(
            "999999999".to_string(),
            "clave_prueba_123".to_string(),
            String::new(),
        )
        .unwrap();

    let resultado = nucleo
        .registrar_ingreso_con_secreto(1, MedioIngreso::Caminando, None, None, String::new())
        .unwrap();

    assert_eq!(resultado.resultado_acceso, ResultadoAcceso::Permitido);
}

#[test]
fn listar_activos_y_registrar_salida() {
    let archivo = tempfile::NamedTempFile::new().unwrap();
    let ruta = archivo.path().to_str().unwrap().to_string();

    let conexion = control_acceso::database::connection::open_database(&ruta).unwrap();
    conexion
        .execute_batch(
            "INSERT INTO empresas (nombre) VALUES ('Empresa Test');
             INSERT INTO contratistas (
                 cedula, nombre, empresa_id, tipo_ingreso, es_personal_ruta, tiene_acceso
             ) VALUES ('111111111', 'Contratista Test', 1, 'SWAT', 0, 1);
             INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo) VALUES (
                 '999999999', 'Actor Test',
                 '$argon2id$v=19$m=19456,t=2,p=1$pO+/qvY8ieaUA97ME2LUPQ$OfE/070ufOj4TtL2SzVyW3sefnJjrMJq32APEHrM/wI',
                 'ROOT', 1
             );",
        )
        .unwrap();
    drop(conexion);

    let nucleo = Nucleo::abrir(ruta).unwrap();
    nucleo
        .autenticar_con_secreto(
            "999999999".to_string(),
            "clave_prueba_123".to_string(),
            String::new(),
        )
        .unwrap();
    let registro = nucleo
        .registrar_ingreso(1, MedioIngreso::Caminando, None, None)
        .unwrap();

    let activos = nucleo
        .listar_ingresos_activos(String::new(), ModoBusquedaActivos::NombreCedula)
        .unwrap();
    assert_eq!(activos.len(), 1);
    assert_eq!(activos[0].registro_id, registro.registro_id);
    assert_eq!(activos[0].contratista_nombre, "Contratista Test");

    nucleo.registrar_salida(registro.registro_id).unwrap();

    let activos_tras_salida = nucleo
        .listar_ingresos_activos(String::new(), ModoBusquedaActivos::NombreCedula)
        .unwrap();
    assert_eq!(activos_tras_salida, Vec::new());
}

/// Regresión directa del motivo por el que `Gafete` es un modo aparte:
/// una cédula que "contiene" el número de gafete no debe aparecer.
#[test]
fn listar_activos_por_gafete_es_exacto_sin_ruido_de_cedula() {
    let archivo = tempfile::NamedTempFile::new().unwrap();
    let ruta = archivo.path().to_str().unwrap().to_string();

    let conexion = control_acceso::database::connection::open_database(&ruta).unwrap();
    conexion
        .execute_batch(
            "INSERT INTO empresas (nombre) VALUES ('Empresa Test');
             INSERT INTO contratistas (
                 cedula, nombre, empresa_id, tipo_ingreso, es_personal_ruta, tiene_acceso,
                 fecha_vencimiento_praind
             ) VALUES
                 ('111111117', 'Con Gafete Siete', 1, 'PRAIND', 0, 1, '2099-12-31'),
                 ('222222222', 'Sin Gafete', 1, 'SWAT', 0, 1, NULL);
             INSERT INTO gafetes (numero, tipo, estado) VALUES (7, 'CONTRATISTA', 'DISPONIBLE');
             INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo) VALUES (
                 '999999999', 'Actor Test',
                 '$argon2id$v=19$m=19456,t=2,p=1$pO+/qvY8ieaUA97ME2LUPQ$OfE/070ufOj4TtL2SzVyW3sefnJjrMJq32APEHrM/wI',
                 'ROOT', 1
             );",
        )
        .unwrap();
    drop(conexion);

    let nucleo = Nucleo::abrir(ruta).unwrap();
    nucleo
        .autenticar_con_secreto(
            "999999999".to_string(),
            "clave_prueba_123".to_string(),
            String::new(),
        )
        .unwrap();
    nucleo
        .registrar_ingreso(1, MedioIngreso::Caminando, Some(7), None)
        .unwrap();
    // Cédula "222222222" no contiene un 7, así que si el modo Gafete
    // filtrara mal (o cayera al modo texto) esto no debería confundirse
    // con el otro contratista de todas formas — el segundo ingreso
    // (sin gafete) es el control negativo de esta prueba.
    nucleo
        .registrar_ingreso(2, MedioIngreso::Caminando, None, None)
        .unwrap();

    let por_gafete = nucleo
        .listar_ingresos_activos("7".to_string(), ModoBusquedaActivos::Gafete)
        .unwrap();
    assert_eq!(por_gafete.len(), 1);
    assert_eq!(por_gafete[0].contratista_nombre, "Con Gafete Siete");

    let texto_no_numerico = nucleo
        .listar_ingresos_activos("abc".to_string(), ModoBusquedaActivos::Gafete)
        .unwrap();
    assert_eq!(texto_no_numerico, Vec::new());
}

#[test]
fn listar_empresas_y_crear_contratista() {
    let archivo = tempfile::NamedTempFile::new().unwrap();
    let ruta = archivo.path().to_str().unwrap().to_string();

    let conexion = control_acceso::database::connection::open_database(&ruta).unwrap();
    conexion
        .execute_batch(
            "INSERT INTO empresas (nombre) VALUES ('Empresa Test');
             INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo) VALUES (
                 '999999999', 'Actor Test',
                 '$argon2id$v=19$m=19456,t=2,p=1$pO+/qvY8ieaUA97ME2LUPQ$OfE/070ufOj4TtL2SzVyW3sefnJjrMJq32APEHrM/wI',
                 'ROOT', 1
             );",
        )
        .unwrap();
    drop(conexion);

    let nucleo = Nucleo::abrir(ruta).unwrap();
    nucleo
        .autenticar_con_secreto(
            "999999999".to_string(),
            "clave_prueba_123".to_string(),
            String::new(),
        )
        .unwrap();

    let empresas = nucleo.listar_empresas().unwrap();
    assert_eq!(empresas.len(), 1);
    assert_eq!(empresas[0].nombre, "Empresa Test");

    let id = nucleo
        .crear_contratista(DatosContratista {
            cedula: "222222222".to_string(),
            nombre: "Nuevo Contratista".to_string(),
            empresa_id: empresas[0].id,
            tipo_ingreso: TipoIngreso::Swat,
            fecha_vencimiento_praind: None,
            es_personal_ruta: false,
        })
        .unwrap();
    assert!(id > 0);

    let resultados = nucleo.buscar_contratistas("Nuevo".to_string()).unwrap();
    assert_eq!(resultados.len(), 1);
    // El núcleo guarda el nombre en mayúsculas.
    assert_eq!(resultados[0].nombre, "NUEVO CONTRATISTA");
}

#[test]
fn crear_contratista_con_fecha_praind_invalida_falla() {
    let archivo = tempfile::NamedTempFile::new().unwrap();
    let ruta = archivo.path().to_str().unwrap().to_string();

    let conexion = control_acceso::database::connection::open_database(&ruta).unwrap();
    conexion
        .execute_batch(
            "INSERT INTO empresas (nombre) VALUES ('Empresa Test');
             INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo) VALUES (
                 '999999999', 'Actor Test',
                 '$argon2id$v=19$m=19456,t=2,p=1$pO+/qvY8ieaUA97ME2LUPQ$OfE/070ufOj4TtL2SzVyW3sefnJjrMJq32APEHrM/wI',
                 'ROOT', 1
             );",
        )
        .unwrap();
    drop(conexion);

    let nucleo = Nucleo::abrir(ruta).unwrap();
    nucleo
        .autenticar_con_secreto(
            "999999999".to_string(),
            "clave_prueba_123".to_string(),
            String::new(),
        )
        .unwrap();

    let resultado = nucleo.crear_contratista(DatosContratista {
        cedula: "333333333".to_string(),
        nombre: "Otro Contratista".to_string(),
        empresa_id: 1,
        tipo_ingreso: TipoIngreso::Praind,
        fecha_vencimiento_praind: Some("no-es-una-fecha".to_string()),
        es_personal_ruta: false,
    });

    assert!(matches!(resultado, Err(NucleoError::FechaInvalida { .. })));
}

/// Las reglas del alta en persona salen del núcleo con un mensaje listo
/// para mostrar (`Rechazado`), no como error interno.
#[test]
fn crear_contratista_aplica_reglas_del_alta() {
    let archivo = tempfile::NamedTempFile::new().unwrap();
    let ruta = archivo.path().to_str().unwrap().to_string();
    let conexion = control_acceso::database::connection::open_database(&ruta).unwrap();
    conexion
        .execute_batch(
            "INSERT INTO empresas (nombre) VALUES ('Empresa Test');
             INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo) VALUES (
                 '999999999', 'Actor Test',
                 '$argon2id$v=19$m=19456,t=2,p=1$pO+/qvY8ieaUA97ME2LUPQ$OfE/070ufOj4TtL2SzVyW3sefnJjrMJq32APEHrM/wI',
                 'ROOT', 1
             );",
        )
        .unwrap();
    drop(conexion);
    let nucleo = Nucleo::abrir(ruta).unwrap();
    nucleo
        .autenticar_con_secreto(
            "999999999".to_string(),
            "clave_prueba_123".to_string(),
            String::new(),
        )
        .unwrap();
    let datos = |tipo, fecha: &str, ruta| DatosContratista {
        cedula: "444444444".to_string(),
        nombre: "Contratista Regla".to_string(),
        empresa_id: 1,
        tipo_ingreso: tipo,
        fecha_vencimiento_praind: Some(fecha.to_string()),
        es_personal_ruta: ruta,
    };

    match nucleo.crear_contratista(datos(TipoIngreso::Praind, "2000-01-01", false)) {
        Err(NucleoError::Rechazado { mensaje }) => assert!(mensaje.contains("vencido")),
        otro => panic!("se esperaba PRAIND vencido, llegó {otro:?}"),
    }
    match nucleo.crear_contratista(datos(TipoIngreso::Swat, "2999-01-01", true)) {
        Err(NucleoError::Rechazado { mensaje }) => assert!(mensaje.contains("ruta")),
        otro => panic!("se esperaba personal de ruta no admitido, llegó {otro:?}"),
    }
    assert!(
        nucleo
            .crear_contratista(datos(TipoIngreso::InHouse, "2999-01-01", true))
            .is_ok()
    );

    assert!(nucleo.praind_vencido_para_formulario("2000-01-01".to_string()));
    assert!(!nucleo.praind_vencido_para_formulario("2999-01-01".to_string()));
    assert!(!nucleo.praind_vencido_para_formulario("2000-01".to_string()));
    assert!(nucleo.admite_personal_ruta_para_formulario(TipoIngreso::InHouse));
    assert!(!nucleo.admite_personal_ruta_para_formulario(TipoIngreso::PorCorreo));
}

#[test]
fn crear_empresa_y_cerrar_sesion() {
    let archivo = tempfile::NamedTempFile::new().unwrap();
    let ruta = archivo.path().to_str().unwrap().to_string();
    let conexion = control_acceso::database::connection::open_database(&ruta).unwrap();
    conexion
        .execute_batch(
            "INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo) VALUES (
                 '999999999', 'Actor Test',
                 '$argon2id$v=19$m=19456,t=2,p=1$pO+/qvY8ieaUA97ME2LUPQ$OfE/070ufOj4TtL2SzVyW3sefnJjrMJq32APEHrM/wI',
                 'ROOT', 1
             );",
        )
        .unwrap();
    drop(conexion);

    let nucleo = Nucleo::abrir(ruta).unwrap();
    nucleo
        .autenticar_con_secreto(
            "999999999".to_string(),
            "clave_prueba_123".to_string(),
            String::new(),
        )
        .unwrap();

    let id = nucleo.crear_empresa("Empresa Nueva".to_string()).unwrap();
    assert!(id > 0);

    nucleo.cerrar_sesion();

    let resultado = nucleo.crear_empresa("Otra Empresa".to_string());
    assert!(matches!(resultado, Err(NucleoError::NoAutenticado)));
}

/// Antes del aplanado de autorización (ver
/// docs/decisiones-tecnicas.md, 2026-09-11), un OPERADOR no podía
/// listar/crear usuarios -- hoy cualquier sesión válida puede,
/// `RolUsuario::puede()` ya devuelve `true` siempre en el núcleo
/// compartido. Este test quedó desactualizado (mobile nunca se tocó en
/// esa pasada, sólo TUI/desktop) y falló apenas se corrió después de
/// esta migración -- se actualiza acá para reflejar el comportamiento
/// real en vez de reintroducir una restricción que ya no existe.
#[test]
fn listar_usuarios_y_crear_usuario_no_depende_del_rol() {
    let archivo = tempfile::NamedTempFile::new().unwrap();
    let ruta = archivo.path().to_str().unwrap().to_string();
    let conexion = control_acceso::database::connection::open_database(&ruta).unwrap();
    conexion
        .execute_batch(
            "INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo) VALUES (
                 '999999999', 'Root Test',
                 '$argon2id$v=19$m=19456,t=2,p=1$pO+/qvY8ieaUA97ME2LUPQ$OfE/070ufOj4TtL2SzVyW3sefnJjrMJq32APEHrM/wI',
                 'ROOT', 1
             );
             INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo) VALUES (
                 '888888888', 'Operador Test',
                 '$argon2id$v=19$m=19456,t=2,p=1$pO+/qvY8ieaUA97ME2LUPQ$OfE/070ufOj4TtL2SzVyW3sefnJjrMJq32APEHrM/wI',
                 'OPERADOR', 1
             );",
        )
        .unwrap();
    drop(conexion);

    let nucleo = Nucleo::abrir(ruta).unwrap();
    nucleo
        .autenticar_con_secreto(
            "999999999".to_string(),
            "clave_prueba_123".to_string(),
            String::new(),
        )
        .unwrap();

    let id = nucleo
        .crear_usuario(DatosUsuario {
            cedula: "777777777".to_string(),
            nombre: "Nuevo Usuario".to_string(),
            password: "unaPassword123".to_string(),
            rol: RolUsuario::Operador,
            activo: true,
        })
        .unwrap();
    assert!(id > 0);

    let usuarios = nucleo.listar_usuarios(String::new()).unwrap();
    assert!(usuarios.iter().any(|u| u.cedula == "777777777"));

    nucleo.cerrar_sesion();
    nucleo
        .autenticar_con_secreto(
            "888888888".to_string(),
            "clave_prueba_123".to_string(),
            String::new(),
        )
        .unwrap();

    let usuarios_como_operador = nucleo.listar_usuarios(String::new()).unwrap();
    assert!(
        usuarios_como_operador
            .iter()
            .any(|u| u.cedula == "777777777")
    );

    let id_creado_por_operador = nucleo
        .crear_usuario(DatosUsuario {
            cedula: "666666666".to_string(),
            nombre: "Otro Usuario".to_string(),
            password: "unaPassword123".to_string(),
            rol: RolUsuario::Operador,
            activo: true,
        })
        .unwrap();
    assert!(id_creado_por_operador > 0);
}

#[test]
fn registrar_ingreso_sin_sesion_falla() {
    let archivo = tempfile::NamedTempFile::new().unwrap();
    let ruta = archivo.path().to_str().unwrap().to_string();
    let nucleo = Nucleo::abrir(ruta).unwrap();

    let resultado = nucleo.registrar_ingreso(1, MedioIngreso::Caminando, None, None);

    assert!(matches!(resultado, Err(NucleoError::NoAutenticado)));
}

fn nucleo_con_actor_y_ruta_79() -> Nucleo {
    let archivo = tempfile::NamedTempFile::new().unwrap();
    let ruta = archivo.path().to_str().unwrap().to_string();

    let conexion = control_acceso::database::connection::open_database(&ruta).unwrap();
    conexion
        .execute_batch(
            "INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo) VALUES (
                 '999999999', 'Actor Test',
                 '$argon2id$v=19$m=19456,t=2,p=1$pO+/qvY8ieaUA97ME2LUPQ$OfE/070ufOj4TtL2SzVyW3sefnJjrMJq32APEHrM/wI',
                 'ROOT', 1
             );
             INSERT INTO rutas (numero, activo, uuid) VALUES (79, 1, 'uuid-ruta-79');
             INSERT INTO encargados_ruta (codigo_empleado, nombre, activo, uuid) VALUES (
                 '5040017', 'Michael Araya Retana', 1, 'uuid-encargado'
             );",
        )
        .unwrap();
    drop(conexion);

    let nucleo = Nucleo::abrir(ruta).unwrap();
    nucleo
        .autenticar_con_secreto(
            "999999999".to_string(),
            "clave_prueba_123".to_string(),
            String::new(),
        )
        .unwrap();
    nucleo
}

fn solicitud_salida_ruta(placa: &str, numero_documento: &str) -> SolicitudSalidaRuta {
    SolicitudSalidaRuta {
        vehiculo_placa: placa.to_string(),
        vehiculo_numero_unidad: Some("22906".to_string()),
        encargado_nombre: "Michael Araya Retana".to_string(),
        encargado_codigo_empleado: Some("5040017".to_string()),
        numero_ruta: 79,
        sub_numero: 1,
        numero_documento: numero_documento.to_string(),
        // No usar `chrono::Utc::now()` para "hoy" acá -- la validación
        // real (`RutaService::registrar_salida`) compara contra
        // `fecha_costa_rica(fecha_hora_salida)` (UTC-6), no contra el
        // día calendario UTC. Entre 00:00 y 06:00 UTC ambos difieren en
        // un día, y este test fallaba exactamente en esa ventana
        // (`DocumentoRequiereAutorizacion` inesperado) -- no era un bug
        // de `interno()`/logging, es un desfase de huso horario en el
        // propio fixture.
        fecha_documento: control_acceso::tiempo::fecha_costa_rica(chrono::Utc::now())
            .format("%Y-%m-%d")
            .to_string(),
        tiene_correo_autorizacion: false,
    }
}

#[test]
fn buscar_rutas_encuentra_el_numero_del_catalogo() {
    let nucleo = nucleo_con_actor_y_ruta_79();

    let resultados = nucleo.buscar_rutas("79".to_string()).unwrap();

    assert_eq!(resultados.len(), 1);
    assert_eq!(resultados[0].numero, 79);
}

#[test]
fn buscar_encargados_ruta_encuentra_por_nombre_o_codigo() {
    let nucleo = nucleo_con_actor_y_ruta_79();

    assert_eq!(
        nucleo
            .buscar_encargados_ruta("araya".to_string())
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        nucleo
            .buscar_encargados_ruta("5040".to_string())
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn registrar_salida_y_retorno_de_ruta_redondea_el_viaje() {
    let nucleo = nucleo_con_actor_y_ruta_79();

    let resultado = nucleo
        .registrar_salida_ruta(solicitud_salida_ruta("C12345", "700101452"))
        .unwrap();
    assert_eq!(resultado.resultado, ResultadoSalidaRuta::Permitido);

    let activas = nucleo.listar_rutas_activas().unwrap();
    assert_eq!(activas.len(), 1);
    assert_eq!(activas[0].id, resultado.salida_id);
    assert_eq!(activas[0].numero_ruta, 79);

    nucleo.registrar_retorno_ruta(resultado.salida_id).unwrap();

    assert_eq!(nucleo.listar_rutas_activas().unwrap(), Vec::new());
}

#[test]
fn registrar_salida_ruta_con_numero_inexistente_falla() {
    let nucleo = nucleo_con_actor_y_ruta_79();
    let mut solicitud = solicitud_salida_ruta("C12345", "700101452");
    solicitud.numero_ruta = 222;

    let resultado = nucleo.registrar_salida_ruta(solicitud);

    assert!(matches!(resultado, Err(NucleoError::Interno { .. })));
}

#[test]
fn entregar_y_devolver_gafete_provisional_redondea_el_viaje() {
    let nucleo = nucleo_con_actor_y_ruta_79();
    let encargado = nucleo
        .buscar_encargados_ruta("5040017".to_string())
        .unwrap()
        .into_iter()
        .next()
        .unwrap();

    let prestamo_id = nucleo
        .entregar_gafete_provisional(encargado.id, 12)
        .unwrap();

    let activos = nucleo.listar_gafetes_provisionales_activos().unwrap();
    assert_eq!(activos.len(), 1);
    assert_eq!(activos[0].id, prestamo_id);
    assert_eq!(activos[0].encargado_codigo_empleado, "5040017");
    // Cubre la conversión `From<PrestamoGafeteProvisionalActivoResumenNucleo>`
    // -- el test del repositorio (núcleo) ya cubre que la columna se lea
    // bien de la base, pero no que el mapeo a la struct de UniFFI la
    // traiga consigo (un campo olvidado en ese `From` no lo detectaría
    // ningún otro test).
    assert_eq!(activos[0].usuario_entrega_nombre, "Actor Test");

    nucleo
        .registrar_devolucion_gafete_provisional(prestamo_id)
        .unwrap();

    assert_eq!(
        nucleo.listar_gafetes_provisionales_activos().unwrap(),
        Vec::new()
    );
}

#[test]
fn entregar_gafete_provisional_a_encargado_inexistente_falla() {
    let nucleo = nucleo_con_actor_y_ruta_79();

    let resultado = nucleo.entregar_gafete_provisional(999, 12);

    assert!(matches!(resultado, Err(NucleoError::Rechazado { .. })));
}

/// Secreto vacío: sin chequeo de nube; las reglas locales siguen
/// aplicando en la misma llamada (el mismo gafete no se presta dos veces).
#[test]
fn entregar_gafete_provisional_con_secreto_aplica_las_reglas_locales() {
    let nucleo = nucleo_con_actor_y_ruta_79();
    let encargado_id = nucleo
        .buscar_encargados_ruta("5040017".to_string())
        .unwrap()[0]
        .id;

    nucleo
        .entregar_gafete_provisional_con_secreto(encargado_id, 12, String::new())
        .unwrap();
    assert!(matches!(
        nucleo.entregar_gafete_provisional_con_secreto(encargado_id, 12, String::new()),
        Err(NucleoError::Rechazado { .. })
    ));
}

fn nucleo_con_actor_empresa_proveedora_y_gafete() -> Nucleo {
    let archivo = tempfile::NamedTempFile::new().unwrap();
    let ruta = archivo.path().to_str().unwrap().to_string();

    let conexion = control_acceso::database::connection::open_database(&ruta).unwrap();
    conexion
        .execute_batch(
            "INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo) VALUES (
                 '999999999', 'Actor Test',
                 '$argon2id$v=19$m=19456,t=2,p=1$pO+/qvY8ieaUA97ME2LUPQ$OfE/070ufOj4TtL2SzVyW3sefnJjrMJq32APEHrM/wI',
                 'ROOT', 1
             );
             INSERT INTO empresas_proveedor (nombre, activo, uuid) VALUES ('Maika', 1, 'uuid-empresa-proveedor');
             INSERT INTO gafetes (numero, tipo, estado) VALUES (7, 'PROVEEDOR', 'DISPONIBLE');",
        )
        .unwrap();
    drop(conexion);

    let nucleo = Nucleo::abrir(ruta).unwrap();
    nucleo
        .autenticar_con_secreto(
            "999999999".to_string(),
            "clave_prueba_123".to_string(),
            String::new(),
        )
        .unwrap();
    nucleo
}

#[test]
fn buscar_empresas_proveedor_encuentra_por_nombre() {
    let nucleo = nucleo_con_actor_empresa_proveedora_y_gafete();

    let resultados = nucleo
        .buscar_empresas_proveedor("maika".to_string())
        .unwrap();

    assert_eq!(resultados.len(), 1);
    assert_eq!(resultados[0].nombre, "Maika");
}

#[test]
fn crear_empresa_proveedor_y_buscarla_redondea_el_viaje() {
    let nucleo = nucleo_con_actor_empresa_proveedora_y_gafete();

    nucleo
        .crear_empresa_proveedor("Dos Pinos".to_string())
        .unwrap();

    let resultados = nucleo
        .buscar_empresas_proveedor("dos pinos".to_string())
        .unwrap();
    assert_eq!(resultados.len(), 1);
}

#[test]
fn registrar_ingreso_y_salida_de_proveedor_redondea_el_viaje() {
    let nucleo = nucleo_con_actor_empresa_proveedora_y_gafete();
    let empresa = nucleo
        .buscar_empresas_proveedor("maika".to_string())
        .unwrap()
        .into_iter()
        .next()
        .unwrap();

    let registro_id = nucleo
        .registrar_ingreso_proveedor(
            "1-1111".to_string(),
            "Juan Perez".to_string(),
            empresa.id,
            None,
            7,
        )
        .unwrap();

    let activos = nucleo.listar_proveedores_activos().unwrap();
    assert_eq!(activos.len(), 1);
    assert_eq!(activos[0].id, registro_id);
    assert_eq!(activos[0].gafete_numero, 7);

    nucleo.registrar_salida_proveedor(registro_id).unwrap();

    assert_eq!(nucleo.listar_proveedores_activos().unwrap(), Vec::new());
}

#[test]
fn registrar_ingreso_proveedor_con_empresa_inexistente_falla() {
    let nucleo = nucleo_con_actor_empresa_proveedora_y_gafete();

    let resultado = nucleo.registrar_ingreso_proveedor(
        "1-1111".to_string(),
        "Juan Perez".to_string(),
        999,
        None,
        7,
    );

    assert!(matches!(
        resultado,
        Err(NucleoError::Rechazado { mensaje }) if mensaje == "Empresa proveedora no encontrada"
    ));
}

/// La llamada combinada aplica la regla de "un ingreso abierto por
/// cédula" antes de escribir (secreto vacío: sin chequeos de nube).
#[test]
fn registrar_ingreso_proveedor_con_secreto_rechaza_cedula_ya_activa() {
    let nucleo = nucleo_con_actor_empresa_proveedora_y_gafete();
    let empresa_id = nucleo
        .buscar_empresas_proveedor("maika".to_string())
        .unwrap()[0]
        .id;
    let registrar = || {
        nucleo.registrar_ingreso_proveedor_con_secreto(
            "1-1111".to_string(),
            "Juan Perez".to_string(),
            empresa_id,
            None,
            7,
            String::new(),
        )
    };

    assert!(
        nucleo
            .aviso_proveedor_con_ingreso_activo("1-1111".to_string())
            .unwrap()
            .is_none()
    );
    registrar().unwrap();
    assert!(matches!(
        registrar(),
        Err(NucleoError::Rechazado { mensaje })
            if mensaje == "Esta cédula ya tiene un ingreso de proveedor activo"
    ));
    assert!(
        nucleo
            .aviso_proveedor_con_ingreso_activo("1-1111".to_string())
            .unwrap()
            .is_some()
    );
}
