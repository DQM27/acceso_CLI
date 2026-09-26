# 06 — Auditoría automatizada: herramientas, supply chain y código muerto

- Fecha: 2026-09-24
- Commit auditado: `372d93d` (rama `claude/modal-modifications-1nb87e`)
- Alcance: núcleo Rust (`/`), escritorio (`desktop/src-tauri` + `npm audit` de `desktop/`), móvil (`mobile/rust-core`), workflows de CI que afectan a escritorio/móvil. `web/` y `web-visitas/` quedan **fuera**.
- Directorio de compilación aparte: `CARGO_TARGET_DIR` fuera del repo (y `target-tools`, `target-cov`). No se editó ningún archivo versionado del repo.
- Máquina: Linux x86_64, 4 núcleos, 15 GB. Toolchain `rustc 1.94.1`, `cargo 1.94.1`, `clippy 0.1.94`.

---

## 1. Comandos ejecutados y resultado

| # | Comando | Herramienta / versión | Resultado |
|---|---|---|---|
| 1a | `cargo clippy --all-targets --no-default-features --features nube,sqlite-plano` (raíz) | clippy 0.1.94 | **Limpio**: 0 warnings, 0 errores (1 min 34 s en frío). |
| 1b | `cargo clippy --all-targets --no-default-features --features nube,sqlite-plano,cifrado-secreto-dispositivo` (raíz; misma combinación de features que el paso de CI "Clippy (features nube + cifrado-secreto-dispositivo)", con el motor plano en lugar de SQLCipher) | clippy 0.1.94 | **Limpio**. |
| 1c | `cargo clippy --all-targets --no-default-features --features sqlite-plano` (raíz sin `nube`) | clippy 0.1.94 | **Limpio**. |
| 1d | `cargo clippy --target x86_64-pc-windows-gnu --all-targets --no-default-features --features nube,sqlite-plano,cifrado-secreto-dispositivo` (raíz, compilación cruzada con mingw-w64, para cubrir el código `#[cfg(windows)]`: DPAPI en `nube/credenciales.rs`) | clippy 0.1.94 + mingw-w64 13.2 | **Limpio**. |
| 1e | Pasada de categorización: `cargo clippy --lib --bins --examples ... -- -W clippy::pedantic -W clippy::nursery -W clippy::cargo -W clippy::unwrap_used -W clippy::expect_used -W clippy::panic -W clippy::indexing_slicing -W clippy::string_slice -W clippy::unreachable -W clippy::as_conversions -W clippy::arithmetic_side_effects -W clippy::let_underscore_must_use -W clippy::unwrap_in_result -W clippy::shadow_unrelated -W clippy::dbg_macro -W clippy::todo -W clippy::unimplemented -W clippy::print_stdout ...` | clippy 0.1.94 | 755 warnings en la lib + 104 en examples. Agrupados en §2. No hay ningún `unwrap_used`, `panic`, `todo`, `unimplemented`, `dbg_macro` ni `print_stdout` en la lib. |
| 2 | `cargo test-plano --no-fail-fast` (= `cargo test --no-default-features --features nube,sqlite-plano`) | cargo 1.94.1 | **766 passed, 0 failed, 1 ignored** en 38 binarios de test (5 min 21 s). Ignorado: `tests/nube_smoke.rs:17` `autentica_un_dispositivo_real_y_recibe_un_token` (necesita red y un secreto de dispositivo real; justificado). |
| 3a | `cargo clippy --manifest-path mobile/rust-core/Cargo.toml --all-targets --no-default-features --features sqlite-plano` | clippy 0.1.94 | **Limpio** (51 s). |
| 3b | `cargo test --manifest-path mobile/rust-core/Cargo.toml --no-default-features --features sqlite-plano` | cargo 1.94.1 | **21 passed, 0 failed**. |
| 3c | `cargo clippy --manifest-path desktop/src-tauri/Cargo.toml --all-targets --no-default-features --features sqlite-plano` (host Linux, después de instalar `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libsoup-3.0-dev`, etc. con apt) | clippy 0.1.94 | **NO COMPILA en Linux**: 4 errores (`unresolved import zeroize`, `cannot find function intentar_abrir_nucleo`) y 2 imports sin usar. Ver [HT-05]. |
| 3d | Lo mismo con `--target x86_64-pc-windows-gnu` (cruzado) | clippy 0.1.94 + mingw-w64 | **Limpio** (2 min 41 s). Los tests de escritorio no se pudieron ejecutar (binarios Windows; no hay Wine). |
| 4a | `cargo audit --file <Cargo.lock>` sobre raíz, `desktop/src-tauri`, `mobile/rust-core`, `sqlite3mc-vendor-lib`, `benchmarks/sqlite-3way/sqlite3mc` | cargo-audit 0.22.2, advisory-db `ef8244d` (2026-09-24, 1268 advisories) | **0 vulnerabilidades** en los 5 lockfiles. **7 warnings "allowed"** (6 unmaintained + 1 unsound), todos en `desktop/src-tauri`. Ver [HT-01]. La comprobación de "yanked" falló a ratos por un 503 del índice de crates.io (proxy); no afecta al cruce con RustSec. |
| 4b | `cargo deny` | — | **No se ejecutó**: no existe `deny.toml`, y `docs/pendientes.md` registra que se descartó a propósito el 2026-09-17. |
| 4c | `npm audit --omit=dev` y `npm audit` en `desktop/` | npm 10.9.7 / node 22.22.2 | **0 vulnerabilidades** (45 dependencias de producción, 459 en total). |
| 5 | `cargo machete` en raíz, `desktop/src-tauri`, `mobile/rust-core`, `sqlite3mc-vendor-lib` (sobre una copia hecha con `git archive` en el scratchpad) | cargo-machete 0.9.2 | 1 candidato: `serde_json` en `desktop/src-tauri`, confirmado con grep. Ver [HT-06]. `cargo +nightly udeps` no se usó (no hay toolchain nightly). |
| 6 | Grep y scripts propios: `#[allow(dead_code)]`, `#[allow(unused`, `todo!`, `unimplemented!`, funciones `pub` del núcleo sin consumidores, módulos huérfanos | grep + python3 | Ver §3 y el inventario final. |
| 7 | `grep -rn unsafe` en el código propio | — | 10 bloques `unsafe`. Todos tienen `// SAFETY:`. Ver [HT-12]. |
| 8 | `uvx zizmor --offline --persona=auditor` y `uvx zizmor --offline` (persona por defecto) sobre `.github/workflows/` | zizmor 1.30.1 | Persona auditor: 61 hallazgos (1 alto, 14 medios, 21 bajos, 25 informativos). El alto está en `deploy-web.yml`, fuera de alcance. Persona por defecto (la misma que usa `zizmor.yml` en CI): **"No findings (61 suppressed)"**. Ver §5. |
| 9 | `cargo llvm-cov --no-default-features --features nube,sqlite-plano,cifrado-secreto-dispositivo --summary-only` (raíz) | cargo-llvm-cov (instalado con `--locked`) + llvm-tools-preview | **Líneas 85,92 %, regiones 83,24 %, funciones 86,61 %**. Los módulos con menos cobertura se detallan en [HT-10]. |

**Notas de entorno:**
- `api.github.com` y las descargas de releases de GitHub las bloquea el proxy (403), así que no se pudieron bajar binarios precompilados. `cargo-audit`, `cargo-machete` y `cargo-llvm-cov` se compilaron con `cargo install --locked`. La advisory-db se clonó con git sin problema.
- Efecto secundario: `cargo machete --with-metadata` creó `benchmarks/sqlite-3way/sqlcipher/Cargo.lock` dentro del repo. El coordinador ya lo eliminó. Además, el build de `desktop/src-tauri` (tauri-build) generó `desktop/src-tauri/gen/schemas/` (18:43 UTC). Está en `.gitignore` y `git status --short` queda limpio, pero es un directorio creado por esta auditoría: **conviene borrarlo a mano** (`desktop/src-tauri/gen/`). La herramienta no me permitió hacer borrados dentro del repo.

---

## 2. Clippy: categorización (pasada con lints extra, solo para contar)

Con la configuración real del repo (`[lints.clippy]` con pedantic/nursery en warn y unos 40 lints en deny), **los tres crates quedan limpios**. Los números de abajo salen de activar lints que el repo tiene en `allow` o que pertenecen al grupo *restriction*:

| Lint | Ocurrencias (lib) | Comentario |
|---|---|---|
| `missing_errors_doc` | 388 | Está en `allow` en Cargo.toml a propósito. Ruido. |
| `too_long_first_doc_paragraph` | 134 | `allow` a propósito. Ruido. |
| `missing_const_for_fn` | 69 | `allow`. Ruido. |
| `must_use_candidate` (función + método) | 82 | `allow`. Ruido. |
| `arithmetic_side_effects` | 45 | Sumas de contadores `u32`/`usize` (`resumen.enviados += ...` en `nube/sincronizacion.rs`, paginación en `application/historial.rs`). Revisadas: son contadores acotados por el tamaño de lotes y páginas. Sin riesgo práctico. En release el desbordamiento hace wrap sin avisar; se podría usar `saturating_add` por prolijidad. |
| `indexing_slicing` | 9 | `historial/exportacion.rs:305-372` (`formatos.x[variante]` con `variante = fila % 2` sobre arrays `[_; 2]`, índice acotado de forma matemática) y `nube/sincronizacion.rs:243,251` (`grupo[0]` detrás de `grupo.len() > 1`). Ninguno puede entrar en pánico. |
| `needless_pass_by_value` | 8 | `allow`. |
| `unreachable` | 5 | `mensajes.rs:186,219`, `nube/sincronizacion.rs:177`, `database/schema.rs:83`, `services/registro_ingreso_service.rs:330`. Invariantes documentados. Aceptable. |
| `expect_used` | 2 (+1 en `build.rs`) | `nube/credenciales.rs:46,57`: tamaño fijo de SHA-256 y del nonce. Correcto. |
| `as_conversions` | 2 | `historial/exportacion.rs:299,366` (`(fila % 2) as usize`). Inocuo. |
| `let_underscore_must_use` | 2 | `application/mod.rs:134` (`PRAGMA optimize` en `Drop`) y `database/connection.rs:230` (`write!` a `String`). Intencionales. |
| `string_slice` | 2 | `lenguaje_comandos/resolver.rs:559,614`. Código muerto, ver [HT-02]. |
| `shadow_unrelated` | 1 | `nube/sincronizacion.rs:132`. |
| `clippy::cargo` | 4 metadatos + 2 duplicados | `syn` 2.0.119/3.0.3 y `getrandom` 0.2.17/0.4.3 duplicados en el árbol. Informativo. |
| `unwrap_used`, `panic`, `todo`, `unimplemented`, `dbg_macro`, `print_stdout` | **0** | — |

**Conteo con grep de `.unwrap()` / `.expect(` fuera de `#[cfg(test)]`** (script que elimina por balanceo de llaves los items marcados con `#[cfg(test)]`):

| Crate | Líneas no-test | `.unwrap()` | `.expect(` | `unreachable!` |
|---|---|---|---|---|
| núcleo `src/` | 19 856 | 0 | 2 (`nube/credenciales.rs:46,57`) | 5 |
| `desktop/src-tauri/src` | 3 435 | 4 | 5 | 0 |
| `mobile/rust-core/src` | 2 239 | 0 | 0 | 0 |

Detalle de escritorio: `lib.rs:592` (`.expect("error while running tauri application")`, patrón estándar de Tauri), `comandos/historial.rs:21` (`with_ymd_and_hms(2000,1,1,...).unwrap()`, constante válida), `pdf/html.rs:137,150,191,200` (`write!` a `String`) y `pdf/generador.rs:78,198,204` (`Mutex::lock().unwrap()`, ver [HT-11]).

Referencias: índice de lints de Clippy, https://rust-lang.github.io/rust-clippy/master/index.html (`unwrap_used`, `indexing_slicing`, `arithmetic_side_effects`).

---

## 3. Hallazgos

### [HT-01] Siete advisories RustSec "unmaintained"/"unsound" en el árbol de escritorio
- Severidad: Baja
- Categoría: Supply chain
- Ubicación: `desktop/src-tauri/Cargo.lock` (dependencias transitivas de `tauri` 2.11.5)
- Estado: Pendiente de auditoría previa (`docs/pendientes.md`, entrada "`cargo audit` corrido en los 3 crates (2026-09-12)": "7 avisos unmaintained/unsound", aceptados sin acción). Son los mismos 7, no apareció ninguno nuevo.
- Evidencia (`cargo audit`, advisory-db 2026-09-24):

  | ID | Crate | Versión | Tipo | Cómo llega al árbol | ¿Se usa en el build de Windows? |
  |---|---|---|---|---|---|
  | RUSTSEC-2024-0429 | glib | 0.18.5 | unsound (`VariantStrIter`) | tauri → muda → gtk → atk → glib | **No**: `cargo tree -i glib --target x86_64-pc-windows-msvc` devuelve "nothing to print". Solo entra en el build de Linux/GTK. |
  | RUSTSEC-2024-0370 | proc-macro-error | 1.0.4 | unmaintained | glib-macros (proc-macro, solo en compilación) | No (solo GTK) |
  | RUSTSEC-2025-0075 | unic-char-range | 0.9.0 | unmaintained | tauri-utils → urlpattern | Sí, pero sin CVE |
  | RUSTSEC-2025-0080 | unic-common | 0.9.0 | unmaintained | ídem | Sí, sin CVE |
  | RUSTSEC-2025-0081 | unic-char-property | 0.9.0 | unmaintained | ídem | Sí, sin CVE |
  | RUSTSEC-2025-0098 | unic-ucd-version | 0.9.0 | unmaintained | ídem | Sí, sin CVE |
  | RUSTSEC-2025-0100 | unic-ucd-ident | 0.9.0 | unmaintained | ídem | Sí, sin CVE |

  Los lockfiles de la raíz, `mobile/rust-core` y `sqlite3mc-vendor-lib` no tienen ningún advisory. `npm audit` de `desktop/` da 0.
- Impacto: ninguno explotable hoy. El único "unsound" (glib, fallo de memoria en `VariantStrIter::impl_get`, corregido en glib ≥ 0.20) solo se compila para Linux, y la app se distribuye para Windows. Los `unic-*` no tienen mantenimiento pero tampoco CVE.
- Referencia externa: https://rustsec.org/advisories/RUSTSEC-2024-0429.html, https://rustsec.org/advisories/RUSTSEC-2024-0370.html, https://rustsec.org/advisories/RUSTSEC-2025-0100.html
- Recomendación: no hace falta acción de código (depende de que upstream migre `tauri`/`muda` a gtk-rs ≥ 0.20 y `urlpattern` fuera de `unic`). Para que dejen de aparecer como "aceptados en silencio", agregar un `audit.toml` (`[advisories] ignore = [...]`) en `desktop/src-tauri/.cargo/` con un comentario del porqué y una fecha de revisión, y correr `cargo audit --deny warnings` en CI. Así cualquier advisory *nuevo* de tipo unmaintained/unsound hace fallar el job en vez de quedar tapado.

### [HT-02] Módulo `lenguaje_comandos` completo es código muerto (≈1 430 líneas + 38 tests)
- Severidad: Media
- Categoría: Código muerto
- Ubicación: `src/lib.rs:5-10`, `src/lenguaje_comandos/{mod.rs,contexto.rs,parser.rs,resolver.rs}`
- Estado: Nuevo. El propio código lo reconoce como "huérfano desde que se retiraron CLI/TUI (2026-09-12)" (`src/lib.rs:6-9`), pero ninguna auditoría previa lo registra como pendiente.
- Evidencia: `grep -rn lenguaje_comandos --include=*.rs` fuera del módulo solo encuentra la declaración `pub mod` y un comentario viejo en `src/application/mod.rs:19`. Ni `desktop/src-tauri`, ni `mobile/rust-core`, ni `tests/` lo importan. El comentario de su `mod.rs` menciona `application/comandos.rs` y `comandos/`, que ya no existen. Clippy no lo marca como `dead_code` porque es `pub`. Además arrastra consigo `AppCore::buscar_auditoria` (`src/application/catalogos.rs:88`), cuyo único llamador de producción es `lenguaje_comandos/resolver.rs:319`. La cobertura de `resolver.rs` es 9,43 %.
- Impacto: 1 430 líneas que se compilan en los tres binarios, 38 tests que se ejecutan en cada CI sin proteger nada que llegue al usuario, y lints de restricción (`string_slice` en `resolver.rs:559,614`, que puede entrar en pánico con índices que caen a mitad de un carácter UTF-8) en código que nadie ejecuta. Confunde a quien audita: parece superficie viva.
- Referencia externa: https://doc.rust-lang.org/rustc/lints/listing/warn-by-default.html#dead-code (el lint `dead_code` no cubre ítems `pub` de una librería).
- Recomendación: **eliminar** `src/lenguaje_comandos/` y el `pub mod` de `lib.rs`. El código ya queda preservado en la rama `archive/cli-tui-2026-09-12`, como dice el propio comentario. Eliminar también `AppCore::buscar_auditoria` si no se le encuentra otro uso, y el comentario obsoleto de `src/application/mod.rs:19-20`.

### [HT-03] API pública del núcleo sin ningún consumidor (12 funciones) y 13 que solo usan los tests
- Severidad: Baja
- Categoría: Código muerto
- Ubicación: ver el inventario (§6). Las principales: `src/application/usuarios.rs:92,102,215,242,290,311`, `src/services/usuario_service.rs:179`, `src/application/nube.rs:719,751`, `src/database/connection.rs:199`, `src/tiempo.rs:116`, `src/models/{tipo_ingreso.rs:41,gafete.rs:34,85}`
- Estado: Nuevo
- Evidencia: script que junta cada `pub fn` del código no-test de `src/` y cuenta referencias por palabra completa en el código de producción del núcleo, `desktop/src-tauri/src`, `mobile/rust-core/src`, `tests/`, `examples/` y `benchmarks/`. Cada candidato se verificó a mano con `grep -rnw` (incluidos `.kt`/`.swift`/`.ts`): **0 referencias** fuera de la definición y de comentarios. Varios doc-comments todavía nombran a la "TUI"/"CLI" como su llamador (p. ej. `usuarios.rs:90-91` "sin bloquear la TUI", `usuarios.rs:238` "gate de `/clave` en la CLI", `connection.rs:195` "único caso real de TUI/CLI"). Son restos de la interfaz de terminal retirada.
- Impacto: API muerta alrededor de la gestión de contraseñas (`*_con_hash`, `preparar_cambio_password_propio`, `verificar_mi_password`). Aumenta la superficie a revisar en seguridad y puede sugerir por error que existen caminos que saltan validaciones.
- Referencia externa: https://doc.rust-lang.org/rustc/lints/listing/warn-by-default.html#dead-code
- Recomendación: eliminar las 12 funciones sin uso (tabla §6). Para las 13 "solo tests" (`resetear_password_root`, `listar_roots_activos`, `registrar_salida_por_gafete`, `dev_auth::*`, ...), decidir caso por caso: si solo sirven como utilidad de test, bajarlas a `pub(crate)` o ponerles `#[cfg(test)]`, o moverlas a un helper de `tests/`. Una alternativa estructural es `#![warn(unreachable_pub)]` combinado con reducir la visibilidad de los módulos internos.

### [HT-04] Restos de la interfaz de terminal retirada: `build.rs`, `winresource`, perfil `release-native`, README y packaging MSIX
- Severidad: Baja
- Categoría: Código muerto
- Ubicación: `build.rs:1-24`, `Cargo.toml:81-82` (`[build-dependencies] winresource`), `Cargo.toml:164-165` (`[profile.release-native]`), `.cargo/config.toml:2` (alias `build-native`), `README.md:28-40,141-165`, `packaging/msix/README.md:28-36`, `Cargo.toml:166-178` (comentario de `profile.production`)
- Estado: Nuevo
- Evidencia:
  - `build.rs:8` lee `CARGO_FEATURE_TERMINAL_UI`, pero la feature `terminal-ui` ya no existe en `Cargo.toml`. La rama de `winresource` no se ejecuta nunca y aun así la build-dependency se compila en cada build.
  - El crate raíz **no tiene ningún target binario** (no hay `src/main.rs` ni `src/bin/` ni `[[bin]]`). `cargo build-native` y el `target\release-native\control_acceso.exe` del README, junto con `Copy-Item target\release\control_acceso.exe` de `packaging/msix/README.md`, apuntan a un ejecutable que ya no existe.
  - El README todavía documenta la TUI (Ratatui, `--reset-root`, respaldos "desde la propia TUI"). `plan-qa-buenas-practicas` punto 4 confirma que los respaldos locales ya no existen.
  - El comentario de `[profile.production]` (`Cargo.toml:170-173`) justifica `panic = "abort"` con que los únicos `expect/unwrap` están en "credenciales.rs, formulario.rs", pero `formulario.rs` ya no existe. `profile.production` tampoco tiene efecto sobre `desktop/src-tauri` ni `mobile/rust-core`: son raíces de workspace distintas y Cargo solo lee perfiles del manifiesto raíz del workspace.
- Impacto: documentación engañosa (un operador que siga el README o el procedimiento MSIX no va a poder compilar nada), y una dependencia y un build script que se compilan en cada build sin hacer nada.
- Referencia externa: https://doc.rust-lang.org/cargo/reference/profiles.html ("Profile settings in a dependency's manifest are ignored").
- Recomendación: eliminar `build.rs` y `winresource`. Eliminar `[profile.release-native]` y el alias `build-native` (o moverlos a `desktop/src-tauri` si se quiere un build nativo del instalador). Reescribir las secciones TUI del README. Marcar `packaging/msix/` como obsoleto o eliminarlo (Tauri ya genera MSI/NSIS). Corregir el comentario de `profile.production` y aclarar que solo aplica al crate raíz.

### [HT-05] `desktop/src-tauri` no compila fuera de Windows, y la rama `#[cfg(not(windows))]` está rota
- Severidad: Baja
- Categoría: Mala práctica
- Ubicación: `desktop/src-tauri/src/lib.rs:2,8,10,251-255`, `desktop/src-tauri/src/estado.rs:11`, `desktop/src-tauri/Cargo.toml` (`zeroize` solo en `[target.'cfg(windows)'.dependencies]`)
- Estado: Nuevo
- Evidencia (`cargo clippy` en Linux con webkit2gtk instalado):
  ```
  src/lib.rs:10:5: error[E0432]: unresolved import `zeroize`
  src/estado.rs:11:5: error[E0432]: unresolved import `zeroize`
  src/lib.rs:253:9: error[E0425]: cannot find function `intentar_abrir_nucleo` in this scope
  src/lib.rs:2:5: warning: unused import: `std::sync::Arc`
  src/lib.rs:8:5: warning: unused import: `control_acceso::tiempo::RelojCorregido`
  ```
  La rama `#[cfg(not(windows))]` de `abrir_nucleo_con_recuperacion` llama a `intentar_abrir_nucleo`, que está marcada `#[cfg(windows)]` (`lib.rs:186`). Con `--target x86_64-pc-windows-gnu` compila limpio.
- Impacto: la app es solo para Windows, así que no afecta a producción. Pero (a) la rama no-Windows es código muerto que nunca compiló; (b) **el job `cargo-geiger` del crate `desktop` corre en `ubuntu-latest` y falla siempre**, y el `|| true` de `cargo-geiger.yml:62` lo tapa, así que el reporte de `unsafe` de escritorio no se genera nunca (ver [HT-08]); (c) nadie puede compilar ni revisar escritorio en Linux o macOS.
- Referencia externa: https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#platform-specific-dependencies
- Recomendación: o bien mover `zeroize` a `[dependencies]` y quitar el `#[cfg(windows)]` de `intentar_abrir_nucleo` (o darle una implementación para no-Windows), o bien declarar el crate como solo Windows (`#![cfg(windows)]` o un `compile_error!` explícito en no-Windows) y pasar el job de geiger de escritorio a `windows-latest`. Agregar un `cargo check --target x86_64-pc-windows-gnu` o un check en Linux en CI para que no se vuelva a romper sin que nadie lo note.

### [HT-06] Dependencia sin uso: `serde_json` en escritorio
- Severidad: Info
- Categoría: Supply chain
- Ubicación: `desktop/src-tauri/Cargo.toml` (`serde_json = "1.0"` en `[dependencies]`)
- Estado: Nuevo
- Evidencia: `cargo machete` → `control-acceso-desktop -- ./Cargo.toml: serde_json`. `grep -rn "serde_json\|json!" desktop/src-tauri/src` no devuelve nada. En la raíz, `mobile/rust-core` y `sqlite3mc-vendor-lib`, machete no encontró nada. Candidatos del crate raíz verificados a mano y que **sí se usan**: `tempfile` (producción, `application/historial.rs:271`), `chrono-tz`, `rand_core`, `log`, `thiserror`.
- Impacto: mínimo (igual entra de forma transitiva por `tauri`), pero es una declaración directa que no hace falta.
- Referencia externa: https://github.com/bnjbvr/cargo-machete
- Recomendación: quitar `serde_json` de `desktop/src-tauri/Cargo.toml`, o si se deja a propósito, agregarlo a `[package.metadata.cargo-machete] ignored` con un comentario.

### [HT-07] Workflows `db-cipher-e2e.yml` y `db-cipher-lab.yml` apuntan a un directorio que no existe
- Severidad: Baja
- Categoría: CI/CD
- Ubicación: `.github/workflows/db-cipher-e2e.yml:18-22`, `.github/workflows/db-cipher-lab.yml:19-67`
- Estado: Nuevo
- Evidencia: los dos ejecutan `cargo run --manifest-path experiments/...`, pero `experiments/` no existe en el repo y está en `.gitignore:15`. Se disparan con push a ramas de fecha vieja (`fix/...-2026-09-10`) y con `workflow_dispatch`. Si alguien los lanza a mano, fallan siempre. `sqlite3mc-vendor-lib/build.rs:19`, `sqlite3mc-vendor-lib/Cargo.toml` y `benchmarks/sqlite-3way/sqlite3mc/Cargo.toml` también remiten a `experiments/db-cipher-lab/README.md`, que ya no existe.
- Impacto: CI muerto que agranda la superficie de workflows (zizmor los audita igual: `superfluous-actions`, `concurrency-limits`) y documentación que remite a archivos que ya no están.
- Referencia externa: https://docs.github.com/en/actions/security-for-github-actions/security-guides/security-hardening-for-github-actions (reducir la superficie de workflows).
- Recomendación: eliminar los dos workflows y actualizar las referencias a `experiments/db-cipher-lab/README.md` para que apunten a `benchmarks/sqlite-3way/README.md` o `docs/decisiones-tecnicas.md`.

### [HT-08] `cargo-geiger.yml`: `|| true` tapa fallos reales de compilación
- Severidad: Baja
- Categoría: CI/CD
- Ubicación: `.github/workflows/cargo-geiger.yml:59-62`
- Estado: Nuevo
- Evidencia: `run: cargo geiger --no-default-features --features ${{ matrix.features }} || true`. La fila `desktop` corre en `ubuntu-latest` sin webkit2gtk y además el crate no compila en Linux ([HT-05]), así que ese reporte falla siempre y el job sale en verde. zizmor (persona auditor) marca además `template-injection` en la línea 64: `${{ matrix.features }}` dentro de `run:`. Hoy es inocuo porque la matriz es estática, pero conviene pasarlo por `env:`.
- Impacto: da una falsa sensación de cobertura de `unsafe` de dependencias para escritorio.
- Referencia externa: https://docs.zizmor.sh/audits/#template-injection
- Recomendación: diferenciar "geiger encontró unsafe", que es aceptable, de "no compiló", que debe hacer fallar el job (por ejemplo `cargo geiger ... --output-format Json > r.json; test -s r.json`). Pasar la fila de escritorio a `windows-latest`. Mover `matrix.features` a `env:`.

### [HT-09] Secretos de firma a nivel de job y sin `environment` protegido; herramientas instaladas sin fijar versión dentro del job de release
- Severidad: Media
- Categoría: CI/CD
- Ubicación: `.github/workflows/release.yml:113-117` (env de job con `ANDROID_KEYSTORE_BASE64`, `ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS`, `ANDROID_KEY_PASSWORD`), `release.yml:84-87` (`TAURI_SIGNING_PRIVATE_KEY*`), `release.yml:160` (`cargo install cargo-ndk`), `build-test.yml:56`, `build-test-mobile.yml:46`; `ci.yml:40-43,74-77,166-169` (`taiki-e/install-action` con `tool: cargo-audit` sin versión); `zizmor.yml:36` (`pipx run zizmor` sin versión)
- Estado: Nuevo. Las auditorías previas solo registran que zizmor se agregó; estos hallazgos no aparecen.
- Evidencia: zizmor 1.30.1 `--persona=auditor`: `secrets-outside-env` (Medium, confianza High) en `release.yml:19` y `release.yml:100`, y `excessive-permissions` (Medium) en `release.yml:1:1` y `cargo-geiger.yml:1:1` (sin bloque `permissions:` a nivel de workflow; los jobs sí lo definen). En el job `build-android`, los 4 secretos del keystore quedan en el `env` de **todo el job**. Eso significa que `cargo install cargo-ndk` (sin `--locked` ni versión fija: baja la última versión publicada en crates.io y ejecuta sus build scripts), `cargo ndk build` (build scripts de ~300 crates) y `./gradlew` (plugins de Gradle) corren con esos secretos en su entorno y con un `GITHUB_TOKEN` con `contents: write`. Con la persona por defecto, que es la que corre en CI, zizmor informa "No findings (61 suppressed)", así que CI no los muestra.
- Impacto: un crate o plugin comprometido en la cadena de build del release podría leer la llave de firma del APK y la contraseña del keystore (filtración que no se puede revertir; con esa llave se pueden firmar actualizaciones maliciosas para la app de la portería). El riesgo baja porque el trigger solo lo dispara quien tiene permiso de push de tags o dispatch, y las acciones están fijadas por SHA. Aun así, la instalación sin versión fija de `cargo-ndk`/`zizmor`/`cargo-audit` rompe la política de "todo fijado" que el repo sí aplica a las actions.
- Referencia externa: https://docs.zizmor.sh/audits/#secrets-outside-env, https://docs.zizmor.sh/audits/#excessive-permissions, https://docs.github.com/en/actions/security-for-github-actions/security-guides/security-hardening-for-github-actions#using-secrets, https://docs.github.com/en/actions/managing-workflow-runs-and-deployments/managing-deployments/managing-environments-for-deployment
- Recomendación: (1) crear un `environment: release` con reviewers requeridos y mover los secretos ahí; (2) pasar los secretos del keystore solo a los pasos "Restaurar el keystore" y "Compilar el APK de release" (`env:` de paso, no de job), y borrar `app/keystore/release.keystore` y `keystore.properties` en un paso `if: always()` final; (3) `cargo install cargo-ndk --locked --version X.Y.Z` (o `taiki-e/install-action` con `tool: cargo-ndk@X.Y.Z`); (4) `tool: cargo-audit@0.22.2`, `pipx run zizmor==1.30.1`; (5) agregar `permissions: {}` a nivel de workflow en `release.yml` y `cargo-geiger.yml`; (6) evaluar correr zizmor en CI con `--persona=pedantic`, o al menos con `--min-severity=medium`, para que estos hallazgos se vean.

### [HT-10] Huecos de pruebas en CI: ni los tests del núcleo con el motor que va a producción (SQLite3MC) ni los tests Rust de móvil corren en CI
- Severidad: Media
- Categoría: Pruebas
- Ubicación: `.github/workflows/ci.yml:60-106` (job `test`), `ci.yml:13-58` (job `test-android`), `.cargo/config.toml:33` (alias `test-3mc` sin uso en CI)
- Estado: Nuevo
- Evidencia:
  - El release de escritorio (default `cifrado-sqlite3mc`) y el de Android (`--features cifrado-sqlite3mc`, `release.yml:181,192`) usan **SQLite3MC**. El job `test` de CI ejecuta los tests del núcleo (766) solo con el default del crate raíz, que es **SQLCipher**. El alias `test-3mc` existe, pero ningún workflow lo usa. Solo los 33 tests de `desktop/src-tauri` pasan por SQLite3MC.
  - `test-android` corre `fmt`, `audit` y `clippy` de `mobile/rust-core`, pero **no `cargo test`**. Sus 21 tests (que pasan localmente, ver §1) no se ejecutan en CI. `gradlew testDebugUnitTest` compila el núcleo del host con `cargo build --release` (`app/build.gradle.kts:144`), sin tests Rust.
  - Cobertura del núcleo (`cargo llvm-cov`, plano + nube): **85,92 % de líneas**. Módulos bajos: `application/nube.rs` 7,85 % (fachada de sincronización, depende de red), `lenguaje_comandos/resolver.rs` 9,43 % (muerto, [HT-02]), `application/gafetes.rs` 41,7 %, `models/cita.rs` 45,5 %, `mensajes.rs` 54,6 %, `application/autenticacion.rs` 55,8 %, `application/usuarios.rs` 59,7 %, `database/connection.rs` 60,4 %. En escritorio, los 15 módulos de `comandos/` (frontera IPC de Tauri) tienen 3 tests en total (`exportacion.rs` 1, `historial.rs` 2).
- Impacto: una regresión específica del motor de producción (pragmas de cifrado, FTS5 o comportamiento de `PRAGMA key` en SQLite3MC) puede llegar a un release sin que ningún test del núcleo lo detecte. Los tests del puente UniFFI de móvil pueden romperse sin que nadie se entere.
- Referencia externa: https://github.com/taiki-e/cargo-llvm-cov
- Recomendación: agregar a `ci.yml` (job `test-gui`, que ya compila `sqlite3mc-vendor-lib`) un paso `cargo test-3mc`, y a `test-android` un paso `cargo test` en `mobile/rust-core` (con el motor plano para ir rápido, o `cifrado-sqlite3mc` con el vendor compilado). Agregar tests para `application/gafetes.rs` y `application/usuarios.rs`, y un mock HTTP (p. ej. `mockito`) para `application/nube.rs`. Opcional: publicar `cargo llvm-cov --lcov` como artefacto.

### [HT-11] `Mutex::lock().unwrap()` en el generador de PDF de escritorio
- Severidad: Baja
- Categoría: Mala práctica
- Ubicación: `desktop/src-tauri/src/pdf/generador.rs:78,198,203-204`
- Estado: Nuevo
- Evidencia: `let transmisor = tx.lock().unwrap().take();` y `*resultado_interno_closure.lock().unwrap() = Some(resultado);`, este último dentro del closure de `with_webview`, que corre en el hilo de la UI. El resto de escritorio (`estado.rs`) maneja el envenenamiento del Mutex con cuidado.
- Impacto: si un pánico previo envenena el mutex, exportar a PDF provoca un segundo pánico en el hilo de WebView2. Es improbable, pero el patrón es inconsistente con el resto del código.
- Referencia externa: https://rust-lang.github.io/rust-clippy/master/index.html#unwrap_used, https://doc.rust-lang.org/std/sync/struct.Mutex.html#poisoning
- Recomendación: usar `lock().unwrap_or_else(PoisonError::into_inner)` o mapear el error a `String`, que ya es el tipo de error de la función. Considerar activar `clippy::unwrap_used = "warn"` en los tres `Cargo.toml` (la lib del núcleo ya da 0).

### [HT-12] Bloques `unsafe`: 10 en código propio, todos justificados; queda una mejora menor de higiene de memoria
- Severidad: Info
- Categoría: Seguridad
- Ubicación y evaluación:

  | Ubicación | Operación | ¿Justificado? |
  |---|---|---|
  | `src/nube/credenciales.rs:141` | `CryptProtectData` | Sí. FFI obligatoria a DPAPI, `// SAFETY:` correcto, `cbData` con `u32::try_from(...).ok()?`. |
  | `src/nube/credenciales.rs:166` | `CryptUnprotectData` | Sí. `CRYPTPROTECT_UI_FORBIDDEN`. |
  | `src/nube/credenciales.rs:188` | `slice::from_raw_parts` sobre el buffer de DPAPI | Sí. Comprueba null y longitud 0 antes. |
  | `src/nube/credenciales.rs:191` | `LocalFree` | Sí. Es la contraparte documentada. |
  | `desktop/src-tauri/src/clave_cifrado.rs:154,181,203,206` | Espejo de los 4 anteriores para `db_key.dat` | Sí. Diferencia menor: `proteger` usa `u32::try_from(len).unwrap_or(u32::MAX)` en vez de `.ok()?` (inocuo con una clave de 32 bytes). |
  | `desktop/src-tauri/src/pdf/generador.rs:183` | `ICoreWebView2Controller::CoreWebView2()` (COM) | Sí. El binding autogenerado es `unsafe`; se llama dentro de `with_webview`, en el hilo correcto. |
  | `desktop/src-tauri/src/pdf/generador.rs:196` | `ICoreWebView2_7::PrintToPdf` (COM) | Sí. |

  `mobile/rust-core` tiene `#![forbid(unsafe_code)]`. Los lints `undocumented_unsafe_blocks` y `multiple_unsafe_ops_per_block` están en deny y se cumplen (clippy cruzado a Windows limpio). `sqlite3mc-vendor-lib/src` no tiene `unsafe`.
- Estado: Nuevo (matiz). La auditoría previa `auditoria-calidad-2026-09.md` §6 dice "Cero `unsafe` en todo el crate (raíz + mobile)". Eso **ya no es cierto** para la raíz: el DPAPI de `nube/credenciales.rs` es posterior a esa auditoría. `docs/pendientes.md` sí lo registra correctamente.
- Impacto: ningún problema de memoria. Mejora de higiene: `copiar_y_liberar` copia el texto plano descifrado (secreto de dispositivo o clave de la base) a un `Vec` sin `zeroize` y libera el buffer de DPAPI con `LocalFree` **sin sobrescribirlo antes**, así que el secreto queda en el heap liberado del proceso.
- Referencia externa: https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptunprotectdata ("free the pbData member by calling LocalFree"; Microsoft recomienda `SecureZeroMemory` para datos sensibles antes de liberar), https://rust-lang.github.io/rust-clippy/master/index.html#undocumented_unsafe_blocks
- Recomendación: en la ruta de descifrado, poner a cero el buffer antes de `LocalFree` (`std::ptr::write_bytes` o `SecureZeroMemory`) y devolver `Zeroizing<Vec<u8>>` (escritorio ya depende de `zeroize`). Actualizar la afirmación de "cero unsafe" de `auditoria-calidad-2026-09.md`.

### [HT-13] Allow de `dead_code` en campos deserializados que no se leen
- Severidad: Info
- Categoría: Código muerto
- Ubicación: `src/nube/sincronizacion.rs:2136-2140` (`FilaGafeteOcupado { #[allow(dead_code)] id }`), `src/nube/auth_supabase.rs:236-241` (`ClaimsToken { #[allow(dead_code)] exp }`)
- Estado: Nuevo
- Evidencia: son los únicos dos `#[allow(dead_code)]` del código propio. No hay ningún `#[allow(unused...)]`, `todo!` ni `unimplemented!`. `FilaGafeteOcupado` solo se usa para contar filas. `exp` lo valida `jsonwebtoken` (`Validation::validate_exp`), así que no hace falta leerlo en Rust.
- Impacto: ninguno.
- Referencia externa: https://rust-lang.github.io/rust-clippy/master/index.html#allow_attributes
- Recomendación: cambiarlos por `#[expect(dead_code, reason = "...")]` (Rust ≥ 1.81; el MSRV es 1.89), para que el compilador avise si el campo empieza a usarse. En `FilaGafeteOcupado` se puede usar `serde::de::IgnoredAny` o un struct vacío `{}`.

### [HT-14] Documentación de auditorías y pendientes desactualizada respecto del código
- Severidad: Info
- Categoría: Código muerto
- Ubicación: `docs/auditorias/auditoria-calidad-2026-09.md:230-233` ("Cero `unsafe`", "Único caso en producción (`cli/formulario.rs:410`)"), `docs/pendientes.md:358-370` ("4 upgrades de Dependabot NO mergeados": `jsonwebtoken` 9→11 y `argon2` 0.5→0.6), `src/lenguaje_comandos/mod.rs:1-10`, `src/application/mod.rs:19-20`
- Estado: Nuevo
- Evidencia: `Cargo.toml` ya declara `jsonwebtoken = "11"` y `argon2 = "0.6"`. `src/cli/` no existe. Hay `unsafe` en `src/nube/credenciales.rs`.
- Impacto: las auditorías futuras parten de premisas falsas.
- Referencia externa: —
- Recomendación: actualizar esas entradas (marcar los upgrades como hechos y corregir la afirmación de "cero unsafe").

---

## 4. Supply chain: resumen

| Lockfile | Crates | Vulnerabilidades | Warnings | Estado |
|---|---|---|---|---|
| `Cargo.lock` (raíz) | 274 | 0 | 0 | OK |
| `desktop/src-tauri/Cargo.lock` | 699 | 0 | 7 (ver [HT-01]) | Pendiente previo, aceptado |
| `mobile/rust-core/Cargo.lock` | 299 | 0 | 0 | OK |
| `sqlite3mc-vendor-lib/Cargo.lock` | 4 | 0 | 0 | OK |
| `benchmarks/sqlite-3way/sqlite3mc/Cargo.lock` | 9 | 0 | 0 | OK |
| `desktop/package-lock.json` (`npm audit --omit=dev` y completo) | 459 (45 de prod) | 0 | — | OK |

`cargo deny`: no aplica (sin `deny.toml`; descartado a propósito en `docs/pendientes.md`). `dependency-review.yml` y Dependabot cubren los 3 ecosistemas cargo.

## 5. CI/CD: resumen para escritorio y móvil (zizmor 1.30.1 + revisión manual)

| Aspecto | Estado |
|---|---|
| Acciones fijadas por SHA | **OK**: todas las `uses:` usan un SHA de 40 caracteres con comentario de versión. |
| Permisos de `GITHUB_TOKEN` | Correctos a nivel de job (`contents: read` salvo en release, que usa `contents: write`). A `release.yml` y `cargo-geiger.yml` les falta `permissions: {}` a nivel de workflow ([HT-09]). |
| `pull_request_target` / `workflow_run` / `issue_comment` | **No se usan**. OK. |
| Inyección de plantillas | Ninguna con datos que controle un atacante. `cargo-geiger.yml:64` usa `matrix.features` (estático) y `release.yml` usa `inputs.version` solo en `with:` y `env:`, no en `run:`. OK. |
| `persist-credentials: false` en checkout | **OK** en todos. |
| Secretos en logs | No se encontraron `echo`/`set -x` con secretos. El keystore se reconstruye con `echo "$ANDROID_KEYSTORE_BASE64" \| base64 --decode` (la variable no se imprime). Queda el problema del alcance del `env` de job ([HT-09]). |
| Envenenamiento de caché | `release.yml` usa `lookup-only: true` y `package-manager-cache: false`, correcto. `ci.yml` restaura caché en PR, lo cual es aceptable: GitHub aísla las cachés de PR por ref y no pueden escribir en la de `main`. OK. |
| Instalación de herramientas sin fijar versión | `cargo install cargo-ndk` (3 sitios), `tool: cargo-audit`, `pipx run zizmor` ([HT-09]). |
| Límites de concurrencia | Ningún workflow tiene `concurrency:` (13 avisos Low). Recomendable en `ci.yml` y `release.yml` (`cancel-in-progress: false` en release). |
| Workflows muertos | `db-cipher-e2e.yml`, `db-cipher-lab.yml` ([HT-07]). |
| Visibilidad de zizmor en CI | La persona por defecto suprime los 61 hallazgos ([HT-09]). |

Referencias: https://docs.github.com/en/actions/security-for-github-actions/security-guides/security-hardening-for-github-actions, https://docs.zizmor.sh/audits/

---

## 6. Inventario de código muerto

| Elemento | Ubicación | Evidencia de que no se usa | Acción sugerida |
|---|---|---|---|
| Módulo `lenguaje_comandos` (parser, resolver, contexto; 1 430 líneas, 38 tests) | `src/lenguaje_comandos/`, `src/lib.rs:10` | 0 imports fuera del módulo (grep en raíz, desktop, mobile, tests). El propio comentario de `lib.rs:6-9` lo declara huérfano. | **Eliminar** (ya está en la rama `archive/cli-tui-2026-09-12`) |
| `AppCore::buscar_auditoria` | `src/application/catalogos.rs:88` | Único llamador de producción: `lenguaje_comandos/resolver.rs:319` (muerto). Solo lo usan además 7 tests. | Eliminar junto con el anterior (y sus tests) o revisar |
| `AppCore::validar_datos_para_crear_usuario` | `src/application/usuarios.rs:92` | 0 referencias (grep en `.rs/.kt/.swift/.ts`). El doc-comment dice "TUI". | Eliminar |
| `AppCore::crear_usuario_con_hash` | `src/application/usuarios.rs:102` | 0 referencias | Eliminar |
| `AppCore::cambiar_password_usuario_con_hash` | `src/application/usuarios.rs:215` | 0 referencias | Eliminar |
| `AppCore::verificar_mi_password` | `src/application/usuarios.rs:242` | 0 referencias. El doc-comment dice "gate de `/clave` en la CLI". | Eliminar |
| `AppCore::preparar_cambio_password_propio` | `src/application/usuarios.rs:290` | 0 referencias. El doc-comment dice "La TUI usa...". | Eliminar |
| `AppCore::cambiar_mi_password_con_hash` | `src/application/usuarios.rs:311` | 0 referencias | Eliminar |
| `UsuarioService::cambiar_password_propio` | `src/services/usuario_service.rs:179` | 0 llamadas (solo aparece en comentarios) | Eliminar |
| `AppCore::gafete_provisional_ocupado_en_sitio` | `src/application/nube.rs:719` | 0 referencias. Escritorio llama directo a `nube::*` para no retener el candado. | Eliminar o revisar |
| `AppCore::gafete_de_proveedor_ocupado_en_sitio` | `src/application/nube.rs:751` | Solo aparece en un comentario de `desktop/.../comandos/proveedores.rs:23` que explica por qué *no* se usa | Eliminar |
| `abrir_conexion_secundaria` (solo lectura) | `src/database/connection.rs:199` | 0 llamadores. Solo se usa la variante `_escritura`. El doc-comment habla de "TUI/CLI". | Eliminar o revisar |
| `hora_actual_texto` | `src/tiempo.rs:116` | 0 referencias | Eliminar |
| `TipoIngreso::from_str_filtro`, `EstadoGafete::from_str_filtro`, `from_str_filtro` (segundo en gafete.rs) | `src/models/tipo_ingreso.rs:41`, `src/models/gafete.rs:34,85` | 0 llamadas (solo aparecen en comentarios) | Eliminar |
| 13 funciones `pub` usadas solo por tests (`resetear_password_root`, `listar_roots_activos`, `activar_usuario`, `desactivar_usuario`, `actualizar_usuario`, `cambiar_password_usuario`, `fijar_password_inicial`, `buscar_historial_completo`, `buscar_auditoria_completo`, `registrar_salida_por_gafete`, `UsuarioService::actualizar_administracion`, `dev_auth::usuario_desarrollo`, `dev_auth::actor_persistido`) | `src/application/usuarios.rs:39,56,126,165,173,181`, `src/application/autenticacion.rs:119`, `src/application/historial.rs:428`, `src/application/catalogos.rs:109`, `src/services/registro_ingreso_service.rs:398`, `src/services/usuario_service.rs:151`, `src/services/dev_auth.rs:15,28` | 0 usos en producción (núcleo, desktop, mobile); solo en `tests/` | Revisar: bajar a `pub(crate)`/`#[cfg(test)]` o eliminar con sus tests |
| `build.rs` + build-dependency `winresource` | `build.rs`, `Cargo.toml:81-82` | Depende de `CARGO_FEATURE_TERMINAL_UI`; la feature `terminal-ui` no existe. El crate no tiene binario. | Eliminar |
| Perfil `release-native` + alias `build-native` | `Cargo.toml:164-165`, `.cargo/config.toml:2` | El crate raíz no tiene target binario; `target/release-native/control_acceso.exe` no se puede generar | Eliminar o mover a desktop |
| Rama `#[cfg(not(windows))]` de `abrir_nucleo_con_recuperacion` + imports `Arc`/`RelojCorregido` | `desktop/src-tauri/src/lib.rs:2,8,251-255` | No compila (E0425/E0432) | Revisar ([HT-05]) |
| Dependencia `serde_json` | `desktop/src-tauri/Cargo.toml` | `cargo machete` + grep sin coincidencias | Eliminar |
| Workflows `db-cipher-e2e.yml`, `db-cipher-lab.yml` | `.github/workflows/` | Apuntan a `experiments/`, que no existe y está en `.gitignore` | Eliminar |
| Referencias a `experiments/db-cipher-lab/README.md` | `sqlite3mc-vendor-lib/build.rs:19`, `sqlite3mc-vendor-lib/Cargo.toml`, `benchmarks/sqlite-3way/sqlite3mc/Cargo.toml` | Ese archivo no existe | Corregir comentarios |
| `packaging/msix/` (procedimiento MSIX) | `packaging/msix/README.md`, `AppxManifest.xml` | Empaqueta `target/release/control_acceso.exe`, que ya no existe (Tauri genera MSI/NSIS) | Revisar / eliminar |
| Secciones TUI/respaldos/`--reset-root` del README | `README.md:28-40,141-165` | No existe ni binario ni respaldos (`plan-qa` punto 4) | Reescribir |
| Comentario de `profile.production` | `Cargo.toml:166-178` | Cita `formulario.rs` (no existe) y el perfil no aplica a desktop/mobile | Corregir |
| `#[allow(dead_code)]` en `FilaGafeteOcupado::id`, `ClaimsToken::exp` | `src/nube/sincronizacion.rs:2138`, `src/nube/auth_supabase.rs:239` | Campos deserializados que no se leen | Cambiar a `#[expect(dead_code, reason=...)]` |
| `benchmarks/sqlite-3way/` y `examples/benchmark_3way.rs` | `benchmarks/`, `examples/` | No es código muerto: sigue referenciado por `docs/decisiones-tecnicas.md` y la sección 12 de la auditoría de rendimiento, y compila con clippy. La decisión de motor ya está tomada (SQLite3MC). | Revisar: conservar como smoke test o archivar |
| `scripts/activar_sandbox.ps1`, `scripts/generar_secreto_dispositivo.mjs` | `scripts/` | **Se usan**: documentados en `docs/recuperacion-sitio-staging.md:224,267` | Conservar |
| `examples/importar_*.rs`, `probar_sincronizacion.rs` | `examples/` | Herramientas manuales documentadas. Compilan limpio. `importar_contratistas.rs:2-3` todavía menciona un "respaldo pre-migración" que ya no existe. | Conservar; corregir el comentario |
| `sqlite3mc-vendor-lib/` | raíz | **Se usa**: lo exige la feature `cifrado-sqlite3mc`, default de desktop y del release Android | Conservar |

Módulos `.rs` huérfanos (archivos no declarados con `mod`): **ninguno** en `src/`, `desktop/src-tauri/src` ni `mobile/rust-core/src`. `todo!`/`unimplemented!`: **0**.
