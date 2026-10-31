# Plan: todo el tráfico del móvil por el núcleo

Estado: **propuesta** (2026-10-31). Complementa
`plan-observabilidad-en-produccion.md`.

## Por qué

Lo operativo del móvil (ingresos, `verificar_entrada`, sincronización con
`estado_del_sitio` y `catalogo_del_sitio`) ya pasa por el núcleo de Rust:
HTTP/2 (comprobado contra producción), conexión reutilizada y respuestas
comprimidas. Tres piezas de Kotlin quedaron fuera y usan `HttpURLConnection`
(HTTP/1.1, conexión y TLS nuevos en cada llamada):

| Pieza | Archivo | Frecuencia |
|---|---|---|
| Telemetría | `Telemetria.kt` | cada 60 s |
| Avisos push: registrar el token y mandar acciones de un aviso | `NotificacionesPush.kt` (`llamarRpc`) | al abrir, al rotar el token, al actuar sobre un aviso |
| Actualizaciones: manifiesto y APK | `ServicioActualizaciones.kt` | al volver a primer plano (con pausa) y al instalar |

Criterio del dueño: si se gana batería, datos o tiempo, aunque sea poco, se
hace.

## La radio del teléfono

El módem celular gasta poco dormido y mucho transmitiendo, y después de cada
envío queda despierto varios segundos ("cola") antes de volver a dormir. Una
conexión nueva además paga DNS + TCP + TLS (varios viajes de ida y vuelta,
cientos de ms en datos móviles) antes del primer byte útil. Cada pieza de
arriba despierta la radio por su cuenta y abre su propia conexión.

## Qué cambia

### 1. Telemetría: al núcleo y "a caballo" de otro tráfico

Ver el plan de observabilidad (lote columnar + brotli 11 + HTTP/2 + token).
Además, en el móvil **no se manda a hora fija**:

- Sale **justo después** de que la radio ya se despertó por otra cosa
  (sincronización, verificación de un ingreso, aviso push), por la misma
  conexión HTTP/2 todavía abierta.
- Sólo si pasa mucho sin otra actividad (propuesta: 15 min) sale sola.
- Al cerrar la app o pasar a segundo plano, lo pendiente se guarda en disco
  como hoy (no se pierde) y sale en el próximo envío.
- El escritorio sigue a 60 s: tiene red fija y conexión siempre caliente.

### 2. Avisos push: RPC desde el núcleo

`registrar_token_push` y las acciones de avisos pasan a métodos del núcleo
(`Nucleo.registrarTokenPush`, `Nucleo.enviarAccionesAviso`): mismo cliente
HTTP/2, misma conexión y mismo token del equipo que ya usa la operación, sin
que Kotlin arme peticiones ni lea el token. Firebase (recibir el aviso,
mostrar la notificación) se queda en Kotlin: es del sistema Android.

### 3. Actualizaciones: red en el núcleo, instalación en Kotlin

Probado contra `lt.megabrisas.com` (Cloudflare) el 2026-10-31: negocia
**HTTP/2**, manda **`ETag`** (`If-None-Match` → **304 con 0 bytes** de
cuerpo; el manifiesto entero son 289 B) y **`Accept-Ranges: bytes`**. El APK
1.8.13 pesa **36,3 MB**.

- **Manifiesto condicional:** se guarda el `ETag`; si no cambió, la nube
  responde 304 sin cuerpo y no hay nada que leer ni validar.
- **Descarga que se retoma:** hoy un corte al 90 % borra el `.part` y empieza
  de cero los 36 MB. Con `Range` se retoma desde lo descargado, y la huella
  SHA-256 se calcula sobre el archivo completo al final (si no coincide, se
  borra y se reintenta entero: nunca se instala un APK dañado).
- **Misma conexión** para manifiesto y APK (mismo servidor).
- **Sólo con Wi-Fi o cargando** para la descarga automática del APK
  (propuesta; la descarga pedida a mano por el usuario va siempre).
- La validación del manifiesto ya está en el núcleo
  (`evaluar_manifiesto_actualizacion`); la descarga escribe en la ruta que da
  Kotlin y avisa el avance por un callback de UniFFI. `PackageInstaller` y la
  verificación de la firma del APK siguen en Kotlin.

## Resultado esperado

- Ninguna petición de la app fuera del núcleo: un solo cliente HTTP/2, una
  sola política de reintentos, compresión y token.
- La radio se despierta por trabajo útil, no por telemetría.
- Telemetría: de ~75 KB por minuto a ~3 KB y menos de un envío por minuto.
- Manifiesto: 0 B de cuerpo cuando no hay versión nueva.
- APK: un corte no tira lo ya descargado.

## Cómo se mide que mejoró

La propia telemetría: bytes enviados por día por equipo, envíos por hora,
`muestra_sistema` (red, batería) antes y después, y el tiempo de la primera
llamada después de un rato quieto.

## Fases

Van con la fase 2-3 del plan de observabilidad (el envío al núcleo), en este
orden: telemetría, push, actualizaciones. Cada una con sus pruebas en el
núcleo y sus bindings regenerados.
