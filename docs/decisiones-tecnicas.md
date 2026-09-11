# Decisiones técnicas

Bitácora de decisiones de infraestructura/tooling que no son obvias leyendo
el código, para que otro dev (o vos mismo en unos meses) no tenga que
reconstruir el razonamiento. Orden cronológico, entradas más nuevas abajo.
No reemplaza los docs de `docs/auditorias/` (esos son diagnóstico técnico
puntual); esto es "por qué está configurado así".

---

## 2026-09-11 — Normalización de EOL (`.gitattributes`)

**Problema:** `desktop/src-tauri/Cargo.toml` aparecía como modificado en
`git status` sin ningún cambio real, incluso recién después de un
`git checkout` limpio, y reaparecía apenas algo (un build) le tocaba el
`mtime`.

**Causa:** el archivo quedó commiteado con CRLF en algún punto, mientras el
resto del repo usa LF. Con `core.autocrlf=true` (típico en Windows), Git
compara el archivo "limpiado" (CRLF→LF) contra el blob guardado -- si el
blob ya era CRLF, nunca coinciden.

**Decisión:** agregar `.gitattributes` con `* text=auto eol=lf` y
renormalizar el árbol (`git add --renormalize .`). Todo archivo de texto
se guarda en LF en el repo sin importar el `autocrlf` de cada máquina.

---

## 2026-09-11 — Switch de motor SQLite de tres vías

**Contexto:** esta rama (`bench/sqlite-3way-*`) existe para comparar tres
motores SQLite -- ver
[`docs/auditorias/AUDITORIA_RENDIMIENTO_CORE_RUST_2026-09-10.md`](auditorias/AUDITORIA_RENDIMIENTO_CORE_RUST_2026-09-10.md),
sección 12:

- `cifrado-sqlcipher` -- motor real de producción. Compila OpenSSL
  vendorizado desde fuente (`rusqlite/bundled-sqlcipher-vendored-openssl`),
  lento en frío (¬20-40 min según la máquina) y depende de un Perl completo
  (ver el gotcha de Strawberry Perl en la sección 18 del mismo audit doc).
- `sqlite-plano` -- SQLite sin cifrar, rápido para iterar (`rusqlite/bundled`).
  **Nunca** para builds que tocan datos reales.
- `cifrado-sqlite3mc` -- candidato en evaluación (SQLite3 Multiple Ciphers,
  ChaCha20-Poly1305). **Todavía no enlaza un motor real** -- no hay crate de
  bindings en crates.io, falta decidir e implementar la integración
  (vendorizar el amalgamation vs. link a DLL, sección 11 del audit doc).
  Activarla sola hoy hace que el build falle en el link -- es el
  comportamiento esperado hasta que se resuelva esa integración, no un bug.

Las tres son mutuamente excluyentes (`compile_error!` en `src/lib.rs`); no
elegir ninguna también es error.

### `desktop/src-tauri` -- default invertido a `sqlite-plano`

Antes, `desktop/src-tauri/Cargo.toml` tenía `cifrado-sqlcipher` hardcodeado
en la lista de features de su dependencia a `control_acceso`, así que
`cargo tauri dev`/`build` **siempre** compilaba el motor real sin forma de
pedir otro.

El fix obvio ("agregar un `[features]` que reenvíe al crate raíz, default
= motor real, pedir el plano con `-f`") **no funciona** con `tauri-cli`
2.11.4 (verificado): su flag `-f/--features` sólo agrega features, no
existe `--no-default-features` en `tauri dev`. Si el default fuera un
motor real, pedir el otro con `-f` activaría los dos a la vez y dispararía
la exclusión mutua.

Por eso el default de `desktop/src-tauri` es `sqlite-plano`, no
`cifrado-sqlcipher` -- al revés que el crate raíz. `cargo tauri dev` sin
flags es rápido en cualquier máquina mientras se evalúan los tres motores.
Para compilar/verificar un motor real específicamente, usar `cargo build`
directo (sí soporta `--no-default-features`), no `tauri dev -f`:

```sh
cargo build-desktop-cipher   # cifrado-sqlcipher, sin hot-reload
cargo build-desktop-3mc      # cifrado-sqlite3mc (no compila todavía, ver arriba)
```

(alias en `.cargo/config.toml`, funcionan desde cualquier directorio del
repo vía `--manifest-path`).

**Cuando se decida el motor final para producción**, este default debe
revisarse -- `sqlite-plano` como default de un crate que sí se empaqueta y
distribuye es intencional *sólo* mientras dura la evaluación de esta rama.

### `mobile/rust-core` -- mismo patrón, sin la limitación de CLI

`cargo ndk`/`cargo build`/`cargo test` sí soportan `--no-default-features`
nativamente (no son `tauri-cli`), así que ahí el default se quedó en el
motor real (`cifrado-sqlcipher`) -- comportamiento sin cambios para el día
a día. Alias para iterar rápido:

```sh
cargo build-mobile-plano   # host, --no-default-features --features sqlite-plano
cargo test-mobile-plano
cargo build-mobile-3mc     # ídem, no compila todavía (ver arriba)
```

Para el `.so` de dispositivo (`cargo ndk -t <target> build --release`), agregar
las mismas flags a mano -- `cargo-ndk` reenvía argumentos desconocidos a
`cargo build` (ver `mobile/README.md`).

### rust-analyzer

`.vscode/settings.json` tenía `"rust-analyzer.cargo.features": "all"`, que
activa TODAS las features de Cargo para el análisis del editor -- con dos
(y ahora tres) motores mutuamente excluyentes, eso dispara el
`compile_error!` como falso positivo permanente en `lib.rs`. Cambiado a
`[]` (usa las `default` del crate raíz, `cifrado-sqlcipher`). Es sólo
configuración del editor, no afecta ningún build real.

---

## 2026-09-11 — `mobile/rust-core` desactualizado tras el merge de conflictos de ingreso

Lo que en la entrada anterior quedó anotado como "pendiente" se resolvió
en la misma sesión. Causa real: el commit `72355da` (*"avisa
simétricamente cuando un conflicto de sitios se cuela offline"*) agregó
`conflictos_ingreso` a `ResumenSincronizacion` y migró la apertura de la
conexión secundaria a la fábrica central
(`database::connection::abrir_conexion_secundaria_escritura`), y lo hizo
en desktop (`desktop/src-tauri/src/comandos/nube.rs`) pero no en
`mobile/rust-core/src/lib.rs` -- quedó desincronizado, no relacionado al
switch de motores de esta rama.

Dos fixes, espejando exactamente el patrón ya usado en desktop:

1. **`conflictos_ingreso`**: en `sincronizar_con_secreto` (sync real), se
   calcula con `nube::contratistas_con_conflicto_activo(&conexion,
   &contexto).unwrap_or_default()` -- mejor esfuerzo, igual que desktop, no
   tumba el resto del sync si falla. En
   `configurar_dispositivo_inicial_con_secreto` (activación inicial, base
   recién configurada) queda `Vec::new()`, mismo criterio que desktop y que
   el `From<ResumenSincronizacionNucleo>` ya existente.
2. **`ruta_base_datos()`**: el núcleo (`AppCore`) ya no expone ese método
   (el free-function equivalente en `database::connection` es para el path
   *por defecto*, no el de una instancia abierta). `Nucleo::abrir` en
   mobile YA recibe el path como parámetro del constructor -- el fix fue
   simplemente guardarlo en un campo (`ruta_base_datos: PathBuf`, mismo
   patrón que `GuiState::ruta_base_datos` en escritorio) en vez de intentar
   leerlo de vuelta desde `AppCore`.

Verificado: `cargo build-mobile-plano --lib` compila limpio y
`cargo test-mobile-plano --lib` -- los 12 tests existentes del crate mobile
pasan sin cambios.
