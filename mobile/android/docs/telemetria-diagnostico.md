# Telemetría de diagnóstico

El teléfono de la portería (Samsung A25) tiene bloqueada la depuración USB,
así que `adb logcat` y el profiler de Android Studio no están disponibles.
Para medir cómo trabaja la app en el equipo real, el build `diagnostico`
manda métricas técnicas a la tabla `telemetria_diagnostico` del proyecto de
Supabase **staging**.

El escritorio manda a la misma tabla, con los mismos nombres de evento y de
campo (`version_app` empieza con `desktop-`): ver
`desktop/docs/telemetria-diagnostico.md`.

## Qué build la tiene

| Build        | Paquete                    | Base      | Telemetría | Compilación                          |
|--------------|----------------------------|-----------|------------|--------------------------------------|
| `release`    | `com.dqm27.lattis`         | producción| no         | R8, llave real                       |
| `debug`      | `com.dqm27.lattis.debug`   | staging   | no         | sin optimizar, depurable             |
| `diagnostico`| `com.dqm27.lattis.diag`    | staging   | **sí**     | R8 como release, llave de debug, `profileable` |

`diagnostico` se compila igual que release porque un build de debug corre
sin optimizaciones y daría tiempos peores que los reales. Los tres paquetes
conviven en el mismo teléfono sin pisarse.

Compilar: `./gradlew assembleDiagnostico`, o desde GitHub con el workflow
manual **Build de prueba (mobile)** eligiendo la variante `diagnostico`.

Con `BuildConfig.TELEMETRIA = false` (release y debug) no se inicializa
nada: cada punto de medición es un chequeo de un booleano.

## Privacidad

Sólo números y nombres técnicos (nombres de pantalla, de operación del
núcleo, de clase). **Nunca** texto leído por el OCR, cédulas, nombres,
placas ni imágenes. El dispositivo se identifica con un UUID aleatorio por
instalación (no ANDROID_ID ni IMEI), y la sesión con otro por arranque. De
los motivos de salida del proceso se guarda sólo el resumen antes de `:`
(el resto puede ser el mensaje de una excepción).

## Eventos (`tipo`)

| Tipo              | Cuándo                                | Datos principales |
|-------------------|---------------------------------------|-------------------|
| `sesion_inicio`   | al arrancar el proceso                | fabricante, modelo, Android, ABI, núcleos, RAM total, clase de memoria |
| `arranque`        | primer frame dibujado                 | ms desde el inicio del proceso; en Android 15+ tipo y motivo del arranque (frío, tibio, caliente) |
| `salida_anterior` | al arrancar, una vez por salida       | por qué murió el proceso antes (ANR, memoria, crash nativo/Java, usuario), PSS/RSS |
| `pantalla`        | al entrar a cada pantalla             | ms hasta su primer frame |
| `muestra_sistema` | cada 30 s en primer plano             | heap Java y nativo, PSS por tipo (Java, nativo, gráficos, código), memoria libre del sistema, hilos, descriptores abiertos, CPU (% de un núcleo y del total), red rx/tx, batería (%, corriente, temperatura, carga, ahorro), estado y margen térmico, pantalla visible |
| `frames`          | cada 30 s, por pantalla               | frames, trabados (> 2× el presupuesto), congelados (> 700 ms), p50/p90/p99/máx |
| `llamada_nucleo`  | cada 30 s, por operación              | llamadas, errores, p50/p90/máx/total ms de cada llamada al núcleo Rust |
| `ocr_sesion`      | al cerrar una cámara de escaneo       | fps, frames, frames con PDF417, descartes por calidad, fallos, medianas de recorte y reconocimiento, ms hasta confirmar, y el diagnóstico del PDF417 (ver abajo) |
| `realtime`        | cada 30 s, si hubo actividad          | conexión (intentos, ms hasta suscribir, fin de cada conexión y minutos conectado, errores), avisos por tabla, ecos propios, KB recibidos, aplicados en la base local y su tiempo, y latencia (ver abajo) |
| `strictmode`      | cada 30 s, agregado                   | disco/red en el hilo principal, recursos sin cerrar, Activities filtradas, con el punto del código de la app |
| `memoria_baja`    | cuando el sistema pide liberar memoria| nivel de `onTrimMemory`, pantalla |
| `retencion`       | objeto vivo 10 s después de liberarlo | posible fuga: `MainActivity` destruida o `EstadoCamaraOcr` cerrado que siguen en memoria tras forzar el GC |
| `error_telemetria`| si falla la propia telemetría         | dónde y tipo de excepción |

## Diagnóstico del PDF417 (en `ocr_sesion`)

Dónde se pierde la lectura del código de la cédula anterior, sólo con
números y motivos (nunca bytes del código):

| Campo | Qué dice |
|---|---|
| `frames_con_codigo` | Frames en que se buscó el código |
| `pdf417_frames_con_codigo_detectado` / `pdf417_codigos_detectados` | Frames en que ML Kit decodificó algún PDF417, y cuántos |
| `imagen_ancho_px_mediana_con_codigo` | Ancho (px) de la imagen que analizó ML Kit al buscar el código: la resolución disponible |
| `pdf417_ancho_codigo_px_mediana` | Ancho (px) de los códigos detectados |
| `pdf417_sin_bytes` | Códigos que llegaron sin bytes crudos |
| `pdf417_bytes_min` / `pdf417_bytes_max` | Largo de los bytes recibidos |
| `pdf417_motivos` | Resultado del núcleo por código: `ACEPTADO`, `PREFIJO_CORTO`, `CEDULA_INVALIDA` (clave o formato distintos), `PRIMER_APELLIDO_INVALIDO` / `SEGUNDO_APELLIDO_INVALIDO` / `NOMBRE_INVALIDO` (posiciones corridas), `NOMBRE_O_APELLIDO_VACIO` |
| `pdf417_errores_lector` | Veces que el lector de códigos terminó con error |
| `pdf417_fotos` | Fotos de alta resolución por resultado: `LEIDA`, `CODIGO_INVALIDO`, `SIN_CODIGO`, `ERROR_CAPTURA`, `ERROR_LECTOR`, `ERROR_ENLACE` (no se pudo volver a poner el análisis) |
| `pdf417_fotos_con_codigo_detectado` | Fotos en que ML Kit encontró algún PDF417 |
| `pdf417_foto_ms_mediana` | Captura + decodificación + lector por foto (ms) |
| `pdf417_foto_ancho_px_mediana` | Ancho (px) de la foto, ya rotada |
| `ms_hasta_pdf417_en_foto` | Desde que se abrió la cámara hasta la primera foto con el código leído |

Cómo leerlo: muchos frames con código y **0 detectados** = falta
resolución (ML Kit ni lo encuentra); detectados pero **sin bytes** = ML
Kit no entrega el binario; con **motivos de rechazo** = el formato del
decodificador no calza con las tarjetas reales.

Las pruebas del 2026-09-29 dieron 0 detectados en 127 frames con ~940 px
de ancho: falta de resolución. Desde entonces, con el reverso en cuadro,
la pantalla saca además fotos de ~12 MP (`FotografoPdf417`, una cada
2 s como mucho); sus campos `pdf417_foto*` dicen si la resolución extra
alcanza. Primera versión: `pdf417_foto_ancho_px_mediana` = 1080 (la foto
enlazada junto con el análisis no subía de resolución); desde la segunda
se cambia el análisis por la foto en cada disparo. Los motivos y largos de `pdf417_*` suman frames y fotos.

```sql
select ocurrido_en, datos->>'pantalla', datos->>'frames_con_codigo',
       datos->>'pdf417_codigos_detectados', datos->>'pdf417_sin_bytes',
       datos->'pdf417_motivos', datos->>'imagen_ancho_px_mediana_con_codigo',
       datos->>'pdf417_ancho_codigo_px_mediana', datos->'pdf417_fotos',
       datos->>'pdf417_foto_ms_mediana', datos->>'ms_hasta_pdf417_en_foto'
from public.telemetria_diagnostico
where tipo = 'ocr_sesion' and (datos->>'lector_pdf417')::boolean
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
