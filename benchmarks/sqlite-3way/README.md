# Brisas — benchmark de motores SQLite

Rama de evaluación: `bench/sqlite-3way-2026-09-12`. Este directorio SÍ vive
en git (a diferencia de `experiments/`, que está en `.gitignore` a
propósito -- ver ese archivo -- por eso este trabajo se mudó acá).

## Criterio de aprobación (smoke test de cada motor)

1. crea una base nueva con cifrado activado;
2. la cabecera del archivo no contiene `SQLite format 3\0` en claro;
3. la base se reabre y devuelve el dato esperado con la clave correcta;
4. una clave incorrecta no permite leer `sqlite_master`.

## Motores evaluados

- **`sqlcipher/`**: `rusqlite 0.40.2` + `bundled-sqlcipher-vendored-openssl`.
  Compila SQLCipher y OpenSSL desde las fuentes vendorizadas del grafo
  Cargo -- build "en frío" lento, depende de un Perl completo (con todos
  sus módulos; el Perl que trae Git para Windows no alcanza).

- **`sqlite3mc/` + `sqlite3mc-vendor-lib/`**: SQLite3 Multiple Ciphers
  2.5.1 (ChaCha20-Poly1305), compilado **100% estático desde la fuente
  vendorizada** (`sqlite3mc-vendor-lib/vendor/`, procedencia y SHA-256 en
  su propio `README-fuente.md`) -- nada de DLL/import library en runtime,
  verificado con `objdump -p` sobre el `.exe` final (sólo aparecen DLLs
  del sistema/CRT de Windows, ninguna de SQLite). Reemplaza al primer
  intento de este laboratorio, que enlazaba contra el ZIP win64 oficial
  (DLL + import lib) -- eso funcionaba pero exigía distribuir la DLL junto
  al ejecutable, descartado a propósito.

  **Por qué son dos crates y no uno**: `libsqlite3-sys` resuelve su propio
  `-lstatic=sqlite3` en el momento en que SE COMPILA A SÍ MISMO -- eso
  pasa ANTES de que el build script de cualquier crate río abajo (que
  depende de `rusqlite`) llegue a ejecutarse. Un `build.rs` en el mismo
  crate que consume `rusqlite` no alcanza a proveer la ruta de búsqueda a
  tiempo. Por eso `sqlite3mc-vendor-lib` compila la librería estática
  aparte, en una ruta fija (`dist/`, no el `OUT_DIR` con hash de Cargo), y
  `sqlite3mc/.cargo/config.toml` le dice a `libsqlite3-sys` dónde
  encontrarla vía `SQLITE3_LIB_DIR`/`SQLITE3_STATIC` -- son las mismas dos
  variables que ya lee ese build script, sólo que ahora apuntan a algo que
  compilamos nosotros en vez de a un ZIP descargado.

  Para correrlo:
  ```sh
  cargo build --release --manifest-path sqlite3mc-vendor-lib/Cargo.toml
  cargo run --release --manifest-path sqlite3mc/Cargo.toml
  ```

- **SQLite normal (baseline)**: pendiente -- falta un tercer crate sin
  ningún cifrado para la comparación de rendimiento completa (ver
  `docs/auditorias/AUDITORIA_RENDIMIENTO_CORE_RUST_2026-09-10.md`,
  sección 12).

Esta carpeta no cambia `Cargo.toml`, `AppCore`, el esquema ni los datos de
producción del proyecto raíz. Que estos smoke tests pasen prueba
compatibilidad básica de compilación/enlace/cifrado; no prueba todavía
migración, DPAPI, rendimiento ni integración completa con Brisas (eso es
el benchmark comparativo real de 5 rondas / 1.000 ciclos que describe la
sección 12 de esa misma auditoría, todavía pendiente de armar acá).
