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
