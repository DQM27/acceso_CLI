# Plan: telemetría y errores en producción

Estado: **propuesta** (2026-10-31). Reemplaza el destino de staging de
`plan-telemetria-y-registro-de-errores.md`; los estudios por equipo, el catálogo
de eventos y la captura propia de errores no cambian.

## Por qué

Hoy la telemetría (`telemetria_diagnostico`) y los errores (`errores_app`)
viven en el proyecto de **staging**, separados de producción:

- **No se pueden cruzar con los equipos.** Cada fila trae un UUID al azar por
  instalación, no `dispositivos.id`: no hay forma segura de saber qué equipo,
  de qué unidad, con qué nombre. Reinstalar la app crea "otro equipo".
- **Entran sin sesión.** La telemetría se escribe con la llave publicable
  (rol `anon`), la única excepción a la regla "nunca a `anon`"; los errores,
  con una clave compilada en las apps. Cualquiera que saque la llave del APK
  puede escribir filas falsas.
- **Columnas imprecisas.** Todo va en `datos` (jsonb), y la versión es un
  texto con formatos distintos por plataforma (`1.8.13 (58)`,
  `desktop-1.10.7+889cc4f`).

## Decisiones (tomadas con el dueño, no reabrir)

- La telemetría y los errores pasan a **producción**.
- **Sin diagnóstico remoto ni modo mantenimiento** por ahora: cada canal
  hacia el equipo es una puerta más que vigilar, y el equipo de operación es
  una persona. Con telemetría y errores completos alcanza para diagnosticar.
- Grafana se retira **después** de que las alertas pasen a correo (Resend).

## Carga sobre la base principal

Medido en staging (2026-10-31, dos equipos, con `fluidez` y `sincronizacion`
encendidos): **~6.600 filas por día, ~415 bytes por fila, ~2,7 MB por día**.
Las apps mandan en **lotes cada 60 s** (hasta 200 filas): una petición por
minuto por equipo. Con 30 días de retención son ~80 MB en el peor caso, y
mucho menos con sólo `base` + `rendimiento`. Es poco para Postgres, y para que
no compita con la operación:

- Esquema propio **`observabilidad`**, **no expuesto** por la API: nadie lee
  ni escribe las tablas directo. Se entra sólo por dos funciones.
- Escritura por **lote en una sola llamada** (`insert ... select from
  jsonb_to_recordset`), sin triggers ni Realtime en estas tablas; el lote
  viaja comprimido con brotli (ver "Transporte").
- Índices mínimos (equipo + hora); sin claves foráneas hacia tablas
  operativas en la ruta caliente salvo `dispositivos`.
- **Retención** automática con `pg_cron` (una vez por día, de madrugada):
  borrar por lotes lo vencido.

## Transporte: HTTP/2 + brotli, por un receptor propio

Probado contra staging el 2026-10-31:

- **HTTP/2:** Supabase lo negocia (`HTTP 2` en API y Edge Functions). El
  núcleo ya lo usa: comprobado con su cliente, `HTTP/2.0` contra producción;
  por ahí van todas las operaciones de las dos apps (ingresos, verificaciones,
  sincronización). Lo que sale de Kotlin por `HttpURLConnection` (HTTP/1.1,
  una conexión nueva cada vez) es la telemetría, el registro de avisos push
  y la búsqueda de actualizaciones.
- **Cuerpo comprimido directo a la RPC: no funciona.** La puerta de entrada de
  Supabase no descomprime el cuerpo de la petición: con `Content-Encoding:
  gzip` o `br`, PostgREST responde `PGRST102 Empty or invalid json`. Sólo
  comprime las *respuestas*.
- **Una Edge Function sí recibe el cuerpo comprimido intacto** (probado con
  `Content-Encoding: br` hacia `receptor-errores`).
- **Ahorro real:** ver la tabla de formatos más abajo.

### Formato del lote: columnar por tipo + brotli 11

Medido sobre el mismo lote real (200 filas del móvil), en bytes:

| Formato | Crudo | gzip 6 | brotli 5 | brotli 11 |
|---|---:|---:|---:|---:|
| JSON por fila (hoy) | 75.463 | 5.532 | 4.846 | 4.054 |
| JSON con cabecera de sesión y tiempo en diferencias | 34.515 | 4.457 | 4.140 | 3.424 |
| **JSON columnar por tipo** | 11.479 | 3.373 | 3.303 | **2.932** |
| El mismo columnar en MessagePack | 11.870 | 3.818 | 3.702 | 3.489 |
| El mismo columnar en CBOR | 11.906 | 3.803 | 3.670 | 3.465 |

- **Columnar:** lo común del lote va una vez en la cabecera (sesión, versión,
  `esquema`, `plataforma`); cada tipo de evento lleva sus nombres de columna
  una vez y después sólo filas de valores; la hora va como diferencia en ms
  con la fila anterior, y una lista de tipos conserva el orden original.
- **Los binarios no ganan:** después de comprimir, MessagePack y CBOR quedan
  más grandes que el JSON columnar (brotli aprovecha mejor el texto
  repetitivo), y JSON se depura y se valida en la base sin decodificador
  extra. Protobuf/gRPC tampoco aplica: Supabase no expone gRPC, y lo que
  gRPC aporta (una conexión HTTP/2 reutilizada, binario compacto) se cubre con
  HTTP/2 + este formato.
- **Brotli 11:** 22 ms para comprimir el lote en una PC (en el teléfono unas
  pocas decenas de ms más), una vez por minuto y en un hilo de fondo;
  descomprimir, 0,08 ms. Total: de 75 KB a **~3 KB por lote (25 veces
  menos)**, ~1,3 MB → ~50 KB de datos por día por equipo.
- **Sin pérdida, comprobado:** el lote reconstruido desde el columnar es
  idéntico al original fila por fila (hora al milisegundo, tipo y cada campo
  de `datos`). Un campo ausente se distingue de uno `null`.

### Que no se corrompa ni se duplique

- **Integridad en el viaje:** TLS ya autentica cada byte; un cuerpo alterado
  no descomprime o no valida y se rechaza entero (400), nunca a medias.
- **Todo o nada:** el lote entra en una sola transacción; si una fila no pasa
  la validación (tipo fuera del catálogo, campo de tipo incorrecto), no entra
  ninguna y el equipo lo sabe por la respuesta.
- **Sin duplicados:** cada lote lleva un `lote_id` (UUID) y la base lo guarda
  con restricción única. Si la respuesta se pierde y el equipo reintenta, el
  segundo envío responde "ya recibido" sin insertar otra vez. El equipo borra
  el lote de su cola sólo con esa confirmación.
- **Pruebas de ida y vuelta** en Rust (columnar → JSON → brotli y vuelta) y
  en la función receptora, con lotes reales como casos.

Por eso:

- Un receptor **`observabilidad`** (Edge Function, `verify_jwt = true`) con dos
  rutas, `/telemetria` y `/errores`. Acepta `Content-Encoding: br` (y `gzip`
  como respaldo), descomprime **con tope** (1 MB descomprimido; un cuerpo que
  lo pase se corta y responde 413: nada de bombas de compresión) y llama a la
  RPC **reenviando el JWT del equipo**, no con `service_role`: la identidad y
  los límites siguen en la base.
- El envío vive **en el núcleo** (Rust, `reqwest` con HTTP/2 y el crate
  `brotli` para comprimir), y lo usan las dos plataformas: el móvil le pasa
  sus lotes por UniFFI en vez de mandarlos por su cuenta. Una sola
  implementación de cola, token, compresión y reintentos.
- Costo: una invocación por minuto por equipo (~43.000 al mes por equipo),
  dentro del cupo de Edge Functions.

## Modelo

```
observabilidad.sesiones      una fila por arranque de la app
observabilidad.eventos       todo evento del catálogo, salvo llamada_nucleo
observabilidad.llamadas      llamada_nucleo (65 % de las filas), con columnas
observabilidad.errores       cada ocurrencia de un error
observabilidad.errores_grupo un error agrupado por huella, con su estado
```

### `sesiones`

| Columna | Tipo | Nota |
|---|---|---|
| `id` | uuid PK | lo genera la app al arrancar |
| `dispositivo_id` | uuid FK `dispositivos` | **lo pone el servidor** desde el JWT |
| `sitio_id` | uuid | ídem |
| `plataforma` | text | `movil` / `escritorio` |
| `version_app` | text | `1.8.13`, `1.10.7`: sólo semver |
| `version_codigo` | int | `versionCode` del móvil; nulo en escritorio |
| `commit` | text | hash corto del build |
| `ambiente` | text | `produccion` / `staging` |
| `iniciada_en` | timestamptz | |
| `sistema` | text | `Android 14`, `Windows 11` |
| `modelo` | text | `Build.MODEL` / nombre del equipo |
| `ram_total_mb` | int | |
| `detalle` | jsonb | lo demás de `sesion_inicio` (ABI, SDK, núcleos) |

Las columnas comunes dejan de repetirse en cada evento: un evento apunta a su
sesión.

### `eventos`

| Columna | Tipo | Nota |
|---|---|---|
| `id` | bigint identity PK | |
| `sesion_id` | uuid | |
| `dispositivo_id` | uuid | del JWT; desnormalizado para filtrar rápido |
| `ocurrido_en` | timestamptz | hora confiable del núcleo |
| `recibido_en` | timestamptz | `now()` del servidor |
| `tipo` | text | validado contra el catálogo |
| `grupo` | text | `base`, `rendimiento`, …: lo deriva el servidor del catálogo |
| `pantalla` | text | si el evento la trae |
| `datos` | jsonb | el detalle propio de cada tipo (contrato, esquema 3) |

### `llamadas`

`id`, `sesion_id`, `dispositivo_id`, `ocurrido_en`, `recibido_en`,
`operacion` (nombre canónico), `llamadas` int, `errores` int, `p50_ms`,
`p90_ms`, `max_ms`, `total_ms` (int), `red_tipo` (lista fija),
`errores_por_tipo` jsonb.

### `errores` y `errores_grupo`

`errores_grupo`: `huella` PK, `plataforma`, `titulo`, `origen`,
`primera_vez`, `ultima_vez`, `ocurrencias`, `versiones` text[], `estado`
(`nuevo`/`visto`/`resuelto`/`ignorado`), `estado_cambiado_en`.

`errores`: `id`, `huella` FK, `event_id` unique (reintentos), `sesion_id`,
`dispositivo_id`, `ocurrido_en`, `recibido_en`, `nivel`, `tipo`, `titulo`,
`traza` jsonb, `pantalla`, `operaciones` jsonb, `contexto` jsonb.

Si un error `resuelto` vuelve a aparecer en una versión nueva, el grupo pasa a
`nuevo` (regresión) y dispara la alerta.

## Seguridad

- `public.registrar_telemetria(sesion jsonb, eventos jsonb)` y
  `public.registrar_errores(reportes jsonb)`: `security definer`, `grant
  execute to authenticated` (los equipos usan su JWT), **nunca `anon`**.
  Exigen `private.dispositivo_vigente()`; equipo y unidad salen de
  `private.dispositivo_que_llama()`, nunca del cuerpo.
- Límites dentro de la función: filas por lote (500), bytes por evento
  (16 KB), lotes por minuto por equipo.
- Los errores se limpian otra vez en el servidor (la misma `limpiar_texto` del
  receptor actual, portada a SQL o hecha en una Edge Function con
  `verify_jwt = true`).
- Lectura para el panel: funciones `security definer` con
  `private.es_admin_global()`, `grant to authenticated`.
- Batería en `supabase/tests/` (probada en staging con `rollback`):
  sin sesión no entra; un equipo retirado no entra; un equipo no puede
  escribir a nombre de otro; tipo fuera del catálogo se rechaza; límites.

## En las apps

- La cola en disco de telemetría y la de errores ya existen; cambia el
  destino: la RPC de producción con el **token del equipo**
  (`CacheTokenDispositivo`), no la llave de staging.
- Sin token (equipo sin vincular o sin red) los eventos esperan en disco,
  con el tope que ya tienen.
- Un pánico se escribe a disco en el momento (ya es así) y se manda al
  arrancar de nuevo, con token.
- Esquema del contrato **3**: versión semver + `version_codigo` + `commit`
  separados; `sesion_inicio` pasa a ser la fila de `sesiones`.

## Fases

| # | Fase | Qué |
|---|---|---|
| 1 | Nube | Migración del esquema `observabilidad`, las RPC, retención con `pg_cron` y batería; Edge Function receptora (brotli, JWT). Staging primero. |
| 2 | Núcleo | Envío desde el núcleo: HTTP/2, brotli y token del equipo, para telemetría y errores de las dos plataformas. |
| 3 | Plataformas | Kotlin entrega sus lotes al núcleo por UniFFI y el escritorio usa el mismo envío; contrato esquema 3. Durante la transición las versiones viejas siguen mandando a staging. |
| 4 | Panel | Telemetría y Errores leen de producción (filtro por equipo, versión, rango en hora de Costa Rica). |
| 5 | Alertas | Edge Function + Resend: errores nuevos o regresiones, equipo sin conectarse, equipo nuevo en vinculación, errores reales de la cola. Los eventos se definen con el dueño. |
| 6 | Retiro | Apagar Grafana, el receptor de staging y las tablas viejas cuando ninguna versión en uso mande allá. |

## Pendiente de decidir

- Días de retención: propuesta **30** para eventos y llamadas, **180** para
  errores (pocos y valiosos para ver regresiones).
- `pg_cron`: disponible en producción (1.6.4) pero **no instalado**; la fase 1
  lo habilita en su migración. `pg_net` ya está instalado (0.20.4) y sirve
  para que las alertas de la fase 5 llamen a la Edge Function de correo.
