//! Vigila la regla de dependencias del núcleo: las reglas de negocio puras
//! (`src/domain`, los tipos que usan en `src/models` y el crate `reglas/`)
//! no pueden tocar la base de datos, la red, el reloj del sistema ni las
//! capas de afuera (servicios, casos de uso, nube). Si alguien mezcla una
//! regla con `SQLite` o con una llamada HTTP, esta prueba falla y dice en
//! qué archivo y en qué línea.
//!
//! Dentro de un mismo crate el compilador no impide que `domain` importe
//! `database`; por eso se revisa el código fuente. Sólo cuenta el código de
//! producción: lo que va después de `#[cfg(test)]` (los módulos de prueba
//! al final de cada archivo) y los comentarios no se miran.

use std::fs;
use std::path::{Path, PathBuf};

/// Lo que una regla pura nunca puede usar: la base, la red y las capas que
/// orquestan (que son las que sí pueden depender del dominio, no al revés).
const PROHIBIDO_EN_REGLAS: [&str; 8] = [
    "rusqlite",
    "reqwest",
    "crate::database",
    "crate::services",
    "crate::application",
    "crate::nube",
    "crate::historial",
    "crate::lenguaje_comandos",
];

/// El dominio recibe la fecha y la hora como parámetro; leer el reloj es
/// trabajo de quien lo llama (ver `crate::tiempo::Reloj`).
const RELOJ_DEL_SISTEMA: [&str; 4] = ["Utc::now", "Local::now", "SystemTime::now", "crate::tiempo"];

/// Dependencias permitidas del crate `reglas/`, que además compila a
/// WebAssembly para el panel web y las Edge Functions.
const DEPENDENCIAS_DE_REGLAS: [&str; 3] = ["chrono", "serde", "thiserror"];

fn raiz() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// Todos los `.rs` bajo `directorio`, recorriendo subcarpetas.
fn archivos_rust(directorio: &Path) -> Vec<PathBuf> {
    let mut archivos = Vec::new();
    let entradas = fs::read_dir(directorio)
        .unwrap_or_else(|error| panic!("no se pudo leer {}: {error}", directorio.display()));
    for entrada in entradas {
        let ruta = entrada.expect("entrada de directorio legible").path();
        if ruta.is_dir() {
            archivos.extend(archivos_rust(&ruta));
        } else if ruta.extension().is_some_and(|extension| extension == "rs") {
            archivos.push(ruta);
        }
    }
    archivos.sort();
    archivos
}

/// Las líneas de código de producción de `archivo` (número y texto): corta
/// en el primer `#[cfg(test)]` y descarta los comentarios.
fn lineas_de_produccion(archivo: &Path) -> Vec<(usize, String)> {
    let contenido = fs::read_to_string(archivo)
        .unwrap_or_else(|error| panic!("no se pudo leer {}: {error}", archivo.display()));
    contenido
        .lines()
        .enumerate()
        .take_while(|(_, linea)| linea.trim() != "#[cfg(test)]")
        .filter(|(_, linea)| !linea.trim_start().starts_with("//"))
        .map(|(indice, linea)| (indice + 1, linea.to_owned()))
        .collect()
}

/// Cada uso de algo de `prohibido` en los archivos de `carpetas`, como
/// `ruta:línea: uso`.
fn violaciones(carpetas: &[&str], prohibido: &[&str]) -> Vec<String> {
    carpetas
        .iter()
        .flat_map(|carpeta| archivos_rust(&raiz().join(carpeta)))
        .flat_map(|archivo| {
            let relativa = archivo
                .strip_prefix(raiz())
                .unwrap_or(&archivo)
                .display()
                .to_string();
            lineas_de_produccion(&archivo)
                .into_iter()
                .filter_map(move |(numero, linea)| {
                    prohibido
                        .iter()
                        .find(|uso| linea.contains(*uso))
                        .map(|uso| format!("{relativa}:{numero}: usa `{uso}`"))
                })
        })
        .collect()
}

/// La revisión no pasa en vacío: en la capa de base de datos, donde
/// `rusqlite` sí corresponde, tiene que encontrarlo.
#[test]
fn la_revision_encuentra_un_uso_real() {
    assert!(
        !violaciones(&["src/database"], &["rusqlite"]).is_empty(),
        "la revisión debería encontrar `rusqlite` en src/database; si no, no está leyendo nada"
    );
}

#[test]
fn las_reglas_no_tocan_la_base_la_red_ni_las_capas_de_afuera() {
    let encontradas = violaciones(
        &["src/domain", "src/models", "reglas/src"],
        &PROHIBIDO_EN_REGLAS,
    );
    assert!(
        encontradas.is_empty(),
        "una regla pura no puede depender de la base, la red ni de las capas que la usan; \
         el caso de uso trae los datos y el dominio decide:\n{}",
        encontradas.join("\n")
    );
}

#[test]
fn el_dominio_no_lee_el_reloj_del_sistema() {
    let encontradas = violaciones(&["src/domain", "reglas/src"], &RELOJ_DEL_SISTEMA);
    assert!(
        encontradas.is_empty(),
        "el dominio recibe la fecha y la hora como parámetro, no lee el reloj:\n{}",
        encontradas.join("\n")
    );
}

#[test]
fn el_crate_reglas_solo_depende_de_crates_puras() {
    let manifiesto = fs::read_to_string(raiz().join("reglas/Cargo.toml"))
        .expect("reglas/Cargo.toml se puede leer");
    let dependencias = manifiesto
        .split("[dependencies]")
        .nth(1)
        .expect("reglas/Cargo.toml tiene [dependencies]")
        .split("\n[")
        .next()
        .unwrap_or_default();
    for linea in dependencias.lines().map(str::trim) {
        if linea.is_empty() || linea.starts_with('#') {
            continue;
        }
        let nombre = linea.split(['.', '=', ' ']).next().unwrap_or_default();
        assert!(
            DEPENDENCIAS_DE_REGLAS.contains(&nombre),
            "`reglas` no puede depender de `{nombre}`: sólo de {DEPENDENCIAS_DE_REGLAS:?} \
             (compila a WebAssembly y tiene que seguir siendo pura)"
        );
    }
}

// ---- Puertos y adaptadores (arquitectura hexagonal) ----

/// Lo que un puerto no puede nombrar: él lo define la aplicación y lo
/// implementa el adaptador, nunca al revés.
const PROHIBIDO_EN_PUERTOS: [&str; 3] = [
    "rusqlite",
    "crate::database::repositories",
    "crate::database::queries",
];

#[test]
fn los_puertos_no_conocen_a_sus_adaptadores() {
    let carpeta = raiz().join("src/application/puertos");
    assert!(
        archivos_rust(&carpeta).len() >= 4,
        "la revisión no pasa en vacío: tiene que haber puertos que revisar"
    );
    let encontradas = violaciones(&["src/application/puertos"], &PROHIBIDO_EN_PUERTOS);
    assert!(
        encontradas.is_empty(),
        "un puerto lo define la aplicación y lo implementa el adaptador (`SQLite`); si nombra \
         `rusqlite` o los repositorios, la dependencia va al revés:\n{}",
        encontradas.join("\n")
    );
}

/// Los archivos de `carpeta` (relativos a la raíz, con `/`) que usan
/// `rusqlite` en código de producción.
fn archivos_con_rusqlite(carpeta: &str) -> Vec<String> {
    let mut archivos: Vec<String> = violaciones(&[carpeta], &["rusqlite"])
        .into_iter()
        .filter_map(|uso| uso.split(':').next().map(|ruta| ruta.replace('\\', "/")))
        .collect();
    archivos.sort();
    archivos.dedup();
    archivos
}

/// Trinquete: los archivos de la capa de aplicación que hoy todavía abren la
/// transacción o consultan con `rusqlite` directamente. **La lista sólo se
/// encoge.** Un archivo nuevo no puede usar `rusqlite` (la conexión se pide a
/// la unidad de trabajo o a un puerto); y cuando uno de estos deja de usarlo,
/// hay que sacarlo de la lista, para que no vuelva a hacerlo sin que nadie lo
/// note.
const APLICACION_CON_RUSQLITE: [&str; 12] = [
    "src/application/accesos.rs",
    "src/application/catalogos.rs",
    "src/application/citas.rs",
    "src/application/gafetes.rs",
    "src/application/gafetes_provisionales.rs",
    "src/application/historial.rs",
    "src/application/mod.rs",
    "src/application/nube.rs",
    "src/application/proveedores.rs",
    "src/application/rutas.rs",
    "src/application/tenedores_gafete.rs",
    "src/application/unidad_de_trabajo.rs",
];

/// Lo mismo para los servicios, que ya trabajan contra traits.
const SERVICIOS_CON_RUSQLITE: [&str; 1] = ["src/services/registro_ingreso_service.rs"];

fn verificar_trinquete(carpeta: &str, permitidos: &[&str]) {
    let actuales = archivos_con_rusqlite(carpeta);
    let nuevos: Vec<&String> = actuales
        .iter()
        .filter(|archivo| !permitidos.contains(&archivo.as_str()))
        .collect();
    assert!(
        nuevos.is_empty(),
        "estos archivos de `{carpeta}` usan `rusqlite` y no deberían: pídanle la conexión a la \
         unidad de trabajo (`AppCore::en_unidad_de_trabajo`) o definan un puerto en \
         `application/puertos`:\n{nuevos:#?}"
    );
    let ya_limpios: Vec<&&str> = permitidos
        .iter()
        .filter(|archivo| !actuales.iter().any(|actual| actual == **archivo))
        .collect();
    assert!(
        ya_limpios.is_empty(),
        "estos archivos de `{carpeta}` ya no usan `rusqlite`: sáquenlos de la lista de \
         permitidos para que no vuelvan a hacerlo:\n{ya_limpios:#?}"
    );
}

#[test]
fn la_aplicacion_no_suma_archivos_que_usen_rusqlite() {
    verificar_trinquete("src/application", &APLICACION_CON_RUSQLITE);
}

#[test]
fn los_servicios_no_suman_archivos_que_usen_rusqlite() {
    verificar_trinquete("src/services", &SERVICIOS_CON_RUSQLITE);
}

#[test]
fn los_servicios_piden_los_puertos_a_la_aplicacion_no_al_adaptador() {
    // Los tres traits ya mudados: se importan de `application::puertos`, no del
    // archivo del adaptador `SQLite` (que los reexporta sólo por compatibilidad).
    const TRAITS_MUDADOS: [&str; 3] = [
        "contratista_repository::ContratistaRepository",
        "gafete_repository::GafeteRepository",
        "registro_ingreso_repository::RegistroIngresoRepository",
    ];
    let encontradas = violaciones(&["src/services", "src/application"], &TRAITS_MUDADOS);
    assert!(
        encontradas.is_empty(),
        "estos traits son puertos de la aplicación: impórtenlos de \
         `crate::application::puertos`:\n{}",
        encontradas.join("\n")
    );
}
