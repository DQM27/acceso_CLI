# Contrato de la telemetría de rendimiento

Una sola definición para las dos apps (móvil y escritorio). Las filas van a la
tabla `telemetria_diagnostico` del **staging**; el detalle de cada plataforma
está en `mobile/android/docs/telemetria-diagnostico.md` y
`desktop/docs/telemetria-diagnostico.md`. Este documento fija lo que tiene que
ser igual en las dos, para que los datos se puedan consultar y comparar sin
limpiarlos antes.

## Principios

1. **Sólo números y nombres técnicos.** Nunca cédulas, nombres, placas, texto
   leído por el OCR, contenido de formularios ni mensajes de error.
2. **Un catálogo cerrado de eventos** (`tipo`), con campos fijos. Un campo
   nuevo se documenta acá antes de usarse.
3. **Valores categóricos con lista fija.** Nada de texto libre: si un campo
   dice "qué tipo de algo", sus valores posibles están en este documento.
4. **La unidad va en el nombre** (`_ms`, `_mb`, `_kb`, `_pct`).
5. **Medir no puede costar más que lo que se mide.** Todo punto de medición
   tiene que ser barato, y lo pesado va en un estudio que se enciende sólo en
   el equipo que se quiere estudiar (ver "Qué mide cada equipo").

## Columnas de la tabla y campos comunes

La tabla tiene columnas fijas: `ocurrido_en`, `dispositivo`, `sesion`,
`version_app`, `tipo` y `datos` (jsonb). **Lo nuevo va dentro de `datos`**, no
como columna, para que agregar un campo no obligue a migrar la base ni
rompa a una app vieja que sigue mandando el formato anterior.

Todo evento, de cualquiera de las dos plataformas, lleva en `datos`:

| Campo | Qué es |
|---|---|
| `esquema` | Versión de este contrato (hoy **2**). Sube cuando cambia el significado o el nombre de un campo existente, no cuando se agrega uno nuevo. |
| `plataforma` | `movil` o `escritorio`. |

`version_app` es `<versión> (<versionCode>)` en el móvil y
`desktop-<versión>+<hash>` en escritorio. Las filas de antes de la fase 2 traen
el formato viejo, con `-diag`. `dispositivo` es un UUID aleatorio
por instalación (reinstalar la app genera uno nuevo).

## Nombre canónico de las operaciones (`llamada_nucleo`)

El campo `nombre` de `llamada_nucleo` es el **nombre canónico** de la operación:

- en `snake_case`;
- sin los sufijos de versiones viejas `_con_secreto` y `_verificado`.

Así `registrarIngresoVerificado`, `registrarIngresoConSecreto` y
`registrar_ingreso` son la misma operación, `registrar_ingreso`, en las dos
plataformas y en todas las versiones. Lo hacen `nombreCanonicoOperacion` (móvil,
`TelemetriaCalculos.kt`) y `nombreCanonicoOperacion` (escritorio,
`telemetriaCalculos.ts`); las dos tienen **los mismos ejemplos de prueba**, y si
una cambia la regla la otra tiene que cambiar con ella.

| Nombre original | Canónico |
|---|---|
| `buscarContratistas` | `buscar_contratistas` |
| `registrarIngresoVerificado` | `registrar_ingreso` |
| `registrarIngresoConSecreto` | `registrar_ingreso` |
| `prepararIngresoConSecreto` | `preparar_ingreso` |
| `sincronizarConNubeConSecreto` | `sincronizar_con_nube` |
| `autenticarConSecreto` | `autenticar` |
| `registrar_ingreso` (escritorio) | `registrar_ingreso` |

Las filas anteriores al esquema 2 conservan los nombres viejos; para consultarlas
junto con las nuevas hay que normalizarlos (la vista de análisis lo hará).

## Valores fijos

`red_tipo` (en `muestra_sistema` y en cada `llamada_nucleo`):

| Valor | Cuándo |
|---|---|
| `sin_red` | No hay red. |
| `sin_internet` | Hay red pero no sale a internet (móvil). |
| `wifi` / `datos` / `ethernet` | El tipo de red (móvil). |
| `con_red` | Hay red, sin saber de qué tipo (escritorio, el navegador no lo dice). |
| `otra` | Cualquier otro caso, o el sistema no dejó leerla (móvil). |

`plataforma`: `movil` o `escritorio`.

`errores_por_tipo` (en `llamada_nucleo`, sólo en las filas que tuvieron errores):
cuántos errores hubo de cada tipo. **Lista fija**, la misma en las dos
plataformas; nunca el mensaje del error, que puede traer nombres, cédulas o
gafetes.

| Tipo | Cuándo |
|---|---|
| `sin_red` | No había conexión o la nube no respondió. |
| `no_autenticado` | No hay sesión activa o expiró. |
| `credenciales` | Credenciales inválidas, usuario inactivo o sin contraseña en el equipo. |
| `sin_autorizacion` | La sesión no está autorizada para esa operación. |
| `reloj` | El reloj del equipo está atrasado respecto al último movimiento. |
| `rechazado` | Una regla lo rechazó (acceso denegado, PRAIND vencido, sin visita vigente…). |
| `conflicto` | Ya existía: gafete asignado o no disponible, ingreso activo, cédula repetida. |
| `no_encontrado` | No existe o no está activo (contratista, gafete, registro). |
| `validacion` | Un dato obligatorio, con formato inválido o incoherente. |
| `base_de_datos` | Falla de la base de datos local. |
| `interno` | Error interno del núcleo. |
| `cancelada` | La operación se canceló. |
| `otro` | No encaja en ninguno: si crece, hay un mensaje nuevo que enseñarle al clasificador. |

En el teléfono el tipo sale de la variante de `NucleoException`
(`tipoDeErrorNucleo`); en el escritorio el núcleo devuelve el error como texto y
`clasificarErrorComando` (`telemetriaCalculos.ts`) lo lee sólo para elegir el tipo.

## Eventos nuevos o ampliados en el esquema 2

| Evento | Qué se agrega |
|---|---|
| `muestra_sistema` (móvil) | `gc_conteo`, `gc_ms`, `gc_bloqueante_conteo`, `gc_bloqueante_ms` (pausas de la recolección de basura desde la muestra anterior; las bloqueantes son las que el hilo de la app tiene que esperar), `carga_sistema_1m` (carga del sistema a un minuto, de `/proc/loadavg`; puede venir vacío si Android no deja leerlo) y `red_tipo`. |
| `llamada_nucleo` | `red_tipo`, el nombre canónico y, si hubo errores, `errores_por_tipo`. |
| `transicion_ingreso` (móvil, nuevo) | Por cada contratista elegido: `origen` (`ocr` o `manual`), `ms_busqueda`, `ms_nucleo` (la espera de la nube), `ms_total` hasta tener el formulario del gafete y `resultado` (`formulario`, `bloqueada` o `error`). |
| `ocr_sesion` (móvil) | `ms_cierre_unbind`, `ms_cierre_modelos`, `ms_cierre_total`: lo que tarda cerrar la cámara en el hilo principal. |

## Qué mide cada equipo: estudios por equipo

Todos los builds llevan el mismo código de medición, sin flags ni variantes de
compilación. Qué eventos manda cada equipo lo decide su **orden de
telemetría**, que el administrador cambia desde la página "Telemetría" del
panel web.

- **Grupos.** Cada evento pertenece a un grupo. El catálogo único está en
  `src/telemetria/grupos.rs` (`CATALOGO`) y lo usan las dos plataformas. Un
  evento que no está en el catálogo **no se manda**: se asigna a un grupo ahí
  antes de usarse.

  | Grupo | Eventos | Cuándo va |
  |---|---|---|
  | `base` | `sesion_inicio`, `arranque`, `salida_anterior`, `cierre`, `panic`, `error_js`, `error_telemetria`, `telemetria_descartada`, `orden_telemetria` | Siempre; no se puede apagar |
  | `rendimiento` | `llamada_nucleo`, `transicion_ingreso`, `pantalla` | Por defecto (sin orden) |
  | `fluidez` | `frames`, `ui_bloqueos` | Desde el panel |
  | `sincronizacion` | `realtime`, `sincronizacion_auto`, `sesion_unidad`, `reloj` | Desde el panel |
  | `recursos` | `muestra_sistema`, `muestra_webview`, `memoria_baja` | Desde el panel |
  | `escaneo` | `ocr_sesion`, `camara_info`, `correccion_mrz` | Desde el panel |
  | `depuracion` | `strictmode`, `retencion` | Desde el panel, por poco tiempo |

- **La orden.** Una fila por equipo en `telemetria_ordenes` (producción, por
  migración): los grupos además de `base`, el vencimiento, una nota y una
  versión que pone la nube. Sin fila, el equipo mide `base` y `rendimiento`.
  Escribe sólo el administrador del panel; cada equipo lee sólo la suya. Quién
  cambió qué queda en `telemetria_ordenes_historial`.
- **Cómo le llega.** Al instante por Realtime (evento `telemetria_config` en
  el canal `sitio:<id>`) y, de respaldo, en cada sincronización. El equipo
  aplica sólo una versión más nueva que la que tiene y la guarda en su base
  local, así que la conserva al reiniciar.
- **Vencimiento.** Lo hace cumplir el propio equipo con su hora confiable,
  aunque no tenga red: al vencer vuelve a `base` y `rendimiento`.
- **Confirmación.** El equipo manda `orden_telemetria` (grupo `base`) al
  arrancar y cada vez que cambia la orden, con `version` (`0` = sin orden) y
  `grupos` (los vigentes, con `base`: sin orden llega `["base", "rendimiento"]`). Es la forma de saber si la orden llegó.
- **Ambiente.** `sesion_inicio` del móvil lleva `ambiente` (`produccion` o
  `staging`) para separar en los datos la app de pruebas de la real.

- **`correccion_mrz`** (móvil, grupo `escaneo`): una por cada corrección de
  confusables de un MRZ confirmado, con `campo`, `posicion`, `leido` y
  `corregido` (un carácter suelto). Antes iba a Sentry como mensaje.

## Registro de errores

Aparte de la telemetría: **siempre encendido**, no depende de la orden ni se
apaga desde el panel. Vive en el núcleo (`src/errores/`) y lo usan las dos
plataformas, sin el SDK de Sentry. Cada error se escribe en el momento en
`errores_pendientes/` (un JSON por error, tope 50, junto a la base local) y un
hilo propio lo manda cada minuto a `receptor-errores` (staging):

```text
POST https://<proyecto>.supabase.co/functions/v1/receptor-errores/v1/errores
x-clave-errores: <clave>
{"reportes": [ ...hasta 10... ]}
```

La dirección y la clave salen del DSN de siempre (`SENTRY_DSN` al compilar;
`SENTRY_DSN_MOVIL` en Gradle): `https://<clave>@<proyecto>.supabase.co/functions/v1/receptor-errores/1`.
Sin DSN, los errores quedan en el equipo.

Cada reporte (`Reporte` en `src/errores/formato.rs`, `esquema` 1):

| Campo | Qué es |
|---|---|
| `id` | UUID aleatorio: el receptor ignora un reintento repetido. |
| `ocurrido_en` | UTC con milisegundos. |
| `plataforma` | `escritorio` o `movil`. |
| `version_app` | La misma de la telemetría (`desktop-1.10.7+abc1234`, `1.8.13 (58)`). |
| `origen` | `panico` (Rust), `excepcion` (Kotlin sin atrapar), `anr`, `nativo`, `js` o `nucleo` (error inesperado del núcleo o del lado Rust del escritorio). |
| `titulo` | Mensaje del error, limpio de datos personales (`errores::limpiar_texto`, las mismas reglas que el receptor). |
| `traza` | Hasta 30 marcos, del más nuevo al más viejo: `funcion`, `archivo` (sin el usuario del sistema), `linea` y `propio`. En un pánico el primero es el lugar exacto, aunque el release no tenga símbolos. |
| `pantalla` | La que estaba viendo quien opera. |
| `operaciones` | Las últimas 20: `nombre` (comando del escritorio o llamada al núcleo del móvil), `ok` y `en`. Nunca datos. |
| `contexto` | Sólo datos del equipo: `os`, `os_version`, `modelo`, `fabricante`, `arquitectura`, `android_sdk`, `ambiente` (`produccion`, `staging` o `desarrollo`). |

Un mismo error (título sin números) se reporta como mucho 3 veces por sesión
y una sesión no pasa de 100. El receptor limpia otra vez, agrupa por huella
(plataforma, origen, título sin números y los últimos tres marcos propios) y
guarda en `errores_app` (`supabase/scripts/errores_app_staging.sql`).

## Lo que todavía no está resuelto

- **Histogramas.** `p50`/`p90` se guardan por ventana de 30 s y no se pueden
  combinar con exactitud entre ventanas; cualquier cifra que junte varias
  ventanas es una aproximación. Pendiente: guardar cubos de un histograma.
- **Costo de medir.** Falta una métrica `telemetria_costo_ms` con el tiempo que
  gasta la propia telemetría.
- **Retención.** No hay una tarea que borre lo viejo. Pendiente: conservar 30
  días.
- **Vistas de análisis** en el staging para consultar sin escribir consultas largas.
- **Restos del interruptor viejo.** La tabla `telemetria_reglas` y la función
  `reglas_telemetria_vigentes` del staging (`supabase/scripts/telemetria_reglas_staging.sql`)
  ya no las consulta ninguna app desde la fase 2. Se pueden borrar cuando todos
  los escritorios tengan instalada la primera versión con la fase 2 (la 1.9.0
  todavía no la trae).
- **Confirmación en el panel.** El panel no muestra qué versión aplicó cada
  equipo: `orden_telemetria` va a `telemetria_diagnostico` del staging, y ahí
  `dispositivo` es el UUID de la instalación, no `dispositivos.id`. Por ahora
  se consulta en Grafana.

## Cómo cambiar el contrato

1. Cambiar este documento primero.
2. Si se **agrega** un campo: basta eso; no sube `esquema`.
3. Si cambia el **significado o el nombre** de un campo existente: sube
   `ESQUEMA_TELEMETRIA` en las dos plataformas (`TelemetriaCalculos.kt`,
   `telemetriaCalculos.ts` y `telemetria.rs`) y se anota acá qué cambió.
