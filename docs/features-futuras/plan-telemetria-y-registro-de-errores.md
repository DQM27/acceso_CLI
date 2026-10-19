# Plan: telemetría que importa y registro de errores propio

Plan del 2026-10-19. **Reemplaza** a `plan-herramienta-diagnostico.md` y a
`plan-telemetria-ajustable.md` (quedan como historia). Cuando se implemente,
`docs/telemetria-contrato.md` se reescribe a partir de este documento.

Sigue siendo una herramienta **del desarrollador**: nada de esto va al panel web
ni a la base de producción del cliente.

## Por qué hace falta

Lo que existe creció por partes y hoy no contesta las preguntas que importan.
Evidencia revisada el 2026-10-19 contra los datos de staging:

- **No se pudo saber si el HTTP/2 mejoró algo.** Los p50/p90 se guardan por
  ventana de 30 s y no se pueden combinar; el release móvil no lleva el commit
  en `version_app` (`1.5.0 (38)`); no se separa el tiempo de red del tiempo
  local, ni se sabe qué versión de HTTP usó cada petición.
- **Los errores reales no llegan al registro propio.** `errores_app` sólo tiene
  los 3 eventos de prueba del 17‑oct. Los 8 pánicos del escritorio
  (`tokio .../blocking/shutdown.rs:51`) y los 3 `ReferenceError` del 18‑oct se
  ven en la telemetría, sin mensaje ni traza. Causa: el SDK de Sentry sigue en
  las dos apps y, si el build no recibe el secreto `SENTRY_DSN` (los builds de
  prueba y de diagnóstico de `build-test-mobile.yml` no lo reciben), cae al DSN
  de respaldo de **sentry.io** (`desktop/src-tauri/src/lib.rs`,
  `mobile/android/app/build.gradle.kts`).
- **El volumen se va en lo que menos se consulta.** En 7 días, el escritorio
  mandó 10 838 filas de `muestra_sistema` y `muestra_webview` (memoria, CPU,
  heap del webview) contra 213 de `transicion_ingreso`, que es el flujo que el
  guarda sufre.
- **Dos formas de encender, una por plataforma.** El móvil se enciende al
  compilar (`TELEMETRIA_EN_PRODUCCION`, `TELEMETRIA_LIGERA`); el escritorio, con
  reglas remotas. La alerta de "equipo nuevo en vinculación" queda ciega si se
  apaga la del móvil.
- **Lógica duplicada en tres lenguajes** (`TelemetriaCalculos.kt`,
  `telemetriaCalculos.ts`, `telemetria.rs`) que tiene que mantenerse igual a mano.
- **Cinco documentos que se contradicen** (por ejemplo, el del móvil dice que el
  release no tiene telemetría).

## Las preguntas que tiene que contestar

Una métrica entra al nivel base **sólo si contesta una de estas preguntas**. Lo
demás es diagnóstico y se enciende con una regla.

| # | Pregunta | Con qué se contesta |
|---|---|---|
| P1 | ¿Cuánto espera el guarda en las operaciones clave? | Tiempo de cada operación del núcleo y de cada flujo de pantalla (ingreso, salida, salida en lote), separando red y local. |
| P2 | ¿Un cambio mejoró o empeoró? | Lo mismo de P1, agrupable por versión **y commit**, con histogramas que se combinan sin error. |
| P3 | ¿La app está fallando? | Registro de errores: pánicos, excepciones, ANR, errores de JS y errores inesperados del núcleo, con traza y agrupados. |
| P4 | ¿Los datos llegan a la nube? | Salud de la sincronización: pendientes por enviar, antigüedad del más viejo, último envío correcto, conexión en vivo. |
| P5 | ¿Alguien intenta usar la app sin permiso? | Evento de vinculación (ya existe la alerta). |

## Diseño

### Dos piezas, un solo canal

1. **Métricas** (P1, P2, P4, P5): números agregados, catálogo cerrado.
2. **Registro de errores** (P3): un evento por error, con traza y contexto.

Las dos usan el mismo transporte: cola en disco, envío por lotes y reintento.

### Todo en el núcleo Rust compartido

`control_acceso` ya lo usan el escritorio y el móvil (`mobile/rust-core`). La
telemetría vive ahí, en un módulo `telemetria` del núcleo, y cada plataforma sólo
aporta lo que el núcleo no puede ver (frames, ANR, errores de JS):

| En el núcleo (una sola vez) | En cada plataforma (adaptador delgado) |
|---|---|
| Identidad del equipo y la sesión | Ciclo de vida (primer plano, cierre) |
| Medición de cada operación del núcleo, incluido el tiempo de red | Flujos de pantalla (ingreso, salida) |
| Histogramas y agregación por ventana | Fluidez (frames congelados, bloqueos de la UI) |
| Gancho de pánicos y registro de errores | Excepciones de Kotlin, ANR, errores de JS |
| Limpiador de datos personales | |
| Cola en disco, envío, reintento y topes | |
| Evaluación de reglas | |

Así se elimina la duplicación Kotlin/TS/Rust, y las pruebas se escriben una sola
vez.

### Identidad: en todo evento

`dispositivo` (UUID por instalación), `sesion` (UUID por arranque),
`plataforma`, `version` (semver), **`commit`** (hash corto, también en
release), `build` (`release`, `diagnostico`, `debug`), `ambiente`
(`produccion`, `staging`) y `esquema`. Con el commit, el antes y el después de un
cambio se separan con `git merge-base`, sin adivinar por fechas.

### Histogramas con cubos fijos

Cada operación se agrega en una ventana de **5 minutos** como
`conteo`, `total_ms` y cubos fijos en ms:
`5, 10, 25, 50, 100, 250, 500, 1000, 2500, 5000, 10000, +inf`.
Los cubos se suman entre ventanas, equipos y versiones, así que cualquier
percentil que se calcule después es consistente. Reemplaza a los p50/p90 por
ventana, que no se pueden combinar.

### Catálogo del nivel base

| Métrica | Dimensiones | Contesta |
|---|---|---|
| `operacion`: histograma del tiempo total y del tiempo **de red** | nombre canónico, resultado (`ok` o tipo de error de la lista fija), `red_tipo` | P1, P2 |
| Peticiones HTTP de la operación y cuántas fueron HTTP/2 | nombre canónico | P2 |
| `flujo`: tiempo percibido (ingreso hasta el formulario, ingreso hasta guardado, salida, salida en lote) | flujo, origen (`ocr`/`manual`), resultado | P1, P2 |
| Fluidez: frames congelados (móvil) y bloqueos de la UI (escritorio) | pantalla | P1 |
| Arranque: ms hasta el primer frame, frío o tibio | — | P1 |
| `estado_sincronizacion` (cada 5 min con la app activa y al cambiar) | pendientes, antigüedad del más viejo, último envío correcto, conexión en vivo y reconexiones | P4 |
| `salida_anterior` (por qué murió el proceso) | motivo con nombre (incluido `PACKAGE_UPDATED`, hoy `DESCONOCIDO_16`) | P3 |
| `vinculacion` (pantalla abierta, código enviado) | — | P5 |

Volumen esperado: decenas a pocos cientos de filas por hora por equipo activo,
sólo de las operaciones que ocurrieron.

### Nivel diagnóstico (sólo con regla)

Memoria, CPU, hilos, descriptores, GC, heap del webview, cámara, detalle del
OCR, latencia de la conexión en vivo: lo que hoy se muestrea cada 30 s o 2 min.
Se enciende por equipo con una regla que vence (24 h por defecto) y sigue yendo a
`telemetria_diagnostico`. StrictMode y el vigilante de fugas siguen siendo
exclusivos del build de diagnóstico: encenderlos en caliente causaría el tirón que
se quiere medir.

### Registro de errores propio (reemplaza al SDK de Sentry)

Qué se captura:

| Origen | Cómo |
|---|---|
| Pánicos de Rust (escritorio y núcleo del móvil) | `std::panic::set_hook` encadenado al anterior: mensaje limpio, lugar y traza. Se escribe **a disco en el momento** (el proceso puede morir) y se envía después. |
| Excepciones de Kotlin | `Thread.setDefaultUncaughtExceptionHandler` encadenado al del sistema, y un manejador para corrutinas. |
| ANR, fallo nativo, falta de memoria (móvil) | `ApplicationExitInfo` al arrancar, con el volcado de hilos del ANR. |
| Errores de JS (escritorio) | `window.onerror`, `unhandledrejection` y el `ErrorBoundary`, hacia un comando del núcleo. |
| Errores inesperados del núcleo | Tipos `interno`, `base_de_datos` y de apertura. Los esperados (`sin_red`, `rechazado`, `validacion`…) sólo se cuentan en las métricas. |

Cada evento lleva identidad, título limpio, traza (marcando lo propio:
`control_acceso`, `com.brisas.controlacceso`, `uniffi.control_acceso_mobile`),
pantalla actual y **migas**: las últimas 20 operaciones (nombre, resultado,
hora), nunca datos. En el equipo, como mucho N eventos por huella y hora. La
**huella** la sigue calculando el receptor (`calcularHuella`), para poder cambiar
la agrupación sin sacar versiones.

### Entrada y almacenamiento

- **Receptores:** Edge Functions de Supabase en staging. `receptor-errores` ya
  existe; pasa a aceptar el formato propio (y sigue aceptando el de Sentry hasta
  que no queden versiones viejas en uso). Se agrega `receptor-telemetria`, con
  validación de forma, tamaño y ritmo. Después se **quita el INSERT público** de
  las tablas.
- **Tablas** (staging): `metricas_operaciones` (cubos como `int[]`),
  `estado_equipos`, `eventos_app` (arranque, salida anterior, vinculación),
  `errores_app` (ya existe; se le agregan `commit`, `build` y `migas`) y
  `telemetria_diagnostico` (sólo el nivel diagnóstico).
- **Vistas** para no escribir consultas largas: percentiles por versión y
  commit a partir de los cubos, y comparación entre dos versiones.
- **Retención:** métricas agregadas y errores, 90 días; diagnóstico, 30 días.
  Con `pg_cron`.

### Encendido

| Pieza | Por defecto | Cómo se cambia |
|---|---|---|
| Registro de errores | Encendido en todos los builds | Una regla lo puede apagar (interruptor de emergencia) |
| Métricas base | Encendidas en todos los builds | Igual |
| Diagnóstico | Apagado | Regla por equipo, con vencimiento |

Las dos plataformas leen las reglas igual (al abrir y tras el login, sin sondeo),
con la evaluación en el núcleo. Desaparecen `TELEMETRIA_EN_PRODUCCION` y
`TELEMETRIA_LIGERA`.

### Grafana (como código en `docs/grafana/`)

- **Lattis · Salud** (el de todos los días): errores nuevos y abiertos;
  pánicos, ANR y fallos por versión; equipos con sincronización atrasada; último
  reporte de cada equipo.
- **Lattis · Rendimiento:** operaciones y flujos clave (p50, p90 y p99) por
  versión y commit, red frente a local, porcentaje de HTTP/2, y una comparación de
  dos versiones lado a lado.
- **Lattis · Diagnóstico:** el detalle de las sesiones con regla.

Alertas: error nuevo (ya existe), pánico/ANR/fallo, sincronización atascada
(pendiente más viejo de más de 15 min en un equipo que reportó en los últimos
10 min) y equipo nuevo en vinculación (ya existe; deja de depender de una
variable de compilación).

## Qué se quita

- SDK de Sentry: crate `sentry`, `io.sentry:sentry-android`, `meta-data` del
  manifiesto, reglas de ProGuard, los DSN de respaldo a sentry.io y el secreto
  `SENTRY_DSN` (se reemplaza por la URL del receptor).
- Los muestreos de recursos del nivel base (pasan a diagnóstico).
- Los cálculos duplicados en Kotlin y TypeScript.
- `TELEMETRIA_EN_PRODUCCION` y `TELEMETRIA_LIGERA`.
- Los planes viejos (se archivan) y lo que sobre de los documentos por
  plataforma (quedan sólo cómo compilar el build de diagnóstico).

## Fases

Una fase por PR (o dos pequeños), con CI en verde y probada en un equipo real.

| # | Fase | Terminada cuando |
|---|---|---|
| 0 | **Arreglo inmediato de errores.** Quitar el DSN de respaldo a sentry.io (sin URL, los errores quedan en el registro local) y pasar la URL del receptor también a `build-test-mobile.yml`. | Un pánico forzado en el escritorio de diagnóstico y una excepción forzada en el móvil de diagnóstico aparecen en `errores_app` y llega el correo de "Error nuevo". |
| 1 | **Módulo `telemetria` del núcleo:** identidad, histograma, agregador, cola en disco con tope, envío con reintento, limpiador y evaluación de reglas. Sin tocar las apps. | Pruebas: la suma de histogramas es exacta; la cola sobrevive a un reinicio y respeta el tope; el limpiador quita cédula, correo, placa y rutas con usuario; las reglas cubren los casos de `telemetria.rs`. |
| 2 | **Registro de errores propio** en las dos apps y fuera el SDK de Sentry. El receptor acepta el formato propio. | Un pánico, una excepción de Kotlin y un error de JS de prueba llegan agrupados, con mensaje, traza legible y migas. Sentry ya no está en `Cargo.toml` ni en Gradle. |
| 3 | **Métricas de operaciones y flujos**, `receptor-telemetria`, tablas, vistas y tablero de Rendimiento. | El tablero compara dos versiones por commit sin SQL a mano, con tiempo de red separado y el porcentaje de HTTP/2. |
| 4 | **Salud de sincronización**, tablero de Salud y alertas. | Dejar un equipo de prueba sin red 20 min con un registro pendiente dispara "sincronización atascada". |
| 5 | **Encendido unificado:** el móvil lee reglas, los muestreos pasan a diagnóstico y se quitan las variables de compilación. | Sin regla, un equipo manda sólo base y errores; con regla, también diagnóstico, y al vencer deja de mandarlo. |
| 6 | **Cierre:** quitar el INSERT público, programar la retención, reescribir el contrato, archivar los planes viejos y borrar el código duplicado. | El contrato describe lo que hace el código y no queda escritura pública en las tablas. |

La fase 0 se puede hacer ya y no depende de las demás.

## Cómo evaluar un cambio de rendimiento

Lección del HTTP/2:

1. **Antes de cambiar**, escribir qué operación y qué cifra se espera mover
   (por ejemplo, el p90 del tiempo de red de `preparar_ingreso`).
2. Separar antes y después **por commit**, no por fecha.
3. Comparar el mismo tipo de red y, si se puede, el mismo equipo. Como mínimo
   200 llamadas por lado.
4. Si el cambio se puede apagar, ponerlo detrás de una regla y alternarlo en el
   mismo equipo: es la única comparación sin ruido de la red del día.
5. Para efectos de red, medir también en laboratorio
   (`tests/medir_conexion_nube.rs`).

## Decisiones abiertas

| Decisión | Recomendación |
|---|---|
| Errores y métricas base encendidos en los equipos del cliente por defecto | Sí. Conversarlo con el cliente antes de la fase 5. |
| Dónde viven los datos de producción medidos | En staging mientras el volumen sea chico; un proyecto propio si crece. |
| Entrada: Edge Function o Worker de Cloudflare | Edge Function: ya existe `receptor-errores`, es la misma pila y son menos piezas. |
| Ventana de agregación | 5 minutos. |

## Reglas del proyecto que aplican

Las de `AGENTS.md`: español, nunca el término prohibido para el puesto de
control, commits a nombre de Daniel Quintana sin atribuciones, y verificar antes
de afirmar.
