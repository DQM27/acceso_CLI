# Telemetría de diagnóstico

El teléfono de la portería (Samsung A25) tiene bloqueada la depuración USB,
así que `adb logcat` y el profiler de Android Studio no están disponibles.
Para medir cómo trabaja la app en el equipo real, la app manda métricas
técnicas a la tabla `telemetria_diagnostico` del proyecto de Supabase
**staging**.

El escritorio manda a la misma tabla, con los mismos nombres de evento y de
campo (`version_app` empieza con `desktop-`): ver
`desktop/docs/telemetria-diagnostico.md`.

## Qué se mide

Un solo código para todos los builds, sin flags ni variantes (ya no existen
`BuildConfig.TELEMETRIA`, `TELEMETRIA_LIGERA`, `-PtelemetriaEnProduccion` ni el
build `diagnostico`). Cada tipo de evento pertenece a un grupo y sólo se mide
si la orden vigente de este teléfono lo permite:

- `base` (arranque, salidas anteriores, errores) va **siempre**;
- `rendimiento` (`llamada_nucleo`, `transicion_ingreso`, `pantalla`) va **por
  defecto**;
- `fluidez`, `sincronizacion`, `recursos`, `escaneo` y `depuracion` los enciende
  el administrador **por equipo** desde el panel, con vencimiento.

El catálogo evento -> grupo está en el núcleo
(`control_acceso::telemetria::CATALOGO`, expuesto como `telemetriaCatalogo()`)
y es el mismo del escritorio. Los grupos que rigen ahora salen de
`Nucleo.telemetriaGruposVigentes()`, que sólo lee la copia en memoria; la app
los vuelve a leer en cada muestra (30 s, así un vencimiento se nota solo), al
llegar una orden y tras cada sincronización. La orden viaja panel → nube →
teléfono (aviso en vivo `telemetria_config` o sincronización) y el núcleo la
guarda: al abrir la app rige aunque no haya red.

- `depuracion` activa StrictMode y el vigilante de fugas, que cuestan: sólo por
  poco tiempo. StrictMode, una vez activado, queda hasta reiniciar la app; lo
  que detecta sólo se anota mientras rija el grupo.
- Hasta que el núcleo abre rige el estado por defecto, y hasta que llega el
  catálogo (unos milisegundos después de `Telemetria.iniciar`) no se mide
  nada.
- Los builds de prueba (`debug`, staging) y el de producción (`release`) miden
  igual: lo que cambia es el ambiente (`ambiente` en `sesion_inicio`).

## Privacidad

Sólo números y nombres técnicos (nombres de pantalla, de operación del
núcleo, de clase). **Nunca** texto leído por el OCR, cédulas, nombres,
placas ni imágenes. El dispositivo se identifica con un UUID aleatorio por
instalación (no ANDROID_ID ni IMEI), y la sesión con otro por arranque. De
los motivos de salida del proceso se guarda sólo el resumen antes de `:`
(el resto puede ser el mensaje de una excepción).

## Eventos (`tipo`)

> **Hora de los eventos** (`ocurrido_en`): sale del reloj confiable del núcleo
> (hora del servidor + contador de arranque), la misma con que se sellan los
> registros, no del reloj del teléfono. Así los tiempos de todos los equipos se
> comparan entre sí y con la bitácora. Antes de abrir el núcleo (los primeros
> eventos del arranque) se usa la del sistema.

| Tipo              | Cuándo                                | Datos principales |
|-------------------|---------------------------------------|-------------------|
| `sesion_inicio`   | al arrancar el proceso                | fabricante, modelo, Android, ABI, núcleos, RAM total, clase de memoria |
| `reloj`           | después de cada sincronización        | reloj confiable (hora del servidor + contador de arranque): `confiable`, `margen_ms`, `diferencia_equipo_ms` (reloj del teléfono menos la hora que usa la app; un salto grande sin conexión = alguien movió la hora) y `ancla_hace_ms` |
| `sesion_unidad`   | en cada sincronización con sesión     | qué respondió la nube a la sesión única (`vigente`, `desplazada`, `sin_usuario` o `error`) y `transcurrido_ms` desde el ingreso; sin cédula ni nombre |
| `arranque`        | primer frame dibujado                 | `ms_desde_creacion_actividad` (siempre); `en_frio` y, sólo en frío, `ms_desde_inicio_proceso` (si el proceso ya vivía -- lo despertó un push -- contaba las horas en segundo plano); `primero_de_la_version` (primer arranque tras instalar una versión, el más lento); estado del Baseline Profile (`perfil_compilado`, `perfil_en_cola`, `perfil_codigo`); en Android 15+ tipo y motivo del arranque (frío, tibio, caliente) |
| `salida_anterior` | al arrancar, una vez por salida       | por qué murió el proceso antes (ANR, memoria, crash nativo/Java, usuario), PSS/RSS |
| `pantalla`        | al entrar a cada pantalla             | ms hasta su primer frame |
| `muestra_sistema` | cada 30 s en primer plano             | heap Java y nativo, PSS por tipo (Java, nativo, gráficos, código), memoria libre del sistema, hilos, descriptores abiertos, CPU (% de un núcleo y del total), red rx/tx, batería (%, corriente, temperatura, carga, ahorro), estado y margen térmico, pantalla visible |
| `frames`          | cada 30 s, por pantalla               | frames, trabados (> 2× el presupuesto de ESE frame: `FrameMetrics.DEADLINE` en Android 12+, porque la pantalla cambia sola entre 120 y 60 Hz), congelados (> 700 ms), p50/p90/p99/máx, `presupuesto_ms` (mediana), y el desglose `hilo_ui_p50/p90_ms` (código de Compose) y `render_p50/p90_ms` (GPU y entrega del búfer) |
| `llamada_nucleo`  | cada 30 s, por operación              | llamadas, errores, p50/p90/máx/total ms de cada llamada al núcleo Rust |
| `ocr_sesion`      | al cerrar una cámara de escaneo       | fps, frames, descartes por calidad, fallos, medianas de recorte y reconocimiento, ms hasta confirmar |
| `camara_info`     | una vez por apertura de cámara, con la cámara ya abierta | qué puede dar la cámara: nivel de hardware, resolución real del análisis y del preview, tamaño YUV y JPEG máximos, tamaño del sensor, zoom máximo y distancia mínima de enfoque (ver abajo) |
| `realtime`        | cada 30 s, si hubo actividad          | conexión (intentos, ms hasta suscribir, fin de cada conexión y minutos conectado, errores), avisos por tabla, ecos propios, KB recibidos, aplicados en la base local y su tiempo, y latencia (ver abajo) |
| `strictmode`      | cada 30 s, agregado                   | disco/red en el hilo principal, recursos sin cerrar, Activities filtradas, con el punto del código de la app |
| `memoria_baja`    | cuando el sistema pide liberar memoria| nivel de `onTrimMemory`, pantalla |
| `retencion`       | objeto vivo 10 s después de liberarlo | posible fuga: `MainActivity` destruida o `EstadoCamaraOcr` cerrado que siguen en memoria tras forzar el GC |
| `error_telemetria`| si falla la propia telemetría         | dónde y tipo de excepción |

## PDF417 (desconectado)

El lector del PDF417 de la cédula anterior se desconectó de la app el
2026-10-19 (en la práctica no se usaba). Las filas de `ocr_sesion` de
versiones anteriores traen los campos `frames_con_codigo`, `pdf417_*`,
`modo_codigo_*`, `imagen_ancho_px_mediana_con_codigo` y `lector_pdf417`; las
nuevas ya no. El decodificador sigue en el núcleo Rust
(`mobile/rust-core/src/pdf417_cedula.rs`).

## Qué puede dar la cámara (`camara_info`)

Con qué resolución quedaron el análisis y el preview, y cuánto más podría
dar la cámara, con sólo números:

| Campo | Qué dice |
|---|---|
| `pantalla` | Pantalla de escaneo que abrió la cámara |
| `hardware_nivel` | `INFO_SUPPORTED_HARDWARE_LEVEL` de Camera2: `LEGACY`, `LIMITED`, `FULL`, `LEVEL_3`, `EXTERNAL` |
| `analisis_ancho` / `analisis_alto` | Resolución REAL elegida para el análisis (`resolutionInfo`; se pide 1920x1080) |
| `preview_ancho` / `preview_alto` | Resolución real del preview |
| `yuv_max_ancho` / `yuv_max_alto` | Mayor tamaño de salida YUV_420_888 |
| `yuv_alta_resolucion_max_ancho` / `_alto` | Mayor tamaño YUV que sólo se da a menor velocidad (sensores de 50 MP); nulo si no hay |
| `jpeg_max_ancho` / `jpeg_max_alto` | Mayor tamaño JPEG |
| `sensor_ancho` / `sensor_alto` | Matriz de píxeles del sensor |
| `zoom_max` | Zoom máximo (razón) |
| `enfoque_min_dioptrias` / `enfoque_min_cm` | Distancia mínima de enfoque; nulos con enfoque fijo |

Cómo leerlo: si `analisis_*` es menor que `yuv_max_*`, se puede pedir más
resolución al análisis (a costa de fps); si ya son iguales, ese es el techo.
`enfoque_min_cm` dice a qué distancia deja de enfocar un documento acercado.

```sql
select ocurrido_en, version_app, datos
from public.telemetria_diagnostico
where tipo = 'camara_info'
order by ocurrido_en desc;
```

## Condiciones de captura, texto y placas (en `ocr_sesion`)

Para calibrar los umbrales con datos reales, de día y de noche. Todo son
números o nombres de formato; nunca el texto, la placa ni la imagen.

| Campo | Qué dice | Sirve para ajustar |
|---|---|---|
| `abierta_desde` | Pantalla desde la que se abrió la cámara (`proveedores`, `activos`...) | Separar los casos de uso |
| `frames_medidos` / `frames_con_linterna` | Frames medidos (también los descartados) y cuántos con linterna | Uso real de la linterna |
| `nitidez_p10/p50/p90` | Nitidez del recorte (relativa: comparar sesiones del mismo teléfono) | Filtro de calidad (`FiltroCalidad`) |
| `luminancia_p10/p50` | Brillo medio del recorte, 0-255 | Distinguir noche/día; sugerir la linterna |
| `reflejo_p50/p90` | Fracción de píxeles saturados | Aviso de reflejo (`UMBRAL_REFLEJO`, 0,03) |
| `palabras` / `palabras_confianza_baja` / `confianza_p10/p50` / `frames_sin_confianza` | Confianza de las palabras de ML Kit | Umbral de los renglones visuales (0,25) |
| `frames_textos_distintos` | Frames en que los renglones visuales difieren del texto original | Cuánto cambia E-3 |
| `vehiculo_formatos` | Lecturas por formato: `CARGA`, `PARTICULAR`, `MOTO`, `NUMERO_UNIDAD` | Qué placas llegan |
| `vehiculo_con_correccion` / `vehiculo_cl_restituida` | Lecturas que corrigieron letra/dígito o restituyeron la "CL" apilada | Máximo de correcciones por placa |
| `vehiculo_por_version_texto` | Lecturas desde los renglones visuales o el texto original | Si E-3 ayuda en placas |
| `vehiculo_frames_sin_lectura` | Frames con texto y sin placa | Cuánto cuesta leer (noche, distancia) |
| `reinicios_votacion` | Veces que la votación empezó de cero por otra lectura | Tolerancia de la votación |
| `ms_hasta_primera_lectura` | Primer frame con algo reconocido (placa, documento con datos o MRZ) | Separar "tardó en encontrarlo" de "tardó en confirmarlo" |
| `documento_estados` | Frames por estado: `BUSCANDO`, `INVALIDO`, `CONFIRMADO` | Cuánto rojo ve quien opera |
| `documento_frames_con_mrz` / `documento_progreso_maximo` | Frames con MRZ en cuadro y avance máximo de la votación | Qué tan cerca estuvo una sesión que no confirmó |
| `documento_confirmado_tipo` / `_fuente` / `_con_nombre` / `_vencido` | Tipo, origen (`OCR_FRENTE`, `MRZ`, `PDF417`), si trajo nombre y si está vencido (nunca número ni nombre) | Qué lector resuelve cada caso |

```sql
select ocurrido_en, datos->>'pantalla', datos->>'abierta_desde',
       datos->>'ms_hasta_confirmar', datos->>'frames', datos->>'descartados_calidad',
       datos->>'luminancia_p50', datos->>'frames_con_linterna', datos->>'nitidez_p50',
       datos->>'reflejo_p90', datos->'vehiculo_formatos', datos->>'vehiculo_frames_sin_lectura',
       datos->>'reinicios_votacion', datos->>'confianza_p10'
from public.telemetria_diagnostico
where tipo = 'ocr_sesion'
order by ocurrido_en desc;
```

## Latencia de Realtime

`latencia_ms_*` es la hora del teléfono al recibir el aviso menos
`changed_at`, la hora del servidor al escribir la fila. La hora del
teléfono se corrige con el desfase de su reloj (`desfase_reloj_ms`), que
el núcleo mide en cada autenticación con precisión de milisegundos
(`public.hora_servidor_ms`, ver `src/nube/reloj_preciso.rs`): varias
consultas y se usa la de viaje más corto, error de ±(la mitad de ese
viaje). Si el proyecto no tiene la función, el desfase sale del header
HTTP `Date` (±1 s) y la latencia sólo sirve para comparar;
`latencia_corregida` dice si hubo desfase para corregir.

La medición precisa corre en segundo plano después de autenticar y se
aplica con el token siguiente, así que al conectar el canal todavía puede
estar el desfase del header `Date`. Por eso el desfase se vuelve a leer,
como mucho cada 10 s, mientras llegan avisos (teléfono y escritorio). Antes
se leía una sola vez por conexión y en staging (2026-09-29) el teléfono dio
latencias de −950 ms durante toda la sesión.

Qué mirar para el lag entre equipos: la latencia del aviso (servidor →
equipo) es sólo el último tramo. Antes está lo que tarda el que guarda en
subir el cambio: `llamada_nucleo` con `enviarCambios` (teléfono)
o `enviar_cambios_nube` (escritorio), más la pausa de agrupación de 600 ms.

## Envío

- Cola en memoria con tope de 5000 filas (si se llena se descartan las más
  viejas: la telemetría no puede ser ella misma una fuga).
- Envío por lotes de 200 cada 60 s y al pasar la app a segundo plano, en un
  hilo de prioridad mínima. POST a PostgREST con la llave publicable y
  `Prefer: return=minimal`.
- Lo que no se pudo enviar (sin red) se guarda en
  `files/telemetria_pendiente.jsonl` y se reintenta en el próximo arranque.

## Tabla y permisos

Script: `supabase/scripts/telemetria_diagnostico_staging.sql` (idempotente,
sólo staging, **no** está en `supabase/migrations/` para que producción
nunca la tenga). RLS activado: la llave publicable sólo puede **insertar**;
leer, modificar o borrar requiere el SQL Editor. No hay límites ni limpieza
automática: son datos de desarrollo y se borran a mano con
`truncate public.telemetria_diagnostico;`.

El mismo script trae consultas de ejemplo (arranques, evolución de la
memoria por sesión, objetos retenidos, llamadas más lentas, sesiones OCR).

## Cómo leer una posible fuga de memoria

1. `retencion` con `EstadoCamaraOcr` o `MainActivity`: algo sigue
   referenciando la cámara o la Activity después de cerrarlas.
2. `muestra_sistema.pss_total_mb` que sube de sesión de escaneo en sesión de
   escaneo y no baja en reposo (misma `sesion`).
3. `descriptores_abiertos` o `hilos` que crecen sin volver: recursos sin
   cerrar (confirmar con `strictmode`).
4. `salida_anterior` con `motivo_nombre` `MEMORIA_BAJA` o `USO_EXCESIVO`.
