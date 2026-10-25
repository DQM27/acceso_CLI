//! Mete el manual de uso de la app (`docs/manual/movil/*.md` y el README con
//! el índice) dentro del `.so`, para la Ayuda sin internet (`src/manual/`).
//! Genera `$OUT_DIR/manual_movil.rs` con un `include_str!` por archivo: así
//! un proceso nuevo entra solo, sin tocar código.

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let manifiesto = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR lo pone cargo");
    let manual = Path::new(&manifiesto).join("../../docs/manual");
    let movil = manual.join("movil");

    // La carpeta, para enterarse de archivos nuevos o borrados; cada archivo,
    // para los cambios de contenido.
    println!("cargo:rerun-if-changed={}", movil.display());
    let mut archivos: Vec<(String, PathBuf)> =
        vec![("README.md".to_string(), manual.join("README.md"))];
    let mut procesos: Vec<PathBuf> = fs::read_dir(&movil)
        .unwrap_or_else(|error| panic!("no se pudo leer {}: {error}", movil.display()))
        .filter_map(Result::ok)
        .map(|entrada| entrada.path())
        .filter(|ruta| ruta.extension().is_some_and(|ext| ext == "md"))
        .collect();
    procesos.sort();
    for ruta in procesos {
        let nombre = ruta
            .file_name()
            .and_then(|n| n.to_str())
            .expect("nombre de archivo en UTF-8")
            .to_string();
        archivos.push((format!("movil/{nombre}"), ruta));
    }

    let mut codigo = String::from(
        "/// (ruta dentro de `docs/manual/`, contenido)\nconst ARCHIVOS: &[(&str, &str)] = &[\n",
    );
    for (ruta, archivo) in &archivos {
        println!("cargo:rerun-if-changed={}", archivo.display());
        // `CARGO_MANIFEST_DIR` es absoluta, así que esta ruta también.
        writeln!(
            codigo,
            "    ({ruta:?}, include_str!({:?})),",
            archivo.display().to_string()
        )
        .expect("escribir en un String no falla");
    }
    codigo.push_str("];\n");

    let salida =
        Path::new(&env::var("OUT_DIR").expect("OUT_DIR lo pone cargo")).join("manual_movil.rs");
    fs::write(&salida, codigo)
        .unwrap_or_else(|error| panic!("no se pudo escribir {}: {error}", salida.display()));
}
