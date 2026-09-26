# Laboratorio: cliente de Supabase Realtime en Rust puro

Rama de evaluación: `claude/realtime-rust-spike`. Igual que
`benchmarks/sqlite-3way/`, este directorio es un crate standalone (su
propio `Cargo.toml`/`Cargo.lock`) que **no toca** el `Cargo.toml`, el
código ni las dependencias del crate raíz `control_acceso`, ni
`desktop/src-tauri`, ni `mobile/rust-core`. Nada de acá se ejecuta en
producción hasta que se decida integrarlo de verdad.

## Por qué existe

Hoy el WebSocket de Realtime lo abren dos clientes independientes,
duplicados: `desktop/src/nubeRealtime.ts` (supabase-js) y
`mobile/android/.../NubeRealtime.kt` (Kotlin). La idea en evaluación es
reemplazar ambos por un único cliente Rust, compartido vía el workspace de
Cargo, que viva detrás del núcleo en vez del frontend. Contexto completo
en `docs/arquitectura/arquitectura-supabase.md` sección 4 (los tres
mecanismos de Realtime) y `docs/auditorias/realtime-verificado.md` (el
incidente de autorización ya resuelto).

No existe un cliente oficial de Supabase para Rust. Los comunitarios
(`supabase-realtime-rs`, `supabase-client-realtime`, `rp-supabase-realtime`)
tienen mantenimiento chico/incierto -- se decidió escribir el protocolo
Phoenix Channels a mano sobre `tokio-tungstenite` en vez de adoptar uno.

## Contra qué proyecto se prueba

**Únicamente `control-acceso-staging`** (`pmrytjktlyiuikxuuxpr`) -- nunca
`control-acceso-nube` (producción). Se verificó que staging ya tiene
replicadas las mismas políticas de `realtime.messages` que producción
(`dispositivos reciben broadcast de su sitio`, presencia leer/publicar),
así que es un sandbox fiel, no un simulacro con reglas distintas.

## Criterio de aprobación, por etapa

### Etapa 1 (esta) -- heartbeat contra el servidor real ✅ implementado

1. Conecta por WebSocket a `control-acceso-staging` con la `apikey` anon.
2. Manda `{topic: "phoenix", event: "heartbeat", ...}` -- el único mensaje
   del protocolo que Phoenix responde SIEMPRE, sin necesitar `phx_join` ni
   JWT de dispositivo (por eso es seguro probarlo sin tocar el flujo de
   auth real todavía).
3. Recibe `phx_reply` con `status: "ok"` para la misma `ref`.
4. Cierra limpio.

Correr:
```sh
REALTIME_WS_URL="wss://pmrytjktlyiuikxuuxpr.supabase.co/realtime/v1/websocket" \
REALTIME_APIKEY="<anon key de control-acceso-staging>" \
cargo run --manifest-path benchmarks/realtime-rust/Cargo.toml --bin smoke_heartbeat
```

Los tests unitarios (`cargo test --manifest-path benchmarks/realtime-rust/Cargo.toml`)
NO tocan la red -- levantan un servidor WebSocket local en
`127.0.0.1:0` que sólo entiende heartbeat, para probar el framing del
protocolo de forma determinística y reproducible en CI. El smoke test
contra el servidor real (`smoke_heartbeat`) es manual a propósito: no
tiene sentido en un pipeline de CI (depende de red externa y de un
proyecto Supabase real).

### Etapa 1.5 (esta) -- punta a punta real: Postgres → WebSocket → SQLite ✅ implementado

`bin/smoke_end_to_end.rs` cierra el círculo completo, con datos reales
(no simulados) en cada paso:

1. Se crean objetos DESCARTABLES en staging (tabla `_lab_lattis_avisos` +
   función `private.lab_lattis_emitir_aviso` + trigger), calcados del
   patrón real de `*_emitir_cambio_nube` (ver arquitectura-supabase.md 4.2)
   pero con `realtime.send(..., private => false)` -- canal PÚBLICO, sin
   política de `realtime.messages` de por medio, para no necesitar
   todavía el JWT de dispositivo (eso sigue siendo la Etapa 2).
2. El binario hace `phx_join` a `realtime:lab:lattis` (el prefijo
   `realtime:` es obligatorio del lado del cliente -- Postgres lo espera
   SIN el prefijo; sin este matiz, `phx_join` devuelve
   `{"reason":"unmatched topic"}`, primer error real encontrado).
3. Un `INSERT` real en `_lab_lattis_avisos` dispara el trigger.
4. El cliente recibe el broadcast -- segundo hallazgo real: un broadcast
   NO llega con tu nombre de evento en el campo `event` de nivel superior;
   Phoenix lo envuelve siempre como `event: "broadcast"`, con el nombre
   real y el payload real ANIDADOS adentro (`payload.event`/
   `payload.payload`). Así es como `supabase-js` implementa
   `.on("broadcast", { event: X }, cb)` por debajo -- documentado en el
   doc-comment de `ClienteRealtime::esperar_evento`.
5. El payload recibido se escribe en una SQLite LOCAL real (motor
   `sqlite-plano`/`rusqlite bundled`, sin cifrar -- sólo para este
   laboratorio) y se relee para confirmar que quedó persistido.
6. Los objetos de staging se borran al terminar -- no queda rastro.

Verificado a mano de punta a punta: `INSERT` en staging → fila visible en
`realtime.messages` → `phx_join` aceptado → broadcast recibido con el
envelope correcto → fila en `avisos_recibidos` de la SQLite local,
releída con éxito.

Los objetos de staging se borraron al cerrar esta etapa -- para repetir la
prueba hace falta recrearlos primero (SQL en el historial de este
laboratorio, no vive en el repo a propósito: son objetos descartables, no
una migración real del esquema). Uso del binario:

```sh
REALTIME_WS_URL="wss://pmrytjktlyiuikxuuxpr.supabase.co/realtime/v1/websocket" \
REALTIME_APIKEY="<anon key de control-acceso-staging>" \
cargo run --manifest-path benchmarks/realtime-rust/Cargo.toml --bin smoke_end_to_end
# en otra sesión, mientras el binario de arriba espera:
# INSERT INTO public._lab_lattis_avisos (mensaje) VALUES ('...');
```

### Etapa 2 (esta) -- `phx_join` a un canal privado real ✅ implementado

`bin/smoke_private_channel.rs`, con un dispositivo y un sitio DESCARTABLES
registrados a mano en staging (ver abajo), probó el flujo real completo:

1. `INSERT` directo en `public.dispositivos` con
   `secret_hash = sha256hex("<secreto de laboratorio>")` -- el mismo
   algoritmo que usa la Edge Function `device-auth` (confirmado leyendo su
   código fuente: `sha256Hex` sobre el `secret` en texto plano).
2. `POST` real a `.../functions/v1/device-auth` con ese secreto → devuelve
   un JWT ES256 real, firmado con la `DEVICE_SIGNING_KEY` de staging,
   `sitio_id` incluido.
3. `phx_join` a `realtime:sitio:<uuid del sitio de laboratorio>` con ese
   `access_token` -- **aceptado a la primera**: la política
   `"dispositivos reciben broadcast de su sitio"` de `realtime.messages`
   validó el JWT real sin ajustes.
4. Un `INSERT` real en `contratistas` (tabla con el trigger
   `contratistas_emitir_cambio_nube`), scoped al `sitio_id` del
   laboratorio, disparó el broadcast privado real.
5. El cliente lo recibió con la forma EXACTA que arma
   `private.emitir_cambio_nube_sitio()` en producción:
   `{"schema","table","operation","sitio_id","dispositivo_id","changed_at"}`.

Nota sin importancia para el resultado, pero digna de dejar anotada:
`dispositivo_id` llegó `null` en el payload -- porque el `INSERT` se hizo
por SQL directo (rol de servicio, sin JWT de dispositivo en el contexto de
la sesión), no vía PostgREST autenticado como el dispositivo. Un `INSERT`
real de la app (que sí pasa por PostgREST con el JWT del dispositivo en el
header) llevaría ese campo poblado -- `auth.jwt()->>'sub'` sólo resuelve
algo cuando la conexión que hace el `INSERT` está autenticada como tal.

Objetos descartables usados y ya borrados (sitio, dispositivo,
contratista de prueba) -- no quedó nada en staging. Para repetir esta
etapa hace falta recrearlos:

```sql
insert into public.sitios (nombre) values ('_lab_lattis_sitio') returning id;
insert into public.dispositivos (sitio_id, tipo, etiqueta, secret_hash)
  values ('<sitio_id de arriba>', 'pc', '_lab_lattis_dispositivo',
          '<sha256hex de tu secreto>')
  returning id;
```
```sh
curl -X POST "https://pmrytjktlyiuikxuuxpr.supabase.co/functions/v1/device-auth" \
  -H "Content-Type: application/json" -H "apikey: <anon key>" \
  -d '{"secret":"<tu secreto>"}'
```

### Etapa 3 (esta, parcial) -- reconexión con backoff ✅ implementado

`backoff.rs` -- la fórmula pura (`base * 2^intentos`, capada en `tope`),
igual criterio que `nubeRealtime.ts` (2s → 60s). Probada con valores
deterministas (sin async, sin tiempo real) para los casos borde: primer
intento, crecimiento exponencial, y que nunca desborde ni supere el tope
con una racha larguísima de fallas (`u32::MAX` intentos seguidos).

`supervisor.rs` -- el bucle conectar → latir → (si se cae) reintentar,
probado contra un servidor LOCAL que corta la primera conexión a
propósito después de un heartbeat, simulando una caída de red real:

1. Conecta, un heartbeat OK.
2. El servidor cierra el socket.
3. `supervisar_heartbeat` lo detecta (`Desconectado`), calcula el backoff
   (`Reintentando`, intento 0) y espera.
4. Reconecta solo, sin que nadie externo intervenga.
5. Sigue latiendo -- segundo heartbeat OK, en una conexión nueva.

Corrido 3 veces seguidas sin fallar (nada de sleeps largos ni timeouts
frágiles -- el test usa canales `mpsc` y espera eventos concretos, no
tiempo fijo). Un segundo test confirma que el contador de intentos
arranca en 0 en la primera reconexión, no arrastra estado de nada previo.

**Lo que esta etapa NO cubre todavía** (quedó fuera a propósito, para no
inflar el alcance):
- Renovación de JWT antes de que expire (el heartbeat no lleva JWT -- eso
  sólo aplica a un `phx_join` privado, que el supervisor de esta etapa no
  hace; integrarlo es la composición obvia de `unirse_privado` +
  `supervisar_heartbeat`, pendiente).
- Recuperar avisos perdidos mientras estuvo desconectado -- Phoenix no
  reenvía solo lo que te perdiste; eso es responsabilidad de la app (en
  `nubeRealtime.ts`, `programarSincronizacion()` al reconectar, que dispara
  un `sincronizarConNube()` completo). El equivalente acá sería, al
  recibir `EventoSupervisor::Conectado`, disparar la misma lógica de
  resync -- no es un problema del cliente WebSocket en sí.

### Etapa 3.5 (esta) -- la fila completa por el WebSocket, sin roundtrip REST ✅ implementado

Esta es la pregunta que originó todo el laboratorio (ver la conversación
que lo arrancó): ¿se puede mandar el DATO real por Realtime, en vez de un
aviso vacío que obliga a `sincronizarConNube()` completo después? Sí --
`realtime.broadcast_changes()` (función nativa de Supabase, no algo
inventado acá) arma el payload con la fila entera:

```sql
perform realtime.broadcast_changes(
  'sitio:' || new.sitio_id::text,  -- topic (mismo canal privado de la Etapa 2)
  'fila_completa',                  -- nombre del evento
  tg_op, tg_table_name, tg_table_schema,
  new, old
);
```

`bin/smoke_broadcast_changes.rs` probó esto contra una tabla e insert
reales (sitio y dispositivo descartables, ya borrados): el broadcast
recibido trajo la fila completa --
`{"id","record":{"cedula","creado_en","id","nombre","sitio_id"},"schema","table","operation","old_record"}`
-- y el binario la escribió en SQLite local (`nombre`, `cedula`, todo)
**sin hacer ningún `GET`/`SELECT` aparte**. Verificado releyendo la SQLite
resultante con `sqlite3`/`python3` directo, no sólo confiando en el log
del programa.

Comparado con el trigger real de producción
(`private.emitir_cambio_nube_sitio`, Etapa 2), que sólo manda metadata
(`schema`,`table`,`operation`,`sitio_id`,`dispositivo_id`,`changed_at`) y
obliga a un pull REST completo para saber qué cambió -- esto demuestra que
el patrón actual de la app (aviso vacío + resync completo) **no es una
limitación de Supabase Realtime**, es una elección de implementación. La
función que lo evitaría (`broadcast_changes`) ya está disponible en el
proyecto, sin ninguna migración nueva.

### Etapa 4 (parcial) -- renovación proactiva de JWT + heartbeat en canal privado ✅ implementado

Faltaba esto para que el canal privado (Etapa 2) aguantara una sesión
larga de verdad, no sólo el smoke test de un par de mensajes:

- **`ClienteRealtime::renovar_token`** -- push `access_token` in-band,
  investigado en `realtime-js`: refresca el JWT de un canal YA UNIDO sin
  `phx_join` de nuevo. Fire-and-forget (el protocolo no contesta en éxito;
  en falla, cierra el canal -- eso lo detecta el `esperar_evento` normal
  como una desconexión más).
- **Heartbeat propio en `supervisar_canal_privado`** -- bug real
  encontrado al implementar esto: el supervisor de canal privado sólo
  escuchaba pasivo, nunca mandaba nada. Phoenix cierra por defecto un
  socket que no manda NADA en ~60s (investigado, ver fuentes) -- un sitio
  silencioso habría desconectado al dispositivo sin que fuera un problema
  de red real. Ahora manda heartbeat cada `intervalo_heartbeat`.

**Probado contra `control-acceso-staging` REAL, no sólo mocks**
(`bin/smoke_supervisor_privado.rs`, dispositivo/sitio descartables, ya
borrados): 90 segundos corriendo, renovando el JWT cada 5s --

```
[  0.0s] Unido al canal privado real.
[  5.0s] Token renovado SIN reconectar (van 1).
...
[ 85.2s] Token renovado SIN reconectar (van 17).
OK -- 17 renovaciones de JWT sin reconectar contra el servidor REAL de Supabase.
```

**17 renovaciones reales, cero desconexiones.** Si el heartbeat propio no
funcionara, el socket habría muerto por inactividad mucho antes de la
renovación número 2 (a los ~60s sin heartbeat) -- llegar a la 17 en 90s es
la prueba de que ambos mecanismos (heartbeat + renovación) conviven bien
en la misma conexión.

Contra mocks locales (deterministas, sin red): un servidor de **una sola
conexión** captura el `access_token` renovado -- si el supervisor
reconectara para "renovar" (en vez de empujarlo in-band), el test se
colgaría esperando una segunda conexión que nunca llega. Otro mock,
completamente silencioso (nunca manda ningún broadcast), confirma que el
cliente manda heartbeat solo, sin que nadie se lo pida.

### Etapa 4 (parcial) -- Presence ✅ implementado

`nubeRealtime.ts` usa Presence para el panel de "quién está conectado" --
faltaba probar si eso también se podía hacer desde cero. Formato del
protocolo verificado contra la documentación oficial primero (`event:
"presence"`, con `type`/`event: "track"` anidados en el payload), pero
la documentación no alcanzó -- dos comportamientos reales que sólo
aparecieron corriendo esto contra `control-acceso-staging` de verdad
(`ClienteRealtime::diagnostico_mostrar_todo`, un modo de depuración que
imprime cada mensaje crudo sin filtrar, hizo falta para verlos):

1. **`presence_state`/`presence_diff` NO llegan automáticamente al hacer
   `phx_join`** -- recién aparecen después de que EL PROPIO cliente hace
   su primer `track()`. La documentación sugiere que `presence_state`
   "se entrega al unirse al canal"; contra el servidor real, unirse sin
   trackear nunca dispara nada.
2. **El `track` SÍ recibe un `phx_reply` normal** -- a diferencia de
   `access_token` (que la documentación confirma que no responde nada en
   éxito), el push de presence sí generó una respuesta `status: ok`.
3. (bug de orden, no de protocolo) El propio `track()` de un cliente
   genera DOS mensajes propios (`presence_state`, el snapshot completo, y
   `presence_diff`, tu propio join) -- si esperás sólo uno y asumís que el
   siguiente mensaje es de OTRO dispositivo, te comés tu propio eco.
   `bin/smoke_presence.rs` lo consume explícito antes de esperar el diff
   real de un segundo dispositivo.

**Probado con DOS dispositivos reales simultáneos** contra staging (sitio
y ambos dispositivos descartables, ya borrados):

```
[A] track()...
[A] presence_state (debería traer sólo a A): {"...":{"metas":[{"cedula":"A",...}]}}
[A] (descartado, es el propio) presence_diff de A: {"joins":{"...":{"cedula":"A"...}}}
[B] conectando, uniéndose y publicando su presencia...
[A] esperando el presence_diff con el join de B...
[A] presence_diff recibido: {"joins":{"...":{"cedula":"B",...}},"leaves":{}}
OK -- A vio en vivo, por Presence real, que B se conectó (cedula=B).
```

El dispositivo A vio en vivo, por Presence real (no por polling ni por un
mock), que el dispositivo B se conectó -- el mismo caso de uso exacto que
usa hoy el panel de presencia de `nubeRealtime.ts`.

### Etapa 4 (parcial) -- chequeo de lints reales ✅ implementado

Antes de tocar `desktop/src-tauri`/`mobile/rust-core` de verdad, un chequeo
más barato: ¿este código pasaría los lints del crate raíz
(`control_acceso`), que son bastante más estrictos que el default de
`cargo clippy` (`pedantic`+`nursery` en warn, y ~35 lints puntuales en
`deny`)? El `[lints.clippy]` de este `Cargo.toml` es una copia EXACTA del
de la raíz -- si se desincroniza del original, este chequeo deja de servir
de nada.

Encontró limpieza real que valía la pena hacer de todos modos: casts
`u128→u64`/`u64→f64` sin manejar el caso de desborde
(`cast_possible_truncation`/`cast_precision_loss`), dos `match`/`if-let`
reemplazables por `map_or_else` (`option_if_let_else`), una función que se
pasó de 100 líneas (`too_many_lines` -- se separó
`supervisar_canal_privado` en una función `mantener_canal_unido` aparte),
un `if let` sobre una variante de enum que clippy prefiere como
`matches!()` (`equatable_if_let`), y un lock de `tokio::sync::Mutex`
retenido más de lo necesario dentro de un `if let` (`significant_drop_in_scrutinee`,
puede llevar a deadlocks reales bajo carga). Todo corregido -- el crate
entero (librería, tests, los binarios `smoke_*`) pasa hoy los mismos lints
que exige `control_acceso` real, sin ningún `#[allow]` genérico, sólo los
puntuales y documentados donde la pérdida de precisión es aceptable a
propósito (el jitter del backoff).

Con esto resuelto, lo que queda antes de plantear un reemplazo real de
`nubeRealtime.ts`/`NubeRealtime.kt`:

- ~~**Meterlo de verdad en el árbol de dependencias real**~~ ✅ hecho para
  `desktop/src-tauri` (ver más abajo) -- `mobile/rust-core` sigue
  pendiente, no se tocó todavía.
- ~~**Puente Rust → frontend**~~ ✅ hecho -- evento `Tauri` real,
  verificado compilando de verdad contra Windows (ver más abajo). Falta
  sólo agregar el `listen(...)` del lado `App.tsx`, documentado pero no
  aplicado a propósito.
- **Shadow-run en producción real** -- correrlo en paralelo, sólo
  comparando/logueando (sin tocar el sync real), antes de considerar
  siquiera un corte real. Esta decisión NO está tomada -- este directorio
  es el experimento que la informa, no el resultado.

### Etapa 4 (parcial) -- integración real contra `desktop/src-tauri` ✅ implementado

Primer paso del punto anterior: ¿este crate encaja de verdad en el árbol
de dependencias de la app de escritorio, sin duplicar ni romper nada que
ya está fijado (`tokio = "=1.53.1"`, `url = "=2.5.8"`, `windows =
"=0.61.3"`)? Se agregó a `desktop/src-tauri/Cargo.toml` una feature
apagada por defecto y sin ningún llamador:

```toml
[features]
lattis-realtime-experimental = ["dep:lattis_realtime_spike"]

[dependencies]
lattis_realtime_spike = { path = "../../benchmarks/realtime-rust", optional = true }
```

Código completamente inerte -- no hay ningún `#[tauri::command]` ni
llamada que lo use todavía, sólo prueba que el grafo de dependencias
resuelve. `cargo metadata --features lattis-realtime-experimental`
(exit 0) confirma versiones unificadas en todo el árbol: `rustls
0.23.45`, `tokio-tungstenite 0.24.0`, `tokio 1.53.1`, `windows 0.61.3`,
`url 2.5.8`, `ring 0.17.14`, `rustls-native-certs 0.8.4` -- y, importante,
**ninguna copia de `aws-lc-rs`** (confirmando que no se coló un backend
que necesitaría un compilador de C para cross-compilar a Windows).

Actualización: se instaló el target `x86_64-pc-windows-gnu`
(`rustup target add`) y el cross-compilador `gcc-mingw-w64-x86-64`
(variante `posix`, la que necesita el `std` de Rust -- la variante `win32`
que trae el paquete por default NO sirve) en esta sandbox, así que el
chequeo barato de `cargo metadata` quedó reemplazado por uno real: ver
"Etapa 4 -- el puente Rust → frontend" más abajo, donde se confirma con
`cargo check`/`clippy --target x86_64-pc-windows-gnu` de verdad, no sólo
resolución de dependencias.

**Bug real encontrado al razonar sobre esta integración** (antes de
cualquier intento de compilar, sólo grepeando el árbol): ningún código de
producción llama `rustls::crypto::ring::default_provider().install_default()`
en ningún lado -- sólo lo hacían los binarios `smoke_*` de este
laboratorio. `desktop/src-tauri` ya trae `reqwest` con la feature
`rustls-tls` (vía `control_acceso/nube`, usado por los comandos que
hablan REST con Supabase), así que `reqwest`/`hyper-rustls` deben estar
instalando su propio `CryptoProvider` en silencio la primera vez que
arman un cliente HTTPS. El patrón que tenían los `smoke_*`
(`.install_default().expect(...)`) es correcto sólo porque cada binario
corre solo, nunca junto a `reqwest` -- en un proceso real donde ambos
coexisten (como sería `desktop/src-tauri` con la feature activada),
quien llegue primero instala el proveedor, y el segundo `.expect()`
entraría en pánico apenas arrancara la app.

Corregido agregando una función compartida en `src/lib.rs`,
`instalar_crypto_provider_tolerante()`, que traga el error de "ya hay uno
instalado" a propósito (`let _ = ...install_default();`) en vez de
`.expect()`-ear -- lo único que importa es que ambos usen el mismo
backend (`ring`, confirmado arriba que no hay `aws-lc-rs` en el árbol),
nunca cuál de los dos ganó la carrera de quién instala primero. Los 6
binarios `smoke_*` de este laboratorio se migraron a esta función
tolerante (siguen pudiendo correr solos igual que antes -- el cambio es
sólo más seguro, no les quita nada). Verificado después del cambio:
`cargo build --bins`, `cargo clippy --all-targets --tests` (limpio, cero
warnings) y `cargo test` (35 tests, todos verdes) en este crate, y
`cargo metadata --features lattis-realtime-experimental` de nuevo en
`desktop/src-tauri` (exit 0, mismas versiones unificadas).

### Etapa 4 (parcial) -- el puente Rust → frontend ✅ implementado

Con la integración de Cargo.toml resuelta, el siguiente pendiente de la
lista de arriba: un evento `Tauri` real, emitido desde código que usa
`lattis_realtime_spike`, que el frontend pueda escuchar -- sin tocar
`comandos::nube`/`AppCore`, sin reemplazar `nubeRealtime.ts`, sin correr
en ninguna máquina real todavía.

Se agregó `desktop/src-tauri/src/lattis_experimental.rs`, un módulo
completo detrás de `#[cfg(feature = "lattis-realtime-experimental")]`
(ni se compila en un build normal) que además exige la variable de
entorno `LATTIS_EXPERIMENTAL_WS_URL` para hacer algo -- la variable, no
la feature, es la puerta real, así que llamarlo siempre desde
`configurar_arranque` (detrás de la misma feature) es seguro incluso en
una máquina con la feature activada pero sin esa variable configurada.
Cuando está configurada, usa `supervisar_heartbeat` del laboratorio
contra la URL que le den (pensada para `control-acceso-staging`, jamás
producción sin que esa decisión esté tomada -- ver "Shadow-run en
producción real" abajo) y por cada `EventoSupervisor` emite
`app.emit("lattis://experimental", …)` con un DTO aplanado y
serializable -- mismo patrón exacto que ya usa
`iniciar_sincronizacion_automatica` con `"nube://sincronizado"` en
`lib.rs`. El frontend lo escucharía igual que ya escucha ese evento
(`desktop/src/App.tsx`, `listen<T>("nube://sincronizado", cb)`) -- no se
agregó ese listener todavía (sería tocar `App.tsx`, código real de UI,
sin que haga falta para probar el mecanismo), queda documentado acá como
el próximo paso obvio si se decide seguir por este camino:

```ts
import { listen } from "@tauri-apps/api/event";

listen<{ tipo: string; detalle: string }>("lattis://experimental", ({ payload }) => {
  console.debug("[lattis experimental]", payload.tipo, payload.detalle);
});
```

**Validación real, no sólo `cargo metadata`**: esta sandbox no tenía el
target `x86_64-pc-windows-gnu` ni un cross-compilador MinGW -- se
instalaron ambos (`rustup target add x86_64-pc-windows-gnu`,
`apt install gcc-mingw-w64-x86-64`, forzando la variante `posix` con
`update-alternatives` -- la `win32` que trae el paquete por default no
sirve para el `std` de Rust) para poder compilar de verdad contra
Windows, el target real de `desktop/src-tauri`. Con eso:

- `cargo check --target x86_64-pc-windows-gnu` (sin la feature): limpio,
  confirma que la base no se rompió.
- `cargo check --target x86_64-pc-windows-gnu --features
  lattis-realtime-experimental`: limpio -- `lattis_experimental.rs`
  compila de verdad contra el árbol real (`tauri 2.11.5`, `tokio 1.53.1`,
  `tokio-tungstenite 0.24.0`, etc.), no sólo resuelve en `cargo metadata`.
- `cargo clippy --target x86_64-pc-windows-gnu --features
  lattis-realtime-experimental -- -D warnings`: exit 0, cero warnings --
  pasa el mismo `[lints.clippy]` estricto que el resto de
  `desktop/src-tauri`.

De paso se confirmó algo que no era obvio antes: en Linux, sin la
feature ni con ella, `cargo check` normal (target del host) siempre
falla en este crate -- `zeroize`/`intentar_abrir_nucleo` sólo existen
bajo `[target.'cfg(windows)'.dependencies]`/`#[cfg(windows)]` (DPAPI,
MessageBox nativo). No es un bug de esta integración: `desktop/src-tauri`
nunca estuvo pensado para compilar en Linux, sólo para cross-compilar a
Windows -- de ahí que el chequeo real tenga que ser siempre con
`--target x86_64-pc-windows-gnu`.

Con esto, de los tres pendientes que quedaban, sólo falta uno real:
**shadow-run en producción real** (decisión no tomada -- el mecanismo de
arriba ya sirve para eso, apuntándolo a la URL que sea vía variable de
entorno, pero nunca se apuntó a `control-acceso-nube`, sólo se probó el
compile).

## Cobertura de pruebas

35 tests en total, en cuatro capas distintas, cada una probando algo que
las otras no cubren. `cargo test --manifest-path benchmarks/realtime-rust/Cargo.toml`
corre las cuatro. Ninguna toca red externa ni credenciales reales --
eso queda en los binarios `smoke_*` de `src/bin/` (manuales, contra
`control-acceso-staging`, documentados arriba).

### 1. Unitarias (`src/*.rs`, `mod tests`) -- 21 tests

Los detalles internos: framing del protocolo, el cliente WebSocket, el
supervisor de reconexión. Incluye 6 pruebas de CAOS basadas en fallas
reales documentadas (no imaginadas -- ver fuentes al final del README):

- **Basura no-JSON del servidor** → error tipado, nunca pánico.
- **Corte en seco sin `close` frame** (`drop(socket)`, no `.close()`) →
  error de conexión, no un cuelgue eterno -- reproduce el caso real de un
  load balancer/proxy que mata la conexión sin avisar.
- **Servidor mudo** (acepta, no responde nunca) → `Timeout` a los 5s, no
  espera para siempre.
- **Canal privado en silencio total** → el cliente manda heartbeat solo,
  sin que nadie se lo pida (Etapa 4).
- **Renovación de JWT capturada en una única conexión** → si el supervisor
  reconectara para "renovar" en vez de empujar el token in-band, el test
  se colgaría esperando una segunda conexión que nunca llega (Etapa 4).
- **Token viejo reenviado al reconectar** (`nunca_reenvia_un_token_viejo_al_reconectar`)
  → reproduce el bug real de `supabase-py`/`supabase-js` donde el cliente
  cachea el JWT en el payload de join y lo reenvía tal cual tras
  reconectar, incluso renovado. Acá es estructuralmente imposible: no
  existe ningún campo donde un token viejo pueda sobrevivir entre
  conexiones, `obtener_token_fresco` se llama de nuevo en cada intento.

### 2. Basadas en propiedades (`proptest`, dentro de `src/*.rs`) -- 7 tests

En vez de elegir a mano cada string rara, se le piden a `proptest` cientos
de entradas al azar por corrida (unicode, comillas, backslashes, strings
vacíos o kilométricos) y se verifica que la propiedad se sostiene siempre;
si falla, `proptest` reduce el caso al mínimo que lo rompe:

- Cualquier topic/referencia/access_token sobrevive intacto un viaje de
  ida y vuelta por JSON.
- `es_reply_ok_de` nunca entra en pánico sin importar la forma del
  `payload` (ni siquiera si no es un objeto).
- El backoff (con o sin jitter) nunca supera el tope, para cualquier
  combinación de intentos/base/tope/semilla.

### 3. De integración (`tests/integracion_*.rs`) -- 5 tests

Ejercitan el crate por su API PÚBLICA únicamente (`lattis_realtime_spike::...`,
sin acceso a nada `pub(crate)`) -- si el contrato público se rompe, esto lo
detecta antes que cualquier consumidor real:

- `integracion_heartbeat.rs`: `ClienteRealtime`/`supervisar_heartbeat`
  funcionan desde afuera del crate.
- `integracion_autorizacion_canal_privado.rs`: codifica de forma
  permanente el contrato de autorización validado a mano contra staging en
  la Etapa 2 -- token correcto se acepta, token incorrecto o vacío se
  rechaza, sin depender de que staging esté arriba.

### 4. De punta a punta (`tests/e2e_*.rs`) -- 2 tests

Automatizados (a diferencia de los `smoke_*` manuales) -- corren en cada
`cargo test`, contra un mock local que representa un servidor Phoenix
completo. Son la comparación directa que responde "¿es más rápido?" --
ver la sección siguiente.

## ¿Es más rápido?

Depende de qué pregunta sea esa. Hay dos preguntas distintas y sólo una
tiene una respuesta rigurosa hoy:

**"¿El patrón `broadcast_changes` es más rápido que el aviso-vacío-actual?"
-- SÍ, y es medible sin ambigüedad**, con los dos tests de punta a punta:

| Test | Round-trips cliente→servidor tras el join |
|---|---|
| `e2e_aviso_vacio_y_resync.rs` (patrón actual de producción) | 1 (`resync_fetch`) |
| `e2e_fila_completa.rs` (`broadcast_changes`) | 0 |

No es un cronómetro sobre `localhost` (eso sería ruido, no evidencia --
loopback no tiene latencia real que medir). Es un CONTEO de mensajes,
determinístico y reproducible: el patrón actual estructuralmente necesita
un mensaje más para tener el dato. En una red real (no loopback), ese
mensaje de más cuesta como mínimo un RTT completo cliente↔Supabase --
típicamente decenas de milisegundos, más si el dispositivo está en una
conexión mala. Eliminar ese round-trip es una ganancia real y
cuantificable, independiente de qué lenguaje lo implemente.

**"¿Es más rápido que `nubeRealtime.ts`/`NubeRealtime.kt` porque está en
Rust?" -- Eso NO está probado, y hay que decirlo con la misma claridad.**
Comparar la velocidad de un cliente WebSocket en Rust contra uno en
JS/Kotlin de forma justa requiere un benchmark controlado con ambos
corriendo bajo las mismas condiciones de red -- algo que este crate,
por sí solo, no puede hacer (no tiene un cliente JS al lado para
comparar). Lo que SÍ se puede afirmar con evidencia real de esta sesión:
la conexión y el parseo de mensajes en Rust no tienen el overhead de un
motor JS/runtime de WebView -- pero eso es una expectativa razonable, no
un número medido. Si en algún momento se quiere esa comparación real, hace
falta un harness aparte que levante ambos clientes contra el mismo
servidor y mida percentiles de latencia -- no está hecho todavía.

## Qué NO hace todavía (a propósito)

- El supervisor de reconexión con canal privado (`supervisar_canal_privado`)
  siempre pide un token fresco al reconectar, pero no renueva el JWT de
  forma PROACTIVA mientras la conexión sigue viva y a punto de expirar --
  sólo reacciona cuando de todos modos tiene que reconectar.
- No recupera avisos perdidos al reconectar (ver Etapa 3, es responsabilidad
  de la app, no del cliente).
- No implementa Presence.
- No está integrado a `sincronizarConNube()` ni a ningún flujo real de la
  app -- es un binario y una librería sueltos, corridos a mano.

## Fuentes de la investigación de fallas reales

- [Access token not refreshed for realtime channels after being offline or in standby · supabase/realtime-js#274](https://github.com/supabase/realtime-js/issues/274)
- [Realtime set_auth doesn't update the join payload, so rejoins send the old token · supabase/supabase-py#1655](https://github.com/supabase/supabase-py/issues/1655)
- [Realtime connection unable to reconnect after TIMED_OUT · supabase/realtime#1088](https://github.com/supabase/realtime/issues/1088)
- [Debugging WebSocket Real-Time Features: Packet Loss & Reconnect Storms](https://buglyst.com/blog/debugging-real-time-features)
- [Writing a Channels Client — Phoenix docs (heartbeat/timeout, entrega at-most-once)](https://phoenix.hexdocs.pm/writing_a_channels_client.html)
- [PR #117 realtime-js "push access token only to joined channels" (el mecanismo `access_token` in-band que usa `ClienteRealtime::renovar_token`)](https://github.com/supabase/realtime-js/pull/117)
- [JavaScript: Update the access token — Supabase docs](https://supabase.com/docs/reference/javascript/auth-setauth)
- [Realtime Protocol — Supabase docs (formato exacto de `presence_state`/`presence_diff`/`track`; el comportamiento real de CUÁNDO se disparan no coincidió del todo con lo que sugiere el texto, ver Etapa 4)](https://supabase.com/docs/guides/realtime/protocol)
- [Presence — Supabase docs](https://supabase.com/docs/guides/realtime/presence)
