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

### Etapa 4 (pendiente) -- decisión de integración

Recién acá se evalúa si esto reemplaza de verdad a `nubeRealtime.ts` y
`NubeRealtime.kt`, o si el costo de mantenimiento de un cliente Phoenix
Channels a mano no se justifica frente a mantener las dos implementaciones
actuales. Esta decisión NO está tomada -- este directorio es el
experimento que la informa, no el resultado.

## Cobertura de pruebas

32 tests en total, en cuatro capas distintas, cada una probando algo que
las otras no cubren. `cargo test --manifest-path benchmarks/realtime-rust/Cargo.toml`
corre las cuatro. Ninguna toca red externa ni credenciales reales --
eso queda en los binarios `smoke_*` de `src/bin/` (manuales, contra
`control-acceso-staging`, documentados arriba).

### 1. Unitarias (`src/*.rs`, `mod tests`) -- 25 tests

Los detalles internos: framing del protocolo, el cliente WebSocket, el
supervisor de reconexión. Incluye 4 pruebas de CAOS basadas en fallas
reales documentadas (no imaginadas -- ver fuentes al final del README):

- **Basura no-JSON del servidor** → error tipado, nunca pánico.
- **Corte en seco sin `close` frame** (`drop(socket)`, no `.close()`) →
  error de conexión, no un cuelgue eterno -- reproduce el caso real de un
  load balancer/proxy que mata la conexión sin avisar.
- **Servidor mudo** (acepta, no responde nunca) → `Timeout` a los 5s, no
  espera para siempre.
- **Token viejo reenviado al reconectar** (`nunca_reenvia_un_token_viejo_al_reconectar`)
  → reproduce el bug real de `supabase-py`/`supabase-js` donde el cliente
  cachea el JWT en el payload de join y lo reenvía tal cual tras
  reconectar, incluso renovado. Acá es estructuralmente imposible: no
  existe ningún campo donde un token viejo pueda sobrevivir entre
  conexiones, `obtener_token_fresco` se llama de nuevo en cada intento.

### 2. Basadas en propiedades (`proptest`, dentro de `src/*.rs`) -- 6 tests

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
