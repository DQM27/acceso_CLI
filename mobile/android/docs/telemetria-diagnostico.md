# Telemetría de diagnóstico

El teléfono de la portería (Samsung A25) tiene bloqueada la depuración USB,
así que `adb logcat` y el profiler de Android Studio no están disponibles.
Para medir cómo trabaja la app en el equipo real, el build `diagnostico`
manda métricas técnicas a la tabla `telemetria_diagnostico` del proyecto de
Supabase **staging**.

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

Cómo leerlo: muchos frames con código y **0 detectados** = falta
resolución (ML Kit ni lo encuentra); detectados pero **sin bytes** = ML
Kit no entrega el binario; con **motivos de rechazo** = el formato del
decodificador no calza con las tarjetas reales.

```sql
select ocurrido_en, datos->>'pantalla', datos->>'frames_con_codigo',
       datos->>'pdf417_codigos_detectados', datos->>'pdf417_sin_bytes',
       datos->'pdf417_motivos', datos->>'imagen_ancho_px_mediana_con_codigo',
       datos->>'pdf417_ancho_codigo_px_mediana'
from public.telemetria_diagnostico
where tipo = 'ocr_sesion' and (datos->>'lector_pdf417')::boolean
order by ocurrido_en desc;
```

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
