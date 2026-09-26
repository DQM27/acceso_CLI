# Handoff -- Realtime en Rust (`lattis_realtime_spike`)

Este documento es para quien (persona o sesión de Claude) continúe este
trabajo si la sesión que lo escribió se corta. `README.md` (mismo
directorio) tiene el detalle etapa por etapa del laboratorio original, con
snippets de código y fuentes citadas; este documento es el resumen
ejecutivo + "qué hacer después".

Rama: `claude/realtime-rust-spike`. Todo el trabajo vive ahí, nunca en
`main`/`master` (pendiente: PR + merge cuando el usuario lo pida).

## TL;DR -- en qué estado está esto

**2026-09-26: decisión de arquitectura tomada -- REEMPLAZA, no coexiste.**
El usuario decidió que este cliente Rust reemplaza por completo a
`supabase-js` (desktop) y `io.github.jan.supabase.realtime` (mobile) en vez
de correr en paralelo como shadow-run -- "para qué mantener el viejo si ya
tengo este" fue el razonamiento, y coexistir con un interruptor para elegir
cuál usar se consideró sobreingeniería. Esto YA es el mecanismo real de
sync en tiempo real de la app, no un experimento con feature flag.

**Cliente Realtime (`benchmarks/realtime-rust/`):** Phoenix Channels
escrito desde cero en Rust -- heartbeat, reconexión con backoff+jitter,
canal privado con JWT de dispositivo, renovación de token sin reconectar,
Presence. Probado exhaustivamente contra el proyecto **sandbox**
`control-acceso-staging` (`pmrytjktlyiuikxuuxpr.supabase.co`), nunca
`control-acceso-nube` (producción).

**Desktop (`desktop/src-tauri/src/realtime_nube.rs`):** reemplaza a
`desktop/src/nubeRealtime.ts` (que ya NO usa `@supabase/supabase-js`, se
sacó del `package.json`). Reusa `GuiState::autenticar_con_cache` para el
JWT (nunca reimplementa `device-auth`), arma el topic real
`realtime:sitio:<sitio_id>`, escucha el evento real `cambio_nube`, filtra
el eco del propio dispositivo, hace debounce de 600ms y llama a la MISMA
`ejecutar_sincronizacion` que ya usan el pulso periódico y el botón manual
-- cero lógica de sync duplicada. Dos comandos Tauri nuevos
(`iniciar_realtime_nube`/`detener_realtime_nube`) reemplazan a
`sesion_realtime_nube`. `nubeRealtime.ts` mantiene la misma interfaz
pública (`iniciarRealtimeNube`/`emitirActualizacion`), así que
`App.tsx`/`BarraNube.tsx` no cambiaron.

**Mobile (`mobile/rust-core/src/realtime_nube.rs`):** reemplaza a
`NubeRealtime.kt` (que ya NO usa `io.github.jan.supabase.realtime`). Sin
`GuiState` equivalente en este crate, el JWT se resuelve del lado Kotlin
(`callback_interface ProveedorTokenRealtimeNube` delegando a
`Nucleo::sesion_realtime_nube_con_secreto`, el mismo método que ya usaba
`NubeRealtime.kt`) -- nunca toca `Nucleo`/`AppCore` directamente. Corre en
su propio `Runtime` de `tokio` (`TareaRealtimeNube`, un `uniffi::Object`
con `detener()`), no comparte runtime con el resto de la app.
`NubeRealtime.kt` mantiene la misma interfaz pública (`iniciar()`/
`detener()`), así que `PantallaPrincipal.kt` no cambió --
`SincronizacionPeriodica` (debounce + serialización real) tampoco cambió,
sigue siendo quien de verdad descarga los cambios.

**Validado en vivo, ambas plataformas:** dispositivos descartables reales
en `control-acceso-staging`, JWT real de `device-auth`, un cambio real
(`INSERT` en `empresas`) disparó el trigger real de producción
(`empresas_emitir_cambio_nube`) y el broadcast llegó completo a través de
la interfaz UniFFI/Tauri completa (no sólo el mecanismo de bajo nivel).
Todo lo descartable ya se limpió de staging.

**Compilado para Android real (2026-09-26):** `cargo ndk` para
`aarch64-linux-android`/`x86_64-linux-android` (nunca antes probado en este
repo) y un APK de debug completo instalable. Compilado en modo
`sqlite-plano` -- `cifrado-sqlcipher` (el motor real) no pudo cruzar-
compilar OpenSSL en esta máquina (falta el módulo Perl
`ExtUtils::MakeMaker`, limitación de esta PC, no del código). **Nunca se
instaló/corrió en un emulador o teléfono real** -- el usuario decidió
reemplazar ya y probarlo él mismo después, sin bloquear el reemplazo por
esa validación.

**Pendiente real:** ver la sección "Lo que falta" más abajo -- sobre todo
el punto 6 (probar en un dispositivo/emulador real) y limpiar referencias
muertas (`sesion_realtime_nube` del lado desktop ya se borró; revisar si
`io.github.jan.supabase` sigue en `build.gradle.kts` de mobile como
dependencia sin uso).

## Restricciones que SIEMPRE aplican (no negociables)

1. **Nunca tocar `control-acceso-nube`** (el proyecto de producción de
   Supabase). Todo esto se prueba contra `control-acceso-staging`
   (`pmrytjktlyiuikxuuxpr.supabase.co`) -- ver
   `docs/recuperacion-sitio-staging.md` para credenciales/contexto de ese
   proyecto.
2. **Cualquier objeto descartable creado en staging para probar** (sitios,
   dispositivos, filas de prueba, triggers/funciones temporales) se limpia
   (`DELETE`/`DROP`) después de cada prueba -- sin dejar basura.
3. **Nunca usar el término prohibido para el puesto de control** (ver
   `AGENTS.md` en la raíz del repo) -- usar "puesto de control", "portería"
   o "punto de acceso".
4. **Comunicarse siempre en español** (`AGENTS.md`).
5. **Commit después de cada cambio exitoso** (`AGENTS.md`) -- commits bien
   documentados, en español, explicando qué se probó y qué se encontró
   (mismo estilo que el historial de esta rama). **Nunca push sin que el
   usuario lo pida explícitamente.**
6. **Nunca generar ni pedir credenciales de un dispositivo de
   producción real.** Si en algún momento el trabajo requiere eso (ver
   "Apuntar a producción" más abajo), es una decisión que le corresponde
   al usuario, no algo que se resuelve solo.

## Mapa de archivos

| Qué | Dónde |
|---|---|
| El cliente Realtime en sí (protocolo, cliente WS, supervisor, backoff, Presence) | `benchmarks/realtime-rust/src/` |
| Binarios de prueba manual contra staging | `benchmarks/realtime-rust/src/bin/smoke_*.rs` |
| Tests (unitarios, propiedades, integración, e2e) | `benchmarks/realtime-rust/src/*.rs` (`mod tests`/`mod propiedades`) y `benchmarks/realtime-rust/tests/` |
| Historia completa etapa por etapa, con fuentes citadas | `benchmarks/realtime-rust/README.md` |
| Este documento | `benchmarks/realtime-rust/HANDOFF.md` |
| Mecanismo real de escritorio (reemplazó a `nubeRealtime.ts` con `supabase-js`) | `desktop/src-tauri/src/realtime_nube.rs`, `desktop/src/nubeRealtime.ts` |
| Mecanismo real de mobile (reemplazó a `NubeRealtime.kt` con `io.github.jan.supabase`) | `mobile/rust-core/src/realtime_nube.rs`, `mobile/android/app/src/main/java/com/brisas/controlacceso/NubeRealtime.kt` |
| El código real de producción de la app (NUNCA tocar sin que haga falta) | `comandos::nube` en `desktop/src-tauri/src/comandos/nube.rs`, `AppCore`/`Nucleo` en el crate raíz y `mobile/rust-core` |
| Edge Function que emite el JWT de dispositivo | `supabase/functions/device-auth/index.ts` |
| Migraciones que definen el esquema/políticas de Realtime reales | `supabase/migrations/*cambio_nube*.sql`, `*emitir_cambio_nube*.sql` |

## Cómo retomar: credenciales y comandos de staging

Proyecto: `control-acceso-staging` -- URL base
`https://pmrytjktlyiuikxuuxpr.supabase.co` (ver
`docs/recuperacion-sitio-staging.md` para el resto de credenciales).

Dispositivo descartable real, de punta a punta (mismo patrón usado varias
veces en esta rama):

```sh
node scripts/generar_secreto_dispositivo.mjs --sitio "<nombre de prueba>" --tipo pc|mobile --etiqueta "<algo descartable>"
# correr el SQL que imprime contra el project_id pmrytjktlyiuikxuuxpr (MCP de Supabase o dashboard)

curl -s -X POST "https://pmrytjktlyiuikxuuxpr.supabase.co/functions/v1/device-auth" \
  -H "Content-Type: application/json" -d '{"secret":"<el secret de arriba>"}'
# devuelve access_token/sitio_id/dispositivo_id reales
```

Con un JWT real se puede correr cualquiera de los tests manuales (todos
"saltan solos" sin fallar si faltan las variables de entorno):

```sh
# Laboratorio puro (sin JWT, canal público)
REALTIME_WS_URL="wss://pmrytjktlyiuikxuuxpr.supabase.co/realtime/v1/websocket" \
REALTIME_APIKEY="<publishable key>" \
cargo run --manifest-path benchmarks/realtime-rust/Cargo.toml --bin smoke_heartbeat

# Canal privado real (mismo binario que ya probó Etapa 4)
REALTIME_WS_URL="..." REALTIME_APIKEY="..." REALTIME_DEVICE_JWT="..." REALTIME_SITIO_ID="..." \
cargo run --manifest-path benchmarks/realtime-rust/Cargo.toml --bin smoke_supervisor_privado

# Mobile -- el puente real completo (callback_interface incluido)
REALTIME_BASE_URL="https://pmrytjktlyiuikxuuxpr.supabase.co" REALTIME_APIKEY="..." \
REALTIME_DEVICE_JWT="..." REALTIME_SITIO_ID="..." REALTIME_DISPOSITIVO_ID="..." \
cargo test --manifest-path mobile/rust-core/Cargo.toml --no-default-features --features sqlite-plano \
  --lib realtime_nube::tests::se_une_al_canal_privado_real_y_llama_al_observador_ante_un_cambio_remoto -- --nocapture
```

Para forzar un broadcast `cambio_nube` real mientras corre alguno de estos
(y ver que de verdad llega), insertar/actualizar una fila en cualquier
tabla con el trigger `*_emitir_cambio_nube` (`empresas` es la más simple)
para el `sitio_id` del dispositivo de prueba -- **limpiar después**.

### Compilar de verdad (Windows + Android)

```sh
# Desktop -- motor plano (nunca cifrado-sqlcipher/sqlite3mc para iterar)
cargo build --manifest-path desktop/src-tauri/Cargo.toml --no-default-features --features sqlite-plano
cargo clippy --manifest-path desktop/src-tauri/Cargo.toml --no-default-features --features sqlite-plano --all-targets

# Mobile -- host (tests JVM)
cargo build --manifest-path mobile/rust-core/Cargo.toml --no-default-features --features sqlite-plano

# Mobile -- target real de Android (requiere NDK + cargo-ndk, ya instalados en esta máquina)
cd mobile/rust-core
cargo ndk -t aarch64-linux-android build --release --no-default-features --features sqlite-plano
cargo ndk -t x86_64-linux-android build --release --no-default-features --features sqlite-plano
# copiar los .so a mobile/android/app/src/main/jniLibs/{arm64-v8a,x86_64}/
# regenerar bindings si cambió la API pública (ver mobile/README.md paso 2)
# y copiarlos a mobile/android/app/src/main/java/uniffi/control_acceso_mobile/ (SÍ se commitea, confirmado en este repo)
cd ../android && ./gradlew assembleDebug
```

**Para lanzar la app de escritorio en el sandbox:**
`. .\scripts\activar_sandbox.ps1` (con el punto y espacio) y después
`npm run tauri dev` desde `desktop/`, en la MISMA terminal.

## Lo que falta, en orden

### 1. Canal privado real con JWT + datos reales -- CERRADO (2026-09-26), desktop y mobile

Ver el TL;DR de arriba.

### 2-3. Conectar frontend/nativo de verdad -- CERRADO (2026-09-26), reemplazo completo

Ya no son "sólo loguear" -- son el mecanismo real. Ver TL;DR.

### 4. Shadow-run comparativo -- DESCARTADO por decisión explícita del usuario

Se iba a comparar el mecanismo viejo vs. el nuevo en paralelo antes de
decidir. El usuario saltó directo a reemplazar (ver TL;DR, "para qué
mantener el viejo si ya tengo este") -- no hay comparación, el mecanismo
viejo ya no existe en el árbol.

### 5. Decisión de arquitectura -- CERRADA: REEMPLAZA

Ver TL;DR.

### 6. Validar de verdad contra Android/iOS -- PARCIAL

**Cerrado:** compila contra el target real (`aarch64-linux-android`,
`x86_64-linux-android`) vía `cargo ndk` -- nunca se había hecho en este
repo antes de esta sesión. APK de debug (`sqlite-plano`) compilado,
instalable.

**Pendiente:** nunca se instaló/corrió en un emulador o teléfono real --
sólo se validó el `.so`/bindings compilando y un test de host (JVM) contra
staging real. El usuario decidió probarlo él mismo (tiene el APK en su
escritorio, `control-acceso-debug-realtime.apk`) en vez de bloquear el
reemplazo por esto.

iOS: sigue sin ningún archivo Realtime -- fuera de alcance, necesitaría un
Mac/Xcode.

### 7. Apuntar a producción (el último paso, no el próximo)

Sigue sin tocarse -- todo lo de arriba fue contra `control-acceso-staging`.
Requiere:
- Un JWT/secreto de un dispositivo de **producción real** -- nunca
  generarlo ni pedirlo sin que el usuario decida explícitamente cómo se
  maneja esa credencial.
- Confirmar con el usuario, de nuevo, antes de ejecutar nada contra
  `control-acceso-nube`.
- Este PR/rama tiene que mergearse a `main` primero (nunca se hizo --
  el usuario dijo explícitamente "nunca hago push por mi cuenta" como
  regla general, así que sigue sin subirse el trabajo de esta sesión más
  allá del propio origin de la rama).

## Qué NO hacer

- No reimplementar la obtención de JWT de dispositivo -- reusar
  `GuiState::autenticar_con_cache` (desktop) o
  `Nucleo::sesion_realtime_nube_con_secreto` vía callback (mobile).
- No asumir que "ya se puede ir a producción" sin que el usuario lo diga
  explícitamente, ni generar/pedir credenciales de un dispositivo de
  producción por cuenta propia.
- No hacer `git push` sin que el usuario lo pida explícitamente (memoria
  del usuario: commitear está bien solo, pushear no).
