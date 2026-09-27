# Handoff: reglas en el núcleo y refactors siguientes

Para la sesión que continúa este trabajo. Leer completo antes de tocar nada.

## Reglas del dueño (obligatorias)

- `AGENTS.md`: comunicarse en español; nunca el término prohibido para el
  puesto de control ("puesto de control", "portería" o "punto de acceso");
  commit bien documentado + push por cada cambio exitoso.
- Respuestas cortas y claras. Decir qué se está haciendo.
- **Nada contra producción** (`control-acceso-nube`). Sólo staging
  (`control-acceso-staging`). Nunca generar ni pedir credenciales de
  dispositivo de producción. Limpiar en staging todo dato de prueba.
- Refactors en ramas separadas y en orden; no mezclar temas.
- Principio rector: **el núcleo Rust decide TODAS las reglas y los
  mensajes; escritorio (TS/Tauri) y móvil (Kotlin) sólo preguntan y
  muestran.** Si una interfaz decide algo, está mal.

## Ramas

| Rama | Estado |
|---|---|
| `main` | Tiene OCR (PR #77) y refactor móvil M1–M7 (PR #79) |
| `claude/dominio-en-nucleo` | Terminada. Reglas del móvil al núcleo |
| `claude/dominio-escritorio` | Paso 1 terminado. Sale de la anterior; mismas reglas para escritorio |
| `claude/nucleo-n1-n3` | N1–N3 del núcleo. Sale de `claude/dominio-escritorio` |
| `claude/pruebas-realtime-rust` | Rama de pruebas: `dominio-escritorio` + realtime en Rust (spike) + N1–N3 |
| `claude/realtime-rust-spike` | Cliente realtime en Rust. **No borrar** (ver paso 4) |
| `refactor-panel-web`, `claude/rediseno-web-visitas` | Otras sesiones. No tocar `web/` ni `web-visitas/` |

Al terminar el paso 1: traer `main` a `claude/dominio-escritorio`
(merge, no rebase) y abrir **una sola PR** a `main` que incluya las dos
ramas de dominio. Sólo abrir la PR si el dueño lo pide.

## Reglas ya acordadas (no reabrir)

Valen para escritorio y móvil, al crear y editar:
- PRAIND vencido se rechaza (al editar, sólo si cambia fecha, tipo o
  la casilla de ruta; si no, no se podría quitar el acceso).
- Personal de ruta sólo PRAIND / IN HOUSE.
- Alta de contratista siempre con acceso; se niega después.
- Nombre: sólo letras, en MAYÚSCULAS. Cédula: sólo dígitos.
- Cédula de proveedor activa: revisar este equipo y el otro dispositivo.
- Motivo de denegación: lo da el núcleo.
- Aviso en listas: **"ACCESO DENEGADO"** (el del móvil).
- PRAIND exigido a todos menos SWAT y POR CORREO.

## Paso 1: reglas en el núcleo — TERMINADO

Hecho en `claude/dominio-escritorio` (ver
`docs/auditorias/reglas-duplicadas-escritorio-2026-09-27.md`):
proveedores y gafete KOF verificados en `src/application/con_nube.rs`
(los usan Tauri y `mobile/rust-core`), y el aviso "ACCESO DENEGADO" /
"PRAIND VENCIDO" de las listas sale de `ContratistaResumen::aviso_acceso`.

Queda de este paso sólo la PR a `main`, si el dueño la pide.
`main` sólo tiene los merges de #77 y #79, sin contenido que falte acá.

## Paso 2: rama nueva `claude/requiere-gafete` (desde `main` ya con paso 1)

Reemplazar "personal de ruta" por la casilla **"requiere gafete"**:
- Casilla por contratista; la puede cambiar cualquiera.
- El PRAIND NO depende de esto (sólo del tipo).
- Toca esquema local, migración, sincronización con Supabase, escritorio,
  móvil y panel web (~50 archivos). Migración probada **sólo en staging**;
  producción la aplica el dueño. Presentar plan al dueño antes de empezar.

## Paso 3: refactor del núcleo — N1, N2 y N3 HECHOS en `claude/nucleo-n1-n3`

Detalle en `docs/auditorias/auditoria-nucleo-rust-2026-09-27.md`.
- **N1:** una sola sincronización (`nube::sincronizar`/`nube::recibir`,
  `src/nube/orquestacion.rs`) con `AlcanceSincronizacion` y
  `PerfilDispositivo`. El móvil no descarga ningún historial (decisión
  del dueño: el celular sólo registra; buscar es de la PC y la web).
- **N2:** nunca red con el candado del núcleo tomado; se fue el código
  "legado" del puente que lo hacía (Kotlin no lo usaba).
- **N3:** el escritorio guarda 24 meses de historial del sitio
  (`nube::retencion`); lo más viejo, en el panel web.
- N4–N8 después, si el dueño quiere.

## Paso 4: consolidar realtime (después de N1)

Comparar `claude/realtime-rust-spike` contra los clientes oficiales
(supabase-js en escritorio, realtime-kt en móvil), en staging, con
**llaves separadas** por equipo (Móvil prueba, PC prueba). Recordatorio:
el escritorio ignora eventos con su mismo `dispositivo_id`, así que una
llave compartida hace parecer que realtime "no anda". Presentar resultados
al dueño y decidir cuál queda.

## Cómo verificar (antes de cada push)

```bash
# Núcleo
cargo fmt && cargo clippy --all-targets -q && cargo test
# Puente móvil
cd mobile/rust-core && cargo fmt && cargo clippy --all-targets -q && cargo test
# Si cambió la API de rust-core: regenerar bindings Kotlin
cargo build --release
cargo run --features bindgen --bin uniffi-bindgen -- generate \
  --library target/release/libcontrol_acceso_mobile.so --language kotlin --out-dir /tmp/bind
cp /tmp/bind/uniffi/control_acceso_mobile/control_acceso_mobile.kt \
   ../android/app/src/main/java/uniffi/control_acceso_mobile/
# Android (borrar resultados viejos antes de contar)
cd ../android && export ANDROID_HOME=/opt/android-sdk
rm -rf app/build/test-results && ./gradlew testDebugUnitTest
# Escritorio
cd desktop && npx tsc --noEmit -p . && npx eslint src && npx vitest run
cd src-tauri && cargo check --target x86_64-pc-windows-gnu   # sólo compila para Windows
```

Trampas conocidas:
- **No usar Prettier**: el proyecto no lo usa y reformatea todo.
- **UniFFI incluye los doc-comments en el checksum**: cambiar sólo el
  comentario de una función exportada de `mobile/rust-core` obliga a
  regenerar los bindings, o la app falla al arrancar ("checksum mismatch").
- `sqlite3mc-vendor-lib/dist` guarda **un solo target**: compilarlo para
  Windows pisa el de Linux y la `.so` de los tests de Android queda sin
  SQLite (`undefined symbol: sqlite3_*`). Después de un chequeo para
  Windows: `touch sqlite3mc-vendor-lib/build.rs` y recompilarlo sin
  `--target`.
- Contenedor nuevo: hay que instalar mingw (`gcc-mingw-w64-x86-64`), el
  target `x86_64-pc-windows-gnu`, el SDK de Android en `/opt/android-sdk`
  (cmdline-tools + `platforms;android-36`, `build-tools;36.0.0`) y
  `npm ci` en `desktop/`. El núcleo con nube se prueba con
  `cargo test --features nube,cifrado-secreto-dispositivo-portable`.
- Disco chico: si cargo falla con "No space left", borrar
  `target/debug/incremental` y `mobile/rust-core/target/debug`.
- Estado al cerrar N1–N3: núcleo 667 (812 con nube), rust-core 58,
  Android 219, vitest 244 tests, todo en verde.
