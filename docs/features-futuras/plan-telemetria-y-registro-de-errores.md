# Plan: telemetría por estudios y registro de errores

Plan del 2026-10-20. **Reemplaza** a `plan-herramienta-diagnostico.md` y a
`plan-telemetria-ajustable.md` (quedan como historia). Parte de **lo que ya
existe** y lo ordena; no se reescribe desde cero.

## Objetivo

Mejorar la app con **datos reales de producción**: los builds finales (release)
de todos los equipos en uso, sin muestreo. Dos piezas:

1. **Registro de errores:** siempre encendido, con **captura propia** (sin el
   SDK de Sentry) que manda a nuestro receptor (`receptor-errores`). Mientras se
   construye, el SDK actual sigue apuntado al receptor (ver la fase 0).
2. **Telemetría por estudios:** los eventos se agrupan por tipo de estudio, y
   cada estudio se **enciende y se apaga por unidad desde un panel web**, según
   lo que se quiera verificar, sin sacar un release.

**El mismo código en todos los builds.** Telemetría y errores no dependen de un
flag ni de un tipo de compilación: desaparecen la feature de Cargo `telemetria`,
`BuildConfig.TELEMETRIA`, `TELEMETRIA_LIGERA`, `-PtelemetriaEnProduccion` y la
variante `diagnostico`. Lo único que decide qué se mide es la orden del panel.

Los datos son del desarrollador: nada va al panel web ni a la base de operación
del cliente.

## 1. Registro de errores: captura propia

**Decisión (2026-10-20): se reemplaza el SDK de Sentry por captura propia, de una
sola vez.** No se agregan más cosas al SDK (ni el SDK de navegador ni más
`captureException`), porque se tirarían al pasar a la captura propia. Lo único
que se hace antes es la fase 0, que evita perder errores mientras tanto.

### Captura propia (a construir)

Se construye junto con el módulo de telemetría del núcleo, porque comparten cola
en disco, envío por lotes con reintento y el limpiador de datos personales.

| Origen | Cómo |
|---|---|
| Pánicos de Rust (escritorio y núcleo del móvil) | `std::panic::set_hook` encadenado al anterior: mensaje limpio, lugar y traza. Se guarda **a disco en el momento** (el proceso puede morir) y se envía después. |
| Excepciones de Kotlin | `Thread.setDefaultUncaughtExceptionHandler` encadenado al del sistema, y manejador de corrutinas. |
| ANR, fallo nativo y falta de memoria (móvil) | `ApplicationExitInfo` al arrancar, con el volcado de hilos del ANR. |
| Errores de JS (escritorio) | `window.onerror`, `unhandledrejection` y el `ErrorBoundary`, hacia un comando del núcleo. |
| Errores inesperados del núcleo | Tipos `interno`, `base_de_datos` y de apertura, en las dos apps. Los esperados (`sin_red`, `rechazado`, `validacion`…) sólo se cuentan en las métricas. |

Cada evento lleva identidad de la unidad, versión y commit, título limpio, traza
(marcando lo propio: `control_acceso`, `com.brisas.controlacceso`,
`uniffi.control_acceso_mobile`), pantalla actual y las últimas 20 operaciones
(nombre, resultado, hora; nunca datos). La huella la sigue calculando el
receptor (`calcularHuella`), para poder cambiar la agrupación sin sacar
versiones.

El **receptor se reutiliza**: pasa a aceptar sólo el formato propio. El de
Sentry se retiró de una vez (decisión del 2026-10-31): hay dos equipos y se
actualizan juntos, así que no hace falta convivir con versiones viejas. El
DSN se conserva: de él salen la dirección y la clave del receptor.

A vigilar, sólo comprobable en un release instalado: que el pánico se guarde
antes de que muera el proceso; que las trazas de Rust del release salgan con
nombres legibles; y los fallos nativos del núcleo en el móvil (hoy el SDK no
tiene soporte nativo activado, así que es un hueco que ya existe).

### Hoy: lo que hace el SDK de Sentry

### Lo que ya captura

| Plataforma | Qué captura hoy | Dónde |
|---|---|---|
| Escritorio (Rust) | Pánicos (automático) y errores inesperados marcados a mano (sincronización, apertura del núcleo, arranque) | crate `sentry` 0.49, `desktop/src-tauri/src/lib.rs` |
| Móvil (Kotlin) | Excepciones no atrapadas, ANR y fallos nativos (automático); trazas legibles (R8 no ofusca) | `io.sentry:sentry-android` 8.9.0, `AndroidManifest.xml` |
| Receptor | Limpia datos personales, agrupa por huella, limita el ritmo, guarda 90 días | `supabase/functions-staging/receptor-errores`, tabla `errores_app` |
| Aviso | Correo "Error nuevo en las apps" | Grafana |

### Lo que falla o falta

| Problema | Efecto | Arreglo |
|---|---|---|
| Si el build no recibe el secreto `SENTRY_DSN`, el código cae a un DSN de respaldo de **sentry.io** (`lib.rs`, `build.gradle.kts`). `build-test-mobile.yml` no lo pasa. | Los errores de esos builds se van al Sentry viejo. `errores_app` sólo tiene los 3 de prueba; los 8 pánicos del escritorio del 18‑oct no llegaron. | Quitar el respaldo (sin URL no se envía) y pasar el secreto en todos los workflows. Confirmar que el secreto de los release apunta al receptor. |
| El frontend del escritorio (React en el webview) no tiene SDK. | Los errores de JS sólo se cuentan en la telemetría (`error_js`), sin mensaje ni traza. | Lo cubre la captura propia. |
| En el móvil, los errores inesperados del núcleo (`interno`, base de datos) se atrapan y no se reportan. Sólo `MrzParser` llama a `Sentry.captureMessage`. | Un pánico de Rust en el móvil llega como excepción atrapada y no se registra. | Lo cubre la captura propia. |
| La telemetría del pánico guarda sólo el lugar, sin mensaje. | No se sabe qué pasó. | Lo cubre la captura propia. |

## 2. Telemetría por estudios

### Qué sirve y qué no

Basado en los datos de staging del 13 al 19‑oct (filas por semana).

| Evento | Plataforma | Filas/sem | Veredicto |
|---|---|---|---|
| `sesion_inicio`, `arranque`, `salida_anterior`, `cierre` | ambas | ~120 | **Sirve.** Cuántos equipos, qué versión, si murió el proceso. Barato. Ajuste: el motivo 16 del móvil es `PACKAGE_UPDATED` (hoy `DESCONOCIDO_16`). |
| `panic`, `error_js`, `error_telemetria` | ambas | ~20 | **Sirve como conteo** (detecta errores aunque falle el SDK). El detalle viene del SDK. |
| `llamada_nucleo` | ambas | 9 700 | **Sirve, es el centro del rendimiento.** Ajustes abajo. |
| `transicion_ingreso` | ambas | 213 | **Sirve mucho:** es la espera que vive el guarda al registrar un ingreso. |
| `pantalla` | ambas | 800 | Sirve: tiempo hasta dibujar cada pantalla. |
| `frames` (móvil), `ui_bloqueos` (escritorio) | — | 1 300 | Sirve cuando se persiguen trabas de la interfaz. |
| `realtime`, `sincronizacion_auto` | ambas | 1 770 | Sirve cuando se estudia la sincronización. |
| `reloj`, `sesion_unidad` | ambas | 2 790 | Sirve sólo para auditar el reloj confiable o la sesión única; se manda en **cada** sincronización. No va siempre. |
| `muestra_sistema`, `muestra_webview` | ambas | 11 360 | **El mayor volumen** (35 % de las filas) y lo menos consultado. Sólo para estudiar memoria, CPU o batería. |
| `memoria_baja` | móvil | 430 | Mayormente ruido del paso a segundo plano. Sólo en el estudio de recursos, y filtrar niveles. |
| `ocr_sesion`, `camara_info` | móvil | 380 | Sirve para calibrar el escáner. `camara_info` repite datos fijos: basta una vez por versión y equipo. |
| `strictmode`, `retencion` | móvil | ~20 | Sirven para cazar fugas, pero cuestan (el vigilante fuerza recolecciones de basura). Grupo propio, sólo por poco tiempo y en una unidad. |

### Los grupos

| Grupo | Eventos | Para qué estudio | Por defecto |
|---|---|---|---|
| `base` | `sesion_inicio`, `arranque`, `salida_anterior`, `cierre`, `panic`, `error_js`, `error_telemetria` | Estado de la flota: equipos, versiones, caídas | **Siempre** (no se apaga) |
| `rendimiento` | `llamada_nucleo`, `transicion_ingreso`, `pantalla` | ¿Cuánto espera el guarda? ¿Un cambio mejoró? | **Encendido** (recomendado: es la línea base) |
| `fluidez` | `frames`, `ui_bloqueos` | ¿Se traba la interfaz? | Desde el panel |
| `sincronizacion` | `realtime`, `sincronizacion_auto`, `sesion_unidad`, `reloj` | ¿Llegan los datos? ¿Latencia entre equipos? ¿Reloj? | Desde el panel |
| `recursos` | `muestra_sistema`, `muestra_webview`, `memoria_baja` | Memoria, CPU, batería, fugas | Desde el panel |
| `escaneo` | `ocr_sesion`, `camara_info` | Calibrar OCR y cámara | Desde el panel |
| `depuracion` | `strictmode`, `retencion` | Cazar fugas (cuesta: avisarlo en el panel) | Desde el panel, con vencimiento corto |

**Por qué `rendimiento` encendido siempre:** para contrastar con datos
anteriores o con otra unidad, esos datos tienen que existir **antes** de que el
cliente diga "está lenta". Agregado por ventanas es barato (decenas de filas por
hora por unidad activa). Los estudios detallados se encienden cuando hacen falta.

Un evento nuevo se asigna a un grupo antes de usarse.

### Cómo se encienden: panel → nube → unidad → núcleo

Los errores **no** pasan por aquí: están siempre encendidos y no se pueden
apagar desde internet (sería un riesgo y sólo actúan cuando hay un error).

#### 1. En la nube (producción, por migración)

Tabla `telemetria_unidades`, una fila por unidad:

| Columna | Qué es |
|---|---|
| `dispositivo_id` | La unidad (`public.dispositivos`). |
| `grupos text[]` | Estudios encendidos, de la lista fija (`check`). `base` no se guarda: va siempre. |
| `vence_en` | Hasta cuándo (nulo = sin vencimiento). La unidad lo hace cumplir sola, aunque no tenga red. |
| `version bigint` | Sube en cada cambio. La unidad aplica sólo una versión más nueva que la que tiene. |
| `nota`, `actualizado_en`, `actualizado_por` | Por qué y quién. |

Y `telemetria_unidades_historial` (quién encendió qué y cuándo), para cruzar un
cambio en los datos con la orden que lo causó.

Seguridad, con RLS:

- **Escribe sólo el rol de desarrollador** del panel. Ni las unidades ni los
  administradores del cliente.
- **Cada unidad lee sólo su propia fila**, con la misma autenticación con la que
  ya sincroniza.
- Lo peor que puede hacer una orden es encender mediciones: no toca datos de
  operación ni apaga los errores.

#### 2. El panel

Una página "Telemetría" con la lista de unidades (la misma de `Dispositivos`):
casillas por grupo, vencimiento, nota y el estado aplicado. Muestra también
**qué versión de la orden confirmó cada unidad** y cuándo (lo dice la propia
unidad en su evento `base`), para saber si ya le llegó.

#### 3. Cómo le llega a la unidad (sin consultas extra)

Dos vías, las dos ya existen:

1. **Tiempo real (al instante).** Un trigger en `telemetria_unidades` hace
   `realtime.send('telemetria_config', 'sitio:<sitio_id>')` con
   `dispositivo_id`, `grupos`, `vence_en` y `version`, igual que hoy
   `avisar_dispositivo_expulsado`. La unidad ya escucha ese canal privado; sólo
   atiende el aviso si `dispositivo_id` es el suyo.
2. **Sincronización (respaldo).** En la sincronización que ya corre, el núcleo
   lee su fila (una consulta pequeña). Cubre la unidad que estaba apagada o sin
   red cuando se dio la orden, o que perdió el aviso.

No hay sondeo propio: si el aviso llega, se aplica en segundos; si no, en la
siguiente sincronización.

#### 4. En la unidad

```
aviso o sincronización
        │  (sólo si version > la guardada)
        ▼
SQLite local: tabla config_telemetria (una fila)
        │
        ▼
memoria del núcleo: grupos encendidos + vence_en
        │
        ├─► cada punto de medición del núcleo pregunta
        │   "¿está encendido rendimiento?" → lectura en memoria
        │
        └─► aviso a la plataforma (callback en el móvil,
            evento de Tauri en el escritorio) para encender o
            apagar lo que mide ella (frames, webview)
```

- **La base local no se consulta en cada medición.** Se lee al abrir el núcleo
  y se escribe sólo cuando llega una orden nueva. Cada punto de medición lee un
  valor en memoria (una lectura atómica y una comparación de hora): no cuesta
  nada medible.
- **Al arrancar**, el núcleo carga la última orden guardada: la unidad mide desde
  el primer segundo, aunque no tenga red.
- **Al vencer**, el núcleo apaga el grupo solo y avisa a la plataforma.
- **Kotlin y TypeScript no guardan nada:** preguntan al núcleo al arrancar
  (`config_telemetria()`) y escuchan el aviso de cambio.
- **La unidad confirma** la versión aplicada en el siguiente evento `base`; el
  panel lo muestra.

#### Ejemplo: "el cliente dice que la app está lenta"

1. En el panel, a esa unidad: `fluidez` y `sincronizacion` por 48 h, con nota
   "lentitud reportada por el cliente".
2. La unidad lo recibe en segundos y lo confirma.
3. En Grafana: su `rendimiento` de hoy contra el de la semana pasada (que existe
   porque está siempre encendido) y contra otra unidad del mismo sitio.
4. A las 48 h se apaga sola.

### Ajustes del grupo `rendimiento`

Lo que impidió saber si el HTTP/2 mejoró algo:

- El release del móvil no lleva el commit en `version_app` (`1.5.0 (38)`); el
  escritorio sí (`desktop-1.8.15+15c0346`). Agregarlo.
- `p50`/`p90` por ventana de 30 s no se pueden combinar. Agregar cubos fijos de
  histograma (`5, 10, 25, 50, 100, 250, 500, 1000, 2500, 5000, 10000` ms) a
  `llamada_nucleo`; los p50/p90 se quedan para no romper los tableros.
- Separar el tiempo de red del local y contar cuántas peticiones fueron HTTP/2.

## Grafana

- Una sección por grupo en el tablero de telemetría, con selector de unidad y de
  periodo para comparar una unidad contra sí misma (semana anterior) o contra
  otra.
- El tablero de errores sobre `errores_agrupados`.
- Alertas que siguen: error nuevo y equipo nuevo en vinculación. Esta última
  queda firme porque `base` va siempre en todos los builds.

## Lo que se retira

- La feature de Cargo `telemetria` y el build de diagnóstico del escritorio.
- `BuildConfig.TELEMETRIA`, `TELEMETRIA_LIGERA`, `-PtelemetriaEnProduccion`, la
  variable `TELEMETRIA_EN_PRODUCCION` y la variante `diagnostico` del móvil.
- `telemetria_reglas` y `reglas_telemetria_vigentes` de staging (las reemplaza
  `telemetria_unidades` en producción). La evaluación de vencimiento y la
  combinación de grupos, con sus pruebas, pasan de `desktop/src-tauri/src/telemetria.rs`
  al núcleo compartido, para que el móvil y el escritorio usen la misma.
- El DSN de respaldo a sentry.io (en la fase 0).
- Con la captura propia (hecho): el crate `sentry`, `io.sentry:sentry-android`,
  sus `meta-data` del manifiesto y el formato de Sentry del receptor. El
  secreto `SENTRY_DSN` se queda: de él salen la dirección y la clave del
  receptor (se podría renombrar, pero obliga a tocar los tres workflows).

## Fases

Cada fase termina con **el release instalado en los equipos reales**.

| # | Fase | Terminada cuando |
|---|---|---|
| 0 | **Errores al receptor (parche, mientras llega la captura propia):** quitar el DSN de respaldo a sentry.io; secreto en todos los workflows; confirmar el de los release. | Un error de prueba del release de cada plataforma llega a `errores_app` y llega el correo. |
| 1 | **Orden en la nube:** migración con `telemetria_unidades`, su historial, RLS (escribe sólo el desarrollador, cada unidad lee la suya) y el trigger que avisa por tiempo real. | Pruebas de RLS: una unidad no lee la fila de otra ni puede escribir; un cambio emite `telemetria_config` en el canal del sitio. |
| 2 | **Orden en la unidad:** tabla local `config_telemetria` (migración del esquema), estado en memoria del núcleo, lectura en la sincronización, atención del aviso en las dos plataformas y confirmación en `base`. Todos los puntos de medición preguntan al núcleo; fuera los flags y las variantes de compilación. | En un release: encender `fluidez` en una unidad empieza a mandar `frames` en segundos; apagarlo los detiene; una unidad sin red al momento de la orden la aplica en la siguiente sincronización; al vencer se apaga sola. |
| 3 | **Panel:** página "Telemetría" para el rol de desarrollador, con el estado confirmado por cada unidad. | Encender y apagar desde el panel sin SQL, y ver la confirmación. |
| 4 | **Captura propia de errores:** módulo del núcleo con la cola compartida, gancho de pánicos, excepciones de Kotlin, `ApplicationExitInfo`, errores de JS y del núcleo; el receptor acepta el formato propio. Se retira el SDK. **Código hecho (2026-10-31, rama `feat/errores-captura-propia`); falta desplegar el receptor y probarlo en un release.** El formato está en `docs/telemetria-contrato.md` ("Registro de errores"). | En un release instalado, un pánico, una excepción de Kotlin, un error de JS y un error interno del núcleo llegan agrupados, con mensaje, traza legible y últimas operaciones. |
| 5 | **Ajustes de `rendimiento`:** commit en la versión del móvil, cubos de histograma, red frente a local y HTTP/2. | El tablero compara una unidad contra su semana anterior y contra otra unidad. |
| 6 | **Recortes:** `camara_info` una vez por versión, niveles de `memoria_baja`, motivo 16 con nombre, retención programada, tablero por grupos, contrato reescrito. | `docs/telemetria-contrato.md` describe los grupos y lo que hace el código. |

La fase 0 se puede hacer ya y no depende de las demás.

## Decisiones abiertas

| Decisión | Recomendación |
|---|---|
| Dónde vive el panel | Una página del panel web existente, visible sólo para un rol de desarrollador: reutiliza el inicio de sesión y la lista de unidades. La seguridad la da RLS, no esconder la página. Alternativa: una web aparte si no se quiere nada de esto junto al panel del cliente. |
| `rendimiento` encendido siempre | Sí (es la línea base para comparar). |
| Avisar al cliente que sus equipos mandan métricas técnicas y errores (sin datos personales) | Sí, antes de encender estudios en toda la flota. |
| Dónde viven los datos medidos | Staging por ahora; un proyecto de Supabase propio de observabilidad si el volumen crece. La orden (`telemetria_unidades`) va en producción porque ahí se autentican las unidades. |

## Reglas del proyecto que aplican

Las de `AGENTS.md`: español, nunca el término prohibido para el puesto de
control, commits a nombre de Daniel Quintana sin atribuciones, y verificar antes
de afirmar.
