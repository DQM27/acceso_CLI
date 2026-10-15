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
   tiene que ser barato, y lo pesado se apaga en producción.

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

`version_app` conserva su formato (`1.4.9-diag+<hash> (<código>)` en el móvil,
`desktop-1.8.9-diag+<hash>` en escritorio). `dispositivo` es un UUID aleatorio
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

## Eventos nuevos o ampliados en el esquema 2

| Evento | Qué se agrega |
|---|---|
| `muestra_sistema` (móvil) | `gc_conteo`, `gc_ms`, `gc_bloqueante_conteo`, `gc_bloqueante_ms` (pausas de la recolección de basura desde la muestra anterior; las bloqueantes son las que el hilo de la app tiene que esperar), `carga_sistema_1m` (carga del sistema a un minuto, de `/proc/loadavg`; puede venir vacío si Android no deja leerlo) y `red_tipo`. |
| `llamada_nucleo` | `red_tipo` y el nombre canónico. |
| `transicion_ingreso` (móvil, nuevo) | Por cada contratista elegido: `origen` (`ocr` o `manual`), `ms_busqueda`, `ms_nucleo` (la espera de la nube), `ms_total` hasta tener el formulario del gafete y `resultado` (`formulario`, `bloqueada` o `error`). |
| `ocr_sesion` (móvil) | `ms_cierre_unbind`, `ms_cierre_modelos`, `ms_cierre_total`: lo que tarda cerrar la cámara en el hilo principal. |

## Lo que todavía no está resuelto

- **Histogramas.** `p50`/`p90` se guardan por ventana de 30 s y no se pueden
  combinar con exactitud entre ventanas; cualquier cifra que junte varias
  ventanas es una aproximación. Pendiente: guardar cubos de un histograma.
- **Costo de medir.** Falta una métrica `telemetria_costo_ms` con el tiempo que
  gasta la propia telemetría.
- **Retención.** No hay una tarea que borre lo viejo. Pendiente: conservar 30
  días.
- **Vistas de análisis** en el staging para consultar sin escribir consultas largas.

## Cómo cambiar el contrato

1. Cambiar este documento primero.
2. Si se **agrega** un campo: basta eso; no sube `esquema`.
3. Si cambia el **significado o el nombre** de un campo existente: sube
   `ESQUEMA_TELEMETRIA` en las dos plataformas (`TelemetriaCalculos.kt`,
   `telemetriaCalculos.ts` y `telemetria.rs`) y se anota acá qué cambió.
