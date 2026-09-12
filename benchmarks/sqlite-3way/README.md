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

- **`sqlite3mc/`**: smoke test aislado de SQLite3 Multiple Ciphers 2.5.1
  (ChaCha20-Poly1305), compilado **100% estático desde la fuente
  vendorizada** (`../../sqlite3mc-vendor-lib/vendor/`, procedencia y
  SHA-256 en su propio `README-fuente.md`) -- nada de DLL/import library
  en runtime, verificado con `objdump -p` sobre el `.exe` final (sólo
  aparecen DLLs del sistema/CRT de Windows, ninguna de SQLite). Reemplaza
  al primer intento de este laboratorio, que enlazaba contra el ZIP win64
  oficial (DLL + import lib) -- eso funcionaba pero exigía distribuir la
  DLL junto al ejecutable, descartado a propósito.

  **`sqlite3mc-vendor-lib/` ya no vive acá** -- se movió a la raíz del
  repo (`/sqlite3mc-vendor-lib/`) porque dejó de ser sólo un experimento
  de laboratorio: es la misma librería estática que ahora usa el motor
  `cifrado-sqlite3mc` del crate raíz `control_acceso` (y, por lo tanto,
  `desktop/src-tauri`/`mobile/rust-core`) -- un solo amalgamation
  vendorizado de 14MB, no uno duplicado por consumidor. Ver
  `docs/decisiones-tecnicas.md`, entrada 2026-09-12, para el porqué de las
  dos fases de compilación (`libsqlite3-sys` resuelve su propio
  `-lstatic=sqlite3` al compilarse a sí mismo, antes de que corra el build
  script de cualquier crate río abajo -- por eso el amalgamation se
  compila aparte, en una ruta fija, ANTES del build que lo consume).

  Para correr sólo el smoke test:
  ```sh
  cargo build --release --manifest-path ../../sqlite3mc-vendor-lib/Cargo.toml
  cargo run --release --manifest-path sqlite3mc/Cargo.toml
  ```

- **SQLite normal (baseline)**: motor real del crate raíz, feature
  `sqlite-plano` (`cargo build-plano`/`cargo test-plano` desde la raíz del
  repo) -- no necesita nada en esta carpeta.

Esta carpeta no cambia `Cargo.toml`, `AppCore`, el esquema ni los datos de
producción del proyecto raíz -- sigue siendo sólo el smoke test aislado
de cifrado de SQLite3MC. El benchmark comparativo real de 5 rondas /
1.000 ciclos contra `AppCore` (sección 12 de
`docs/auditorias/auditoria-rendimiento-core-rust-2026-09-10.md`) corre
ahora contra los tres motores reales (`cifrado-sqlcipher`/`sqlite-plano`/
`cifrado-sqlite3mc` del crate raíz), no contra estos crates de
laboratorio -- ver ese documento para el harness y los resultados.
