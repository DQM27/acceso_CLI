//! Benchmark comparativo de los tres motores SQLite contra `AppCore` real --
//! ver `docs/auditorias/auditoria-rendimiento-core-rust-2026-09-10.md`,
//! sección 12, y `docs/decisiones-tecnicas.md` (entradas sobre el switch de
//! tres vías y el enlace real de `cifrado-sqlite3mc`).
//!
//! Las tres features son mutuamente excluyentes y se deciden en tiempo de
//! compilación (ver `compile_error!` en `src/lib.rs`) -- este binario no
//! puede comparar los tres motores en una sola corrida. Se compila y corre
//! UNA VEZ POR MOTOR, y los tres bloques de resultados impresos se pegan a
//! mano en la sección 12 de la auditoría (ese doc es el destino final de los
//! resultados, no un archivo aparte):
//!
//! ```text
//! cargo run --release --example benchmark_3way --no-default-features --features nube,sqlite-plano
//!
//! cargo build --release --manifest-path sqlite3mc-vendor-lib/Cargo.toml
//! cargo run --release --example benchmark_3way --no-default-features --features nube,cifrado-sqlite3mc
//!
//! cargo run --release --example benchmark_3way --features nube   (cifrado-sqlcipher, el default)
//! ```
//!
//! Carga: 5 rondas de 1.000 ciclos completos de ingreso + salida cada una,
//! mismo `AppCore`, mismo contratista (caso feliz sin gafete/PRAIND -- el
//! benchmark mide el motor de base de datos, no las reglas de negocio).
//! El tiempo de compilación queda fuera de la medición.

use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use control_acceso::application::AppCore;
use control_acceso::database::connection::{open_database, open_database_cifrada};
use control_acceso::models::medio_ingreso::MedioIngreso;
use control_acceso::tiempo::RelojSistema;

const RONDAS: usize = 5;
const CICLOS_POR_RONDA: usize = 1_000;
// Clave fija de laboratorio -- este binario nunca toca datos reales, no hay
// nada que proteger acá; sólo ejercita el mismo camino de cifrado que usa
// producción (`aplicar_clave`/`PRAGMA key`, y `PRAGMA cipher` bajo
// cifrado-sqlite3mc).
const CLAVE_LABORATORIO: [u8; 32] = [0x42; 32];
const CEDULA_ACTOR: &str = "999999999";
// Hash Argon2id de "clave_prueba_123" -- mismo fixture que ya usan los tests
// de integración de mobile/rust-core, reutilizado acá para no depender de
// argon2 en tiempo de ejecución sólo para sembrar el benchmark.
const HASH_ACTOR: &str = "$argon2id$v=19$m=19456,t=2,p=1$pO+/qvY8ieaUA97ME2LUPQ$OfE/070ufOj4TtL2SzVyW3sefnJjrMJq32APEHrM/wI";

fn nombre_motor() -> &'static str {
    if cfg!(feature = "sqlite-plano") {
        "sqlite-plano (baseline, sin cifrar)"
    } else if cfg!(feature = "cifrado-sqlite3mc") {
        "cifrado-sqlite3mc (ChaCha20-Poly1305)"
    } else {
        "cifrado-sqlcipher (motor real de producción)"
    }
}

/// `Get-Process` en vez de una API nativa: cero dependencias nuevas para un
/// binario que no se distribuye, sólo se corre a mano en el runner de
/// benchmark (siempre Windows, ver el encabezado de este archivo).
fn working_set_mb() -> Option<f64> {
    let pid = std::process::id();
    let salida = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            &format!("(Get-Process -Id {pid}).WorkingSet64"),
        ])
        .output()
        .ok()?;
    if !salida.status.success() {
        return None;
    }
    let bytes: u64 = String::from_utf8_lossy(&salida.stdout)
        .trim()
        .parse()
        .ok()?;
    #[allow(clippy::cast_precision_loss)]
    Some(bytes as f64 / (1024.0 * 1024.0))
}

fn abrir_core(ruta: &Path) -> AppCore {
    if cfg!(feature = "sqlite-plano") {
        AppCore::abrir(ruta).expect("abrir AppCore (sqlite-plano)")
    } else {
        AppCore::abrir_con_reloj_cifrado(ruta, &CLAVE_LABORATORIO, Arc::new(RelojSistema))
            .expect("abrir AppCore cifrado")
    }
}

fn sembrar(ruta: &Path) {
    let connection = if cfg!(feature = "sqlite-plano") {
        open_database(ruta).expect("abrir/migrar la base de benchmark")
    } else {
        open_database_cifrada(ruta, &CLAVE_LABORATORIO).expect("abrir/migrar la base de benchmark")
    };
    connection
        .execute_batch(&format!(
            "INSERT INTO empresas (nombre) VALUES ('Empresa Benchmark');
             INSERT INTO contratistas (
                 cedula, nombre, empresa_id, tipo_ingreso, es_personal_ruta, tiene_acceso
             ) VALUES ('000000000', 'Contratista Benchmark', 1, 'SWAT', 0, 1);
             INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo) VALUES (
                 '{CEDULA_ACTOR}', 'Actor Benchmark', '{HASH_ACTOR}', 'ROOT', 1
             );"
        ))
        .expect("sembrar datos mínimos");
}

fn mediana(valores: &mut [f64]) -> f64 {
    valores.sort_by(|a, b| a.total_cmp(b));
    let mitad = valores.len() / 2;
    if valores.len() % 2 == 0 {
        (valores[mitad - 1] + valores[mitad]) / 2.0
    } else {
        valores[mitad]
    }
}

fn main() {
    println!(
        "=== Benchmark 3 vías -- motor activo: {} ===",
        nombre_motor()
    );
    println!("Rondas: {RONDAS}, ciclos por ronda: {CICLOS_POR_RONDA}\n");

    let directorio = tempfile::tempdir().expect("crear directorio temporal");
    let ruta = directorio.path().join("benchmark.db");
    sembrar(&ruta);

    let core = abrir_core(&ruta);
    let actor = core
        .autenticar(CEDULA_ACTOR, "clave_prueba_123")
        .expect("autenticar actor de benchmark");

    let memoria_inicial = working_set_mb();
    let mut memoria_pico = memoria_inicial.unwrap_or(0.0);
    let mut duraciones_ronda_ms = Vec::with_capacity(RONDAS);

    for ronda in 1..=RONDAS {
        let inicio = Instant::now();
        for _ in 0..CICLOS_POR_RONDA {
            let preparacion = core
                .preparar_ingreso(1)
                .expect("preparar_ingreso no debería fallar en el caso feliz sembrado");
            assert!(
                !preparacion.requiere_gafete,
                "el contratista sembrado no debería pedir gafete"
            );

            let resultado = core
                .registrar_ingreso(&actor, 1, MedioIngreso::Caminando, None)
                .expect("registrar_ingreso no debería fallar en el caso feliz sembrado");

            core.registrar_salida(&actor, resultado.registro_id)
                .expect("registrar_salida no debería fallar en el caso feliz sembrado");
        }
        let duracion_ms = inicio.elapsed().as_secs_f64() * 1000.0;
        duraciones_ronda_ms.push(duracion_ms);

        if let Some(mb) = working_set_mb() {
            memoria_pico = memoria_pico.max(mb);
        }

        println!(
            "  ronda {ronda}/{RONDAS}: {duracion_ms:.1} ms total, {:.4} ms/ciclo",
            duracion_ms / CICLOS_POR_RONDA as f64
        );
    }

    let memoria_final = working_set_mb();

    let minimo = duraciones_ronda_ms
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min);
    let maximo = duraciones_ronda_ms
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let promedio = duraciones_ronda_ms.iter().sum::<f64>() / duraciones_ronda_ms.len() as f64;
    let mediana_valor = mediana(&mut duraciones_ronda_ms.clone());

    println!(
        "\n--- Resumen ({} -- pegar en la sección 12 de la auditoría) ---",
        nombre_motor()
    );
    println!("Tiempo por ronda (1.000 ciclos ingreso+salida):");
    println!("  mediana:  {mediana_valor:.1} ms");
    println!("  promedio: {promedio:.1} ms");
    println!("  mínimo:   {minimo:.1} ms");
    println!("  máximo:   {maximo:.1} ms");
    println!(
        "  por ciclo (sobre la mediana): {:.4} ms",
        mediana_valor / CICLOS_POR_RONDA as f64
    );
    println!("Memoria del proceso (Working Set):");
    println!(
        "  inicial: {}",
        memoria_inicial.map_or_else(|| "n/d".to_string(), |v| format!("{v:.1} MB"))
    );
    println!("  pico durante las rondas: {memoria_pico:.1} MB");
    println!(
        "  final:   {}",
        memoria_final.map_or_else(|| "n/d".to_string(), |v| format!("{v:.1} MB"))
    );
}
