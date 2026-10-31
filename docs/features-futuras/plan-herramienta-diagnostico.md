# Herramienta de diagnóstico y desarrollo (telemetría, errores y Grafana)

> **Reemplazado** el 2026-10-19 por `plan-telemetria-y-registro-de-errores.md`.
> Se conserva como historia; no es la guía de trabajo.

Plan acordado el 2026-10-16. **No es una función del producto del cliente.** Es
una herramienta de desarrollo para ver cómo anda la app y encontrar problemas.
Este documento deja las decisiones tomadas, el estado de lo que ya existe y lo
que falta, para retomarlo sin repetir la discusión.

Relación con otros documentos: `docs/telemetria-contrato.md` describe lo que
manda hoy la telemetría de rendimiento y sus reglas. **Este plan relaja
deliberadamente varias de esas reglas** (ver "Lo que cambia del contrato"); el
contrato se reescribe cuando se implemente, no antes.

## Principio de separación

El panel web y la base de producción son del cliente y sirven para monitorear
**su operación**. Nada de diagnóstico ni de desarrollo va ahí, ni escondido.

| Pieza | Dónde vive | Quién accede |
|---|---|---|
| Datos de telemetría y de errores | Proyecto **staging** de Supabase | Sólo el desarrollador |
| Reglas de telemetría (interruptor remoto) | Tabla en el staging | Sólo el desarrollador |
| Pantallas y avisos | Grafana Cloud propio | Sólo el desarrollador |
| Panel web, producción, sincronización | Operación del cliente | El cliente. No se toca |

Si los errores con detalle crecen o se quiere aislarlos más, se mueven a un
proyecto Supabase aparte; Supabase Pro puede cobrar cómputo por proyecto
adicional (verificar antes).

## Estado: lo que ya existe (2026-10-16)

- **Telemetría de rendimiento** en el móvil, enviada al staging, activable por
  release con la variable `TELEMETRIA_EN_PRODUCCION` (ver el contrato).
- **Usuario de sólo lectura** `grafana_lector` en el staging, con permiso
  únicamente de `select` sobre `telemetria_diagnostico`. Script:
  `supabase/scripts/telemetria_lector_grafana_staging.sql`. La contraseña no
  está en el repo.
- **Grafana Cloud** conectado a ese usuario como fuente PostgreSQL (por el pooler
  de sesión de Supabase: la conexión directa es sólo IPv6 y Grafana Cloud no
  llega por ahí; y sin el agente de red privada, que no hace falta).
- **Tablero "Lattis · Telemetría (staging)"** (uid `lattis-telemetria`): equipos
  que reportaron, ingresos medidos, errores del núcleo, frames congelados,
  memoria, hilos y descriptores, tiempos del núcleo, errores por hora, escaneos
  por tipo de documento y fluidez por pantalla. Con selector de versión y hora
  de Costa Rica.
- **Alerta** "Equipo nuevo en la pantalla de vinculación" (carpeta
  `lattis-alertas`): avisa por correo si un equipo abre la pantalla de primer
  arranque o el escáner del código de vinculación en los últimos 15 minutos.
  **Sin probar de punta a punta**: al momento de escribir esto la regla no
  había evaluado ni una vez y no se había recibido ningún correo.
- Tablero y alerta **viven sólo en Grafana**, no en el repo todavía.

Limitaciones de la alerta: sólo ve apps que manden telemetría (si se apaga
`TELEMETRIA_EN_PRODUCCION` queda ciega) y también salta cuando se instala un
teléfono propio. El servidor de producción ya registra los códigos de vinculación
inválidos (`eventos_seguridad_dispositivos`, tipo `codigo_inexistente`), pero
**sólo cuando alguien llega a enviar un código**; abrir el escáner no deja rastro
ahí. Usar esa tabla como fuente exigiría darle a Grafana lectura sobre
producción: decisión de seguridad aparte, no tomada.

## Qué se quiere construir

Una herramienta **versátil**: que se pueda medir casi cualquier cosa cuando
hay una sospecha, sin sacar un release.

1. **Interruptor remoto con criterios.** Reglas que dicen qué equipos, qué
   eventos, con qué detalle y hasta cuándo. Siempre con vencimiento (por defecto
   24 horas) y apagado por defecto: sin regla o sin red, no se mide.
2. **Eventos libres.** La app puede mandar cualquier evento con cualquier campo
   (la columna `datos` ya es `jsonb`), sin migrar ni cambiar un contrato.
3. **Niveles.** Métricas ligeras siempre disponibles; **diagnóstico** con detalle
   (mensajes, sondas puntuales) sólo cuando una regla lo enciende. Lo pesado
   (StrictMode, vigilante de fugas) sigue sólo en el build de diagnóstico: no se
   puede encender en caliente sin causar el tirón que se quiere medir.
4. **Limpiador automático obligatorio** de todo lo que sale del teléfono.
5. **Errores con detalle por el mismo canal**, en una tabla aparte (otro ciclo de
   vida: se limpia a los 90 días y con acceso más restringido que las métricas).
6. **Grafana como pantalla y como avisos** sobre esas tablas.

### Cómo se elige a qué equipo aplica una regla

El identificador de la telemetría es un UUID aleatorio por instalación y a
propósito no se relaciona con el equipo registrado en producción. Los criterios
posibles son los que la telemetría ya conoce al iniciar sesión: **versión de la
app, plataforma, modelo, versión de Android** y ese UUID (se copia desde Grafana).

### Dónde se controla

Sin pantalla propia al principio: las reglas son filas en una tabla del staging
que se crean o apagan desde el editor SQL de Supabase (o por una sesión con el
conector). Grafana puede mostrar las reglas activas, de sólo lectura. Si hace
falta una página, sería una **consola propia** en un subdominio propio,
protegida con Cloudflare Access sólo para el desarrollador. **Nunca** en el panel
web del cliente.

La app lee sus reglas del staging con la llave pública, de sólo lectura y
recibiendo únicamente las que le corresponden. Escribir reglas exige credenciales
del desarrollador.

## Lo que cambia del contrato

- Deja de ser un catálogo cerrado de eventos y campos: se permiten eventos y
  texto libres.
- **Se mantiene, y ahora se cumple con código y no sólo con política:** nunca
  viajan sin limpiar cédulas, nombres, placas, correos, rutas con el usuario del
  sistema, texto leído por el OCR ni imágenes. Razón práctica: cualquier mensaje
  o traza puede traer una cédula sin que nadie lo decida, la tabla acepta
  inserciones con la llave pública y los datos viven fuera de producción.
- La regla "producción nunca activa la telemetría por su cuenta" cambia: el
  código de telemetría iría dentro de **todo** release, apagado, y lo enciende
  una regla. Es una decisión a conversar con el cliente, porque son sus teléfonos
  y se podría medir sin un release de por medio.
- Se actualiza el encabezado de `supabase/scripts/telemetria_diagnostico_staging.sql`
  y `docs/telemetria-contrato.md` cuando esto se implemente.

## Sentry, GlitchTip y Grafana

- **Grafana no sustituye a Sentry ni a GlitchTip**: no captura errores ni los
  agrupa ni traduce trazas. Es una pantalla sobre datos que ya existen.
- Los errores que cuenta la telemetría hoy ("Errores del núcleo") son
  **controlados**: el núcleo los devuelve y la app los atrapa. El SDK de Sentry
  sólo envía crashes y lo que el código le manda a propósito, así que **no recibe
  esos errores**. Muchos tampoco son fallos (sin red, no autenticado al cambiar de
  turno, rechazos por regla de negocio) y mandarlos a Sentry repetiría el ruido de
  los 550 eventos "No hay una sesión activa" del escritorio.
- Camino elegido: las apps siguen con el SDK de Sentry, el destino pasa a ser
  propio (Edge Function receptora + tabla de errores, con limpiador) y Grafana es
  la pantalla. Cambiar de destino (Sentry, GlitchTip o propio) es cambiar una
  dirección, no el código de las apps.
- **Sentry gratis se queda como red de seguridad** mientras tanto.
- GlitchTip (código abierto, MIT) queda como alternativa si se tuviera un
  servidor propio. Su backend es Python/Django: no se puede montar sobre Edge
  Functions; sólo sirve como referencia de diseño (agrupación, modelo de datos),
  conservando el aviso de la licencia.
- El flujo de "resuelto/asignado" de Sentry no lo da Grafana. Se cubriría con una
  columna de estado en la tabla, editada desde SQL.

## Fases

1. **Tabla de reglas en el staging y la app que las lee** (el interruptor, sin
   pantalla). Mover el código de telemetría a todo release, apagado por defecto.
2. **Limpiador** en el teléfono y en el receptor.
3. **Errores con detalle** por el mismo canal, sólo lo inesperado (`Interno`,
   `Apertura`, crashes), en su tabla.
4. **Receptor compatible con el SDK de Sentry** (Edge Function): validar el
   formato exacto de envío del SDK de Android y del de Rust antes de programarlo.
5. **Traducción de trazas minificadas del móvil** (guardar el `mapping.txt` de cada
   release y usar `retrace`). Es lo más caro; va al final.
6. Reescribir `docs/telemetria-contrato.md` como "reglas de limpieza y niveles".

## Pendientes sueltos que salieron de este trabajo

- **Posible fuga de hilos y descriptores en el móvil.** En una sesión de más de 14
  horas del A25, entre las 5:20 a. m. y las 12:37 p. m. los descriptores abiertos
  pasaron de 225 a 344 y los hilos de 119 a 174, con la memoria plana. En ese lapso
  hubo 71 pasadas a segundo plano: unos 1,7 descriptores y 0,8 hilos por vuelta.
  **Hipótesis sin probar:** la conexión en vivo con la nube se reabre cada vez que
  la app vuelve a primer plano y algo de la anterior no se cierra (sockets e hilos
  del cliente de red). Revisar dónde se cierra el cliente en `NubeRealtime.kt` al
  pasar a segundo plano. El tablero ya muestra la tendencia.
- **Sincronización sin sesión.** La sincronización automática y la conexión en vivo
  exigen un usuario con sesión; al cambiar de turno, o con la PC en el login entre
  turnos, corren igual y fallan con "no autenticado". Es la causa del problema
  número uno de Sentry en escritorio (550 eventos en 27 días) y de los errores de
  cambio de turno en el móvil. Arreglo: que no corran ni reporten mientras no haya
  sesión.
  **Escritorio: resuelto** (2026-10-17): el pulso se salta en silencio sin sesión y
  solo llega a Sentry lo inesperado, una vez por hora por mensaje
  (`desktop/src-tauri/src/sincronizacion_auto.rs`). Eran 552 avisos "No hay una
  sesión activa" en 28 días (pico de 186 el 7 de octubre) y, con los otros cuatro
  avisos de la misma ruta, más del 90 % del ruido del escritorio. **Falta el móvil.**
- **La sincronización del móvil tarda siempre entre 2 y 3 s.** No hay datos por paso
  para explicarlo; habría que medir cada paso dentro de la sincronización.
- **Destello verde en Activos del escritorio.** Es el destello de celdas cambiadas
  de AG Grid (`enableCellChangeFlash`, `Tabla.tsx`), con su color por defecto, y se
  dispara en cada recarga cuyo valor cambió (hay dos recargas al dar una salida: una
  inmediata y otra cuando termina la sincronización que dispara). No se identificó
  qué celda lo causa sin ver la app corriendo; falta la respuesta de qué parpadea
  exactamente. Opciones: destello sólo para cambios de otro equipo, o quitarlo de
  Activos.
- **`dispositivos.app_version`** en producción conserva la versión con la que se
  vinculó el equipo, no la actual.
- **Los "trabados" de la cámara están inflados en pantallas de 120 Hz** (el límite
  es el doble del presupuesto del cuadro: 16,7 ms). Importan los congelados y el
  máximo. **Resuelto en el móvil** (2026-10-19): cada cuadro se compara contra su
  propio plazo (`FrameMetrics.DEADLINE`) y la fila trae el desglose hilo de
  pantalla / render para saber dónde se va el tiempo. Las filas de versiones
  anteriores siguen infladas.
- **Alertas y tableros de Grafana:** ya están como JSON en `docs/grafana/`. Ajustar las tablas para teléfono
  (se cortan las columnas) y el gráfico de errores por hora (sale verde).
- **Regla de equipos nuevos:** confirmar que evalúa y que llega el correo. Agregar
  después la notificación a la app móvil de Grafana (por la documentación pasa por
  la integración Grafana IRM; no se confirmó cómo recibe las notificaciones la app).
- **Moverse a R2 y la página de descargas** siguen en `docs/pendientes.md`.

## Decisiones que quedan abiertas

- Si los errores con detalle van en el proyecto de staging o en uno propio.
- Si Grafana debe leer también los eventos de seguridad de producción (exige darle
  lectura sobre producción).
- Conversar con el cliente el cambio de "producción nunca mide por su cuenta" a
  "mide cuando una regla lo enciende".
