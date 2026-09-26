# Handoff -- laboratorio Realtime en Rust (`lattis_realtime_spike`)

Este documento es para quien (persona o sesión de Claude) continúe este
trabajo si la sesión que lo escribió se corta. Es el punto de entrada --
`README.md` (mismo directorio) tiene el detalle etapa por etapa, con
snippets de código y fuentes citadas; este documento es el resumen
ejecutivo + "qué hacer después", para no tener que releer 600+ líneas de
README para saber por dónde seguir.

Rama: `claude/realtime-rust-spike`. Todo el trabajo vive ahí, nunca en
`main`/`master`.

## TL;DR -- en qué estado está esto

**Probado y sólido:** un cliente de Supabase Realtime (Phoenix Channels)
escrito desde cero en Rust (`benchmarks/realtime-rust/`), con heartbeat,
reconexión con backoff+jitter, canal privado con JWT de dispositivo,
renovación de token sin reconectar, Presence, y `broadcast_changes` (fila
completa por WebSocket, sin round-trip REST extra) -- todo probado contra
el proyecto **sandbox** `control-acceso-staging`
(`pmrytjktlyiuikxuuxpr.supabase.co`), nunca `control-acceso-nube`
(producción).

**Integrado, pero a medias:** ese cliente ya compila e integra de verdad
en `desktop/src-tauri` (Windows real) y `mobile/rust-core` (host +
transitivo Android), detrás de una feature `lattis-realtime-experimental`
apagada por defecto. Hay un puente mínimo (evento `Tauri` / callback
`UniFFI`) que **ya se probó conectado a staging de verdad** -- pero sólo
hace **heartbeat público, sin autenticar, sin datos reales**. Nadie del
lado frontend (`App.tsx`) ni nativo (Kotlin/Swift) lo escucha todavía.

**No existe todavía:** un reemplazo real de `nubeRealtime.ts`
(desktop/web) ni de `NubeRealtime.kt` (Android). Esto sigue siendo un
laboratorio que prueba que el mecanismo de bajo nivel es viable -- no un
producto terminado. "Apuntar a producción" NO es el último paso: hay una
lista completa de trabajo real antes de eso (ver más abajo).

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
5. **Commit + push después de cada cambio exitoso** (`AGENTS.md`) --
   commits bien documentados, en español, explicando qué se probó y qué
   se encontró (mismo estilo que el historial de esta rama).
6. **Nunca generar ni pedir credenciales de un dispositivo de
   producción real.** Si en algún momento el trabajo requiere eso (ver
   "Apuntar a producción" más abajo), es una decisión que le corresponde
   al usuario, no algo que se resuelve solo.

## Mapa de archivos

| Qué | Dónde |
|---|---|
| El cliente Realtime en sí (protocolo, cliente WS, supervisor, backoff) | `benchmarks/realtime-rust/src/` |
| Binarios de prueba manual contra staging | `benchmarks/realtime-rust/src/bin/smoke_*.rs` |
| Tests (unitarios, propiedades, integración, e2e) | `benchmarks/realtime-rust/src/*.rs` (`mod tests`/`mod propiedades`) y `benchmarks/realtime-rust/tests/` |
| Historia completa etapa por etapa, con fuentes citadas | `benchmarks/realtime-rust/README.md` |
| Este documento | `benchmarks/realtime-rust/HANDOFF.md` |
| Puente hacia desktop (evento Tauri) | `desktop/src-tauri/src/lattis_experimental.rs` (feature `lattis-realtime-experimental` en `desktop/src-tauri/Cargo.toml`) |
| Puente hacia mobile (callback UniFFI) | `mobile/rust-core/src/lattis_experimental.rs` (misma feature en `mobile/rust-core/Cargo.toml`) |
| El código real que esto podría llegar a reemplazar (desktop/web) | `desktop/src/nubeRealtime.ts` |
| El código real que esto podría llegar a reemplazar (Android) | `mobile/android/app/src/main/java/com/brisas/controlacceso/NubeRealtime.kt` |
| El código real de producción de la app (NUNCA tocar sin que haga falta) | `comandos::nube` en `desktop/src-tauri/src/comandos/nube.rs`, `AppCore` en el crate raíz |
| Edge Function que emite el JWT de dispositivo | `supabase/functions/device-auth/index.ts` |
| Migraciones que definen el esquema/políticas de Realtime reales | `supabase/migrations/*realtime*.sql`, `*presencia*.sql` |

## Cómo retomar: credenciales y comandos de staging

Proyecto: `control-acceso-staging` -- URL base
`https://pmrytjktlyiuikxuuxpr.supabase.co` (ver
`docs/recuperacion-sitio-staging.md` para el resto de credenciales, sólo
la publishable key hace falta para lo de abajo).

```sh
export REALTIME_WS_URL="wss://pmrytjktlyiuikxuuxpr.supabase.co/realtime/v1/websocket"
export REALTIME_APIKEY="<publishable key de control-acceso-staging, ver docs/recuperacion-sitio-staging.md>"
```

Con eso:

```sh
# Smoke test más simple -- heartbeat público, sin JWT.
cargo run --manifest-path benchmarks/realtime-rust/Cargo.toml --bin smoke_heartbeat

# El test real de shadow-run del puente mobile (el que sí se puede EJECUTAR
# en un host Linux normal, a diferencia del de desktop que sólo se pudo
# compilar para Windows en la última sesión por falta de un runner de
# binarios .exe -- ver README.md, "Etapa 4 -- el shadow-run probado...").
cd mobile/rust-core && cargo test --features lattis-realtime-experimental \
  el_shadow_run_conecta_a_staging_y_llama_al_observador_real -- --nocapture
```

Para un canal PRIVADO real (con JWT de dispositivo) hace falta además
crear un dispositivo/sitio descartable en staging y llamar a
`device-auth` -- ver `benchmarks/realtime-rust/src/bin/smoke_private_channel.rs`
(tiene el flujo documentado paso a paso en su doc-comment) y
**acordarse de limpiar (`DELETE`) esas filas después**.

Toolchains ya instalados en una sandbox de trabajo típica de este repo
(pueden no estar en una nueva -- reinstalar si hace falta, son pasos
baratos vía `apt`/`rustup`):

```sh
# Para poder compilar de verdad desktop-tauri (target real, Windows):
rustup target add x86_64-pc-windows-gnu
sudo apt-get install -y gcc-mingw-w64-x86-64
sudo update-alternatives --set x86_64-w64-mingw32-gcc /usr/bin/x86_64-w64-mingw32-gcc-posix
# ^ la variante "posix" es obligatoria -- la "win32" que trae el paquete
# por default no sirve para el `std` de Rust.

# Para poder compilar (no correr -- ver más abajo) desktop-tauri en Linux
# host (necesario para clippy/algunos checks, la app en sí es Windows-only):
sudo apt-get install -y libgtk-3-dev libwebkit2gtk-4.1-dev \
  libjavascriptcoregtk-4.1-dev libsoup-3.0-dev \
  libayatana-appindicator3-dev librsvg2-dev
```

## Lo que falta, en orden, con pasos concretos

### 1. Canal privado real con JWT + datos reales en el puente (el más importante)

Hoy `lattis_experimental.rs` (desktop y mobile) sólo hace
`supervisar_heartbeat` contra el topic público `"phoenix"`, sin
autenticar. La producción real necesita el canal PRIVADO por sitio
(`sitio:<sitio_id>`, ver `supabase/migrations/*realtime*.sql`) con JWT de
dispositivo, y `broadcast_changes` con la fila completa (no sólo un
aviso vacío) -- ambos mecanismos YA están probados y funcionando en el
laboratorio (`supervisar_canal_privado`, `ClienteRealtime::unirse_privado`,
ver `README.md` Etapas 2 y 3.5), sólo falta conectarlos al puente.

**Fricción real a resolver, no un simple copy-paste:**
`supervisar_canal_privado` pide `obtener_token_fresco: impl FnMut() ->
String` -- una función SÍNCRONA que devuelve un JWT válido cada vez que
hace falta (al conectar y, opcionalmente, en cada renovación proactiva).
En la app real, obtener un JWT de dispositivo implica una llamada HTTP a
`device-auth` (ver `supabase/functions/device-auth/index.ts`) usando el
secreto del dispositivo -- lo cual hoy ya lo hace `control_acceso::nube`
en algún lado del crate raíz. Antes de escribir código hay que decidir:

- **Opción A (recomendada):** el puente NO reimplementa la obtención del
  JWT -- recibe un token ya vigente (y una forma de refrescarlo) desde el
  código que lo llama (`configurar_arranque` en desktop,
  quien-invoque-la-función-UniFFI en mobile), que a su vez se lo pide al
  módulo `nube` real (que YA sabe cómo hacerlo, con su propio cacheo).
  Evita duplicar la lógica de auth de dispositivo en dos lugares.
- **Opción B:** el puente reimplementa su propia llamada HTTP a
  `device-auth` con `reqwest` (agregar la dependencia), independiente del
  módulo `nube`. Más simple de escribir, pero duplica lógica real de
  producción (secreto del dispositivo, endpoint, manejo de errores) en un
  módulo "experimental" -- riesgo de que las dos copias diverjan.

Recomendación: A. Requiere mirar `control_acceso::nube` (crate raíz) para
ver qué función expone hoy para pedir/cachear un JWT de dispositivo, y
si se puede llamar desde `desktop/src-tauri`/`mobile/rust-core` sin pasar
por `AppCore` completo (que trae sesión de usuario, DB, etc. -- de más
para esto).

**Para probarlo contra staging sin tocar producción:** crear un sitio +
dispositivo descartable (mismo patrón que
`benchmarks/realtime-rust/src/bin/smoke_private_channel.rs`), llamar a
`device-auth` para conseguir un JWT real, usar `supervisar_canal_privado`
con `ConfigCanalPrivado { topic: format!("sitio:{sitio_id}"), .. }`, y
un trigger temporal con `realtime.broadcast_changes(...)` sobre una tabla
de prueba (o una tabla real, con cuidado de no afectar datos reales) --
limpiar todo (dispositivo, sitio, trigger) al terminar.

### 2. Conectar el frontend de verdad (desktop)

`desktop/src/App.tsx` ya escucha `"nube://sincronizado"`
(`listen<ResumenSincronizacion>(...)`, línea ~563). Agregar, sólo si se
decide seguir con esto en serio (es código real de UI, no del
laboratorio):

```ts
listen<{ tipo: string; detalle: string }>("lattis://experimental", ({ payload }) => {
  console.debug("[lattis experimental]", payload.tipo, payload.detalle);
});
```

Empezar SOLO con un `console.debug` (observación pura, sin actuar sobre
el evento) hasta decidir qué hace React con esto de verdad.

### 3. Conectar el lado nativo de verdad (mobile)

Implementar `ObservadorLattisExperimental` en Kotlin
(`mobile/android/app/src/main/java/...`) y llamar a
`iniciar_shadow_run_lattis_experimental` desde algún punto de arranque de
la app -- mismo criterio que el punto 2: empezar sólo logueando
(`Log.d`), no actuando. iOS ni siquiera tiene todavía un archivo
Realtime -- si se llega a este punto, probablemente haga falta escribirlo
desde cero del lado Swift (fuera del alcance de este laboratorio Rust).

### 4. Shadow-run real (comparar, no sólo observar)

El "shadow-run" de verdad (el término que se usó desde el principio en
este trabajo) significa correr el mecanismo nuevo EN PARALELO al sync
real y comparar: ¿llegó el mismo dato? ¿más rápido? ¿algún caso donde uno
funciona y el otro no? Hoy sólo se prueba que conecta y loguea -- no hay
ninguna lógica de comparación. Esto requeriría, como mínimo:

- Loguear (local, o a Sentry como ya hace el resto de la app) cuándo
  llega un evento por el canal viejo (`"nube://sincronizado"`, disparado
  por el pulso de 2 minutos o por `cambio_nube` de Realtime Broadcast) vs.
  cuándo llegaría el mismo cambio por el canal nuevo
  (`broadcast_changes`).
- Decidir una ventana de tiempo razonable de shadow-run (¿días? ¿semanas?)
  antes de siquiera considerar un corte real.

### 5. Decisión de arquitectura (le corresponde al usuario, no a quien continúe esto solo)

Antes de ir más lejos, hay una pregunta sin responder: ¿esto REEMPLAZA
`nubeRealtime.ts`/`NubeRealtime.kt` (uno de los dos gana), o COEXISTEN
(Realtime en Rust sólo para notificación rápida, el resto del sync sigue
igual)? La respuesta cambia bastante el diseño de los puntos 2-4. Si
quien continúa esto no tiene esa respuesta, preguntarle al usuario antes
de comprometerse a una de las dos rutas.

### 6. Validar de verdad contra Android/iOS

`mobile/rust-core` compila para host y resuelve (vía `cargo metadata`)
para Android, pero nunca se compiló contra el target real
(`aarch64-linux-android`) -- falta el NDK (el `apt` de una sandbox típica
sólo tiene versiones viejas, r10e-r19c; lo ideal es NDK r25+ vía
`sdkmanager` si hay Android SDK disponible, o descargarlo directo de
`https://developer.android.com/ndk/downloads`) y configurar el linker
(`cargo-ndk` es la forma más simple: `cargo install cargo-ndk`, después
`cargo ndk -t arm64-v8a build`). iOS: ni siquiera hay hoy un
`build-rust-xcframework.sh` corrido con esta feature -- necesitaría un
Mac o un entorno con Xcode, no disponible en una sandbox Linux típica.

### 7. Apuntar a producción (el último paso, no el próximo)

Sólo después de 1-6. Requiere:
- Un JWT/secreto de un dispositivo de **producción real** -- nunca
  generarlo ni pedirlo sin que el usuario decida explícitamente cómo se
  maneja esa credencial (¿un dispositivo de prueba dedicado en
  producción? ¿uno real prestado temporalmente?).
- Confirmar con el usuario, de nuevo, antes de ejecutar nada contra
  `control-acceso-nube` -- la primera vez que se preguntó (esta misma
  sesión), la respuesta fue seguir en staging. No asumir que eso cambió.

## Qué NO hacer

- No reimplementar la obtención de JWT de dispositivo en el puente sin
  antes mirar qué ya existe en `control_acceso::nube` (evitar duplicar
  lógica de auth real).
- No tocar `App.tsx`/Kotlin de forma que ACTÚE sobre el evento (dispare un
  sync, cambie estado de UI) antes de tener claro el punto 5
  (reemplazo vs. coexistencia) -- empezar sólo observando/logueando.
- No asumir que "ya se puede ir a producción" sin que el usuario lo diga
  explícitamente, ni generar/pedir credenciales de un dispositivo de
  producción por cuenta propia.
- No instalar un NDK/SDK de Android completo sólo para validar esta
  feature experimental si el costo (tiempo, espacio en disco) no se
  justifica todavía -- documentar como pendiente es una respuesta válida
  (así se dejó en la última sesión).
