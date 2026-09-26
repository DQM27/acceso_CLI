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

### Etapa 3 (pendiente) -- reconexión y backoff

Igual criterio que ya implementa `nubeRealtime.ts` hoy: backoff exponencial
(2s → 60s tope), renovación de JWT antes de que expire, recuperar avisos
perdidos mientras estuvo desconectado.

### Etapa 4 (pendiente) -- decisión de integración

Recién acá se evalúa si esto reemplaza de verdad a `nubeRealtime.ts` y
`NubeRealtime.kt`, o si el costo de mantenimiento de un cliente Phoenix
Channels a mano no se justifica frente a mantener las dos implementaciones
actuales. Esta decisión NO está tomada -- este directorio es el
experimento que la informa, no el resultado.

## Qué NO hace todavía (a propósito)

- No maneja reconexión ni backoff.
- No renueva JWT.
- No implementa Presence.
- No está integrado a `sincronizarConNube()` ni a ningún flujo real de la
  app -- es un binario y una librería sueltos, corridos a mano.
