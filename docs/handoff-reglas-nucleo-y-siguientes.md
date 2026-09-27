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
| `claude/dominio-escritorio` | **Trabajar aquí.** Sale de la anterior; mismas reglas para escritorio |
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

## Paso 1: terminar reglas en `claude/dominio-escritorio`

Detalle en `docs/auditorias/reglas-duplicadas-escritorio-2026-09-27.md`.

1. **Proveedores (escritorio):** `desktop/src-tauri/src/comandos/proveedores.rs`
   orquesta chequeos de nube por su cuenta. Llevarlo al núcleo
   (`src/application/proveedores.rs` o un helper en `src/nube`) para que lo
   usen Tauri y `mobile/rust-core` (`registrar_ingreso_proveedor_con_secreto`).
   Primer chequeo: `AppCore::proveedor_con_ingreso_activo_en_sitio`.
   Cuidado: no retener el candado del núcleo durante llamadas HTTP.
2. **Gafete KOF (escritorio):** igual con
   `comandos/gafetes_provisionales.rs` (mismo patrón que
   `entregar_gafete_provisional_con_secreto` del móvil).
3. **Aviso "ACCESO DENEGADO" / "PRAIND VENCIDO" en listas:** hoy lo calculan
   `desktop/src/pantallas/NuevoIngresoModal.logica.ts` (`avisosContratista`,
   dice "Sin acceso") y `FilasActivos.kt`, cada uno con su reloj. Hacer que
   el núcleo lo devuelva en `ContratistaResumen` (texto listo) y que ambos
   sólo lo muestren.
4. Borrar del doc de duplicados cada punto resuelto.

## Paso 2: rama nueva `claude/requiere-gafete` (desde `main` ya con paso 1)

Reemplazar "personal de ruta" por la casilla **"requiere gafete"**:
- Casilla por contratista; la puede cambiar cualquiera.
- El PRAIND NO depende de esto (sólo del tipo).
- Toca esquema local, migración, sincronización con Supabase, escritorio,
  móvil y panel web (~50 archivos). Migración probada **sólo en staging**;
  producción la aplica el dueño. Presentar plan al dueño antes de empezar.

## Paso 3: refactor del núcleo (`docs/auditorias/auditoria-nucleo-rust-2026-09-27.md`)

Rama nueva `claude/nucleo-n1-n3`:
- **N1 (crítica):** la orquestación de la sincronización está copiada 4
  veces → un solo lugar.
- **N2 (alta):** el móvil hace red con el candado del núcleo tomado.
- **N3 (alta):** el historial local crece para siempre.
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
- Disco chico: si cargo falla con "No space left", borrar
  `target/debug/incremental` y `mobile/rust-core/target/debug`.
- Estado al cerrar esta sesión: núcleo 664, rust-core 58, Android 219,
  vitest 245 tests, todo en verde.
