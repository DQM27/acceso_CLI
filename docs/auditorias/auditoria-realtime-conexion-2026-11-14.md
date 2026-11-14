# Auditoría del canal en vivo (Realtime): escritorio y móvil — 2026-11-14

Diagnóstico de cómo se conectan la app de escritorio y la app móvil con la
nube por el canal en vivo, con foco en **robustez** y **telemetría** (la
velocidad está bien). No se cambió código: este documento es para que otra
sesión retome el trabajo.

Rama analizada: `mejoras/conexion-y-detalles` (commit `3b38474`).

## Estado al cerrar esta sesión

- El 2026-11-14 se activó la telemetría (grupo `sincronizacion`) en **todos**
  los dispositivos, por **7 días** (vence alrededor del 2026-11-21). El
  domingo 15 es un día sin operación; los datos útiles empiezan el lunes 16.
- Pendiente: revisar los datos nuevos (consultas al final) y decidir qué
  implementar. Nada de lo propuesto abajo está hecho.
- Límite de lo que la semana de datos puede decir: las versiones instaladas
  tienen los huecos de telemetría T2, T3 y T4. La semana va a confirmar
  **cuánto** se cae el canal, pero no **por qué**. Los hallazgos R1 y R2 ya
  están demostrados con los datos viejos y no dependen de esperar.

## De dónde salen los datos

- Tabla `public.telemetria_diagnostico` del proyecto Supabase **staging**
  (`pmrytjktlyiuikxuuxpr`): los equipos de producción mandan ahí su
  telemetría. Evento `realtime`, del 2026-10-15 al 2026-11-01: 2 PC (1 114
  filas), 7 teléfonos (2 161 filas). También `llamada_nucleo` y
  `sincronizacion_auto`.
- Registros de Realtime del proyecto de **producción**
  (`xidaepyaljzkpbsxrqsm`), últimas 24 h al 2026-11-14.
- Las versiones medidas son anteriores al arreglo de "sin red" del
  2026-11-14 (`nubeRealtime.ts`, `esperandoRed`).

## Cómo funciona hoy (resumen)

- Servidor: el trigger `private.emitir_cambio_nube_sitio()` llama a
  `realtime.send(...)` al canal privado `sitio:<id>` con el evento
  `cambio_nube`; para `ingresos`, `ingresos_proveedor`, `ingresos_correo` y
  `prestamos_gafete_provisional` el aviso trae la fila (`registro`). La RLS de
  `realtime.messages` exige dispositivo vigente. El token de dispositivo lo
  emite `device-auth` y dura **1 h** (`TTL_TOKEN_SEGUNDOS`).
- Escritorio: `desktop/src/nubeRealtime.ts` (supabase-js / realtime-js
  2.118). Pide la sesión al núcleo (`sesion_realtime_nube`), se suscribe,
  aplica la fila con `aplicar_cambio_nube` (`src/nube/en_vivo.rs`) y, al
  suscribirse, corre una sincronización completa. Pulso automático cada 5 min
  (`desktop/src-tauri/src/lib.rs`).
- Móvil: `NubeRealtime.kt` (realtime-kt 3.2.2) + `SincronizacionPeriodica.kt`
  + `CambiosNube.kt`. El canal sólo vive con la app en primer plano
  (`PantallaPrincipal.kt`, ON_START/ON_STOP); pulso completo cada 5 min.
- Caché del token: `src/nube/cache_token.rs`, entrega el token hasta 15 min
  antes de vencer (`MARGEN_EXPIRACION`).

## Lo que funciona bien

| | Escritorio | Móvil |
|---|---|---|
| Latencia del aviso (p50 / p90) | 114 / 248 ms | 112 / 392 ms |
| Tiempo hasta suscribir (p50) | 0,92 s | 0,93 s |
| Guardar la fila del aviso (p50) | 41 ms | 23 ms |

El diseño de fondo es sólido: canal privado por sitio, aviso con la fila,
sincronización completa al suscribirse y pulso como red de seguridad.

## Hallazgos de robustez

### R1. El escritorio se reconecta cada 45 min por diseño (confirmado)

El token dura 1 h y la caché lo entrega hasta 15 min antes de vencer; el canal
programa su renovación con `expires_in - 60 s` y, al renovar, desmonta el
socket entero (`limpiarCanal`) y vuelve a conectar, con una sincronización
completa (~13 consultas, ~1,2 s) cada vez.

- Datos: 83 renovaciones; 51 con 44–46,5 min de conexión (45,2 min
  repetido); el resto hasta 59 min.
- Propuesta: que el callback `accessToken` del cliente pida el token vigente
  al núcleo (`sesionRealtimeNube()`, que ya usa la caché) en vez de devolver
  uno fijo, y renovar con `realtime.setAuth()` sobre el mismo socket. Sin
  corte ni sincronización completa por renovación.
- De paso: varios comentarios hablan de un token de "12 h"
  (`cache_token.rs`, `NubeRealtime.kt`); hoy es 1 h.

### R2. Dos mecanismos de reconexión compiten en el escritorio (confirmado)

Cuando el canal cae, realtime-js se vuelve a unir solo y el callback de
`subscribe` recibe `SUBSCRIBED` otra vez (sincronización completa + `track`).
Pero el `CHANNEL_ERROR` anterior ya programó `reconectar()`, que ~2 s después
destruye esa conexión sana y abre otra (segunda sincronización completa).

- Datos: 8 filas con `ms_hasta_suscribir_max` > 60 s (hasta 2 090 362 ms).
  El 2026-10-20 08:01 una fila da 697 165 ms = 11,6 min, exactamente los
  `minutos_conectado_max` de la conexión que acababa de caer: el "tiempo
  hasta suscribir" se midió desde el intento original.
- Propuesta: un solo dueño de la reconexión. Mínimo: al recibir `SUBSCRIBED`
  cancelar `temporizadorReconectar`. Mejor: dejar a la librería las caídas
  transitorias y reconstruir el cliente sólo ante fallo de autenticación o
  falla prolongada.

### R3. Un rechazo del canal no invalida el token en caché

En el corte del 2026-10-20 (11:46–12:06 UTC) `sesion_realtime_nube` respondió
en ~2 ms (caché) cada minuto mientras el canal daba `CHANNEL_ERROR`. Esa vez
era falta de red (`sincronizacion_auto` también falló, en 47 ms), pero si la
causa fuera un token rechazado el canal quedaría en bucle con el mismo token
hasta que la caché lo suelte (hasta 45 min). La sincronización sí invalida un
token vencido (`FalloSincronizacion::TokenVencido`); el canal no.

- Propuesta: ante un error de unión de tipo autenticación, invalidar la caché
  (`invalidar_token_cacheado`) antes del reintento.

### R4. Recuperación lenta con red pero sin internet (escritorio)

El arreglo del 2026-11-14 cubre `navigator.onLine === false`. Con un teléfono
compartiendo sin datos, `onLine` es `true`: sigue reintentando con tope de
60 s y, cuando vuelve internet, espera lo que quede de ese minuto.

- Propuesta: usar el éxito del pulso o de cualquier sincronización como señal
  de "volvió la red" para reconectar enseguida y poner el contador en cero.

### R5. El móvil espera 30 s fijos tras un error

`NubeRealtime.iniciar()` usa 2 s o 30 s fijos, sin espera creciente ni
reintento al volver la red. `PuntoConexion.kt` ya escucha
`ConnectivityManager` para el indicador; se puede reutilizar.

- Datos: 1 525 intentos, 1 375 suscripciones, 57 errores (42 `NoAutenticado`,
  13 `Interno`, 2 por tiempo de suscripción). ~100 conexiones por día por
  teléfono, que es lo esperado con el canal sólo en primer plano.

### R6. Un aviso fuera de orden puede revivir una fila

`guardar_ingreso_remoto` (`src/nube/sincronizacion/ingresos.rs`) hace
`INSERT OR REPLACE` sin comparar `updated_at`. Un aviso de entrada que llegue
después del de salida dejaría a la persona "adentro" hasta el pulso (5 min
como mucho). Poco probable; se cierra comparando `updated_at`.

### R7. Las pruebas e2e del panel abren websockets reales contra producción (confirmado)

`web/e2e/panel.spec.ts` y `asistente.spec.ts` simulan HTTP con `page.route`
pero no el websocket, y guardan un token falso (`prueba.<base64>.firma`). El
panel se conecta al Realtime de producción con ese token.

- Datos: 303 errores `UnknownErrorOnChannel: :expected_header_map` en 24 h,
  todos con `sub = 00000000-0000-4000-8000-000000000001` (el usuario de
  prueba), 225 tokens distintos, en ráfagas.
- Propuesta: `page.routeWebSocket` en las pruebas. No afecta a los equipos,
  pero ensucia los registros con que se diagnostica el canal.

## Hallazgos de telemetría

### T1. La salud del canal no se mide por defecto

El evento `realtime` está en el grupo `sincronizacion`, que se enciende por
equipo y vence: no hubo datos entre el 2026-11-01 y el 2026-11-14.

- Propuesta: pasar lo mínimo (intentos, caídas, motivo, minutos conectado) a
  un grupo siempre activo y dejar latencia y detalle como opcional.

### T2. En el escritorio los errores se registran como `"string"`

`tipoDeError` (`telemetriaCalculos.ts`) devuelve `"string"` para los rechazos
de Tauri: 536 de 702 errores del canal. `llamada_nucleo` ya clasifica con
`clasificarErrorComando` (`sin_red`, etc.): de 872 errores de
`sesion_realtime_nube`, los 344 posteriores al 2026-10-20 salen como
`sin_red`. Reutilizarlo en el canal.

### T3. `CHANNEL_ERROR` no dice la causa

No se distingue si el socket no abrió, si se rechazó la unión, si faltó el
latido o si cerró el servidor; el mensaje sólo va a `console.info`.

- Propuesta: registrar la fase, la clase del mensaje, `navigator.onLine` y si
  la ventana estaba visible.
- Sin explicar por esto: 3 de 36 caídas ocurren a 1,2 min exactos de
  conectar; al menos dos, justo después de una renovación (2026-10-19 18:59
  y 20:32 UTC).
- Hipótesis sin verificar: con la ventana minimizada, WebView2 puede frenar
  los temporizadores y atrasar el latido. Si los datos lo confirman,
  realtime-js tiene la opción `worker: true`.

### T4. El móvil casi nunca registra el fin de conexión

`Telemetria.realtime?.terminado(...)` está después del `withTimeoutOrNull`;
al pasar a segundo plano la corrutina se cancela y no se anota.

- Datos: 9 fines para 1 375 suscripciones; `minutos_conectado_max` del móvil
  no sirve (máximo 2,4 min).
- Propuesta: anotarlo en un `finally`, con motivo `segundo_plano`.

### T5. Falta la métrica que mide la robustez

No se registra cuántos cambios encontró la sincronización posterior a una
reconexión (avisos perdidos). Además hay latencias negativas (−1 042 ms en
escritorio, −1 781 ms en móvil): conviene marcar las muestras tomadas con el
desfase impreciso.

## Orden recomendado

1. T2, T3 y T4: cambios chicos que dan la visibilidad que falta.
2. R1 y R2: la mayor ganancia en el escritorio.
3. R3 y R7.
4. R4, R5, R6, T1 y T5.

## Qué mirar cuando haya datos nuevos

Comparar contra las cifras de arriba. En staging:

```sql
-- Resumen por plataforma desde la activación
with r as (
  select case when version_app like 'desktop-%' then 'desktop' else 'movil' end p, datos
  from public.telemetria_diagnostico
  where tipo = 'realtime' and ocurrido_en >= '2026-11-14'
)
select p, count(*) filas,
       sum((datos->>'intentos_conexion')::int) intentos,
       count(*) filter (where datos->>'ms_hasta_suscribir_max' is not null) filas_suscritas,
       percentile_cont(0.5) within group (order by (datos->>'latencia_ms_p50')::numeric) lat_p50,
       percentile_cont(0.9) within group (order by (datos->>'latencia_ms_p90')::numeric) lat_p90
from r group by p;

-- Motivos de error y de fin de conexión
with r as (
  select case when version_app like 'desktop-%' then 'desktop' else 'movil' end p, datos
  from public.telemetria_diagnostico
  where tipo = 'realtime' and ocurrido_en >= '2026-11-14'
)
select 'fin' clase, p, k motivo, sum(v::int) n
from r, jsonb_each_text(datos->'fin_de_conexion') e(k, v) group by 1, 2, 3
union all
select 'error', p, k, sum(v::int)
from r, jsonb_each_text(datos->'errores') e(k, v) group by 1, 2, 3
order by 1, 2, 4 desc;

-- R1: renovaciones a los 45 min. R2: reuniones de la librería (> 60 s)
select count(*) filter (where datos->'fin_de_conexion' ? 'renovacion') renovaciones,
       count(*) filter (where (datos->>'ms_hasta_suscribir_max')::numeric > 60000) reuniones_libreria,
       count(*) filter (where datos->'fin_de_conexion' ? 'CHANNEL_ERROR'
                          and (datos->>'minutos_conectado_max')::numeric between 1.0 and 1.5) caidas_a_1_min
from public.telemetria_diagnostico
where tipo = 'realtime' and version_app like 'desktop-%' and ocurrido_en >= '2026-11-14';
```

Preguntas a contestar con esos datos:

- ¿El arreglo de "sin red" bajó los errores `string` del escritorio?
- ¿Cuántas caídas por día por PC, y a qué horas? Cruzar con
  `sincronizacion_auto`: si el pulso funciona mientras el canal falla, el
  problema es del websocket y no de la red.
- ¿Se repiten las caídas a 1,2 min de una renovación?
- En más PC y sitios, ¿se mantienen las proporciones de R1 y R2?

En producción, los registros de Realtime (`source = 'realtime_logs'`) sólo
guardan 24 h por consulta: filtrar el `sub` de las pruebas e2e (R7) para ver
errores reales de los equipos.
