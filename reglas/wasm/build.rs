//! Calcula la "huella" de las fuentes de las reglas (FNV-1a de 64 bits sobre
//! los `.rs` y `Cargo.toml` de `reglas/` y `reglas/wasm/`, en orden fijo y
//! sin `\r`) y la deja en `huella.rs` para `huella_fuentes()`. Los tests del
//! panel calculan la misma huella sobre las fuentes del repo y la comparan
//! con la del paquete commiteado: si alguien cambia una regla y no regenera
//! el paquete, el test falla (`web/src/reglas/reglas.test.ts`).

use std::path::{Path, PathBuf};

fn archivos(raiz: &Path) -> Vec<PathBuf> {
    let mut lista = vec![raiz.join("Cargo.toml")];
    let mut fuentes: Vec<PathBuf> = std::fs::read_dir(raiz.join("src"))
        .expect("src/")
        .map(|entrada| entrada.expect("entrada").path())
        .filter(|ruta| ruta.extension().is_some_and(|ext| ext == "rs"))
        .collect();
    fuentes.sort();
    lista.extend(fuentes);
    lista
}

fn main() {
    let wasm = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let reglas = wasm.parent().expect("reglas/").to_path_buf();
    let mut lista = archivos(&reglas);
    lista.extend(archivos(&wasm));

    let mut huella: u64 = 0xcbf2_9ce4_8422_2325;
    for ruta in &lista {
        println!("cargo:rerun-if-changed={}", ruta.display());
        let nombre = ruta
            .strip_prefix(&reglas)
            .expect("dentro de reglas/")
            .to_string_lossy()
            .replace('\\', "/");
        let contenido = std::fs::read(ruta).expect("leer fuente");
        for byte in nombre
            .bytes()
            .chain([0])
            .chain(contenido.into_iter().filter(|b| *b != b'\r'))
        {
            huella ^= u64::from(byte);
            huella = huella.wrapping_mul(0x0100_0000_01b3);
        }
    }
    println!("cargo:rerun-if-changed=build.rs");
    let destino = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR")).join("huella.rs");
    std::fs::write(
        destino,
        format!("pub const HUELLA: &str = \"{huella:016x}\";\n"),
    )
    .expect("escribir huella");
}
