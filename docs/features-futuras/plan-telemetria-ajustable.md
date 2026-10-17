# Telemetría ajustable (versión definitiva): pendientes y orden de trabajo

Estado al 2026-10-17. Este documento es la **guía de trabajo** para construir la
telemetría que se enciende y se ajusta sin sacar un release, en las dos apps
(móvil y escritorio). Completa a `docs/features-futuras/plan-herramienta-diagnostico.md`
(principios, decisiones y estado de Grafana) y a `docs/telemetria-contrato.md`
(lo que manda hoy la telemetría). No repite lo que ya dicen: lo enlaza, y agrega
lo que salió después, el orden concreto y los criterios para dar cada paso por
terminado.

## Para quién y para qué

Es una herramienta **del desarrollador**, no del cliente. Sirve para medir cómo
anda la app real cuando hay una sospecha ("se pega al dar salida", "la
sincronización tarda 2 a 3 s", "se acumulan descriptores") sin tener que armar
un build de prueba y pedir que lo instalen. Nada de esto va en el panel web ni
en la base de producción del cliente.

## Reglas que no se discuten

- **Staging, nunca producción.** Los datos van al proyecto de staging de Supabase
  (`pmrytjktlyiuikxuuxpr`). Producción (`xidaepyaljzkpbsxrqsm`) no se toca sin
  preguntar.
- **Apagada por defecto.** Sin regla vigente, o sin red, no se mide.
- **Toda regla vence** (por defecto en 24 horas).
- **Nada personal viaja.** Ni cédulas, nombres, placas, correos, rutas con el
  usuario del sistema, texto del OCR ni imágenes. Hoy es política; la meta es que
  lo haga el código (ver la fase 2).
- **Medir no puede costar más que lo que se mide.**
- **Credenciales fuera del repo y del chat.** Las llaves publicables no son
  secretas, pero la contraseña de `grafana_lector`, los tokens de Cloudflare y las
  llaves de servicio van a secretos o variables, nunca al código.

## Lo que ya existe

| Pieza | Dónde | Estado |
|---|---|---|
| Telemetría de rendimiento del móvil | `mobile/android/.../Telemetria.kt`, `TelemetriaCalculos.kt` | Funciona. En producción se enciende al compilar con la variable `TELEMETRIA_EN_PRODUCCION` (modo ligero). |
| Telemetría del escritorio | `desktop/src-tauri/src/telemetria.rs`, `desktop/src/telemetria.ts`, `telemetriaCalculos.ts` | **Solo en el build de diagnóstico** (feature de Cargo `telemetria`), que además redirige la app a staging y a carpetas propias. Los releases normales no la traen. |
| Tabla | `telemetria_diagnostico` en staging (`recibido_en`, `ocurrido_en`, `dispositivo`, `sesion`, `version_app`, `tipo`, `datos` jsonb) | Solo INSERT con la llave pública. Sin topes de tamaño ni de frecuencia. |
| Usuario de lectura | `grafana_lector` (`supabase/scripts/telemetria_lector_grafana_staging.sql`) | Funciona. La contraseña está en Grafana, no en el repo. |
| Grafana Cloud | Tablero "Lattis · Telemetría (staging)" y alerta de equipos nuevos | Funciona. Solo viven en Grafana, no en el repo. La alerta no se probó de punta a punta. |
| Actualizador de las apps | `lt.megabrisas.com` (R2) | Móvil y escritorio se actualizan solos. Permite sacar versiones nuevas con facilidad. |

Hallazgo de esta etapa: **el escritorio no manda ningún tiempo a staging**, así que
no hay datos para explicar la interfaz que se pega (ver "Lo que se persigue
primero").

## Lo que se persigue primero: medir por qué se pega el escritorio

Síntoma reportado en uso real: se pega un momento **siempre** (a) al registrar una
salida, (b) al registrar un ingreso, justo antes de mostrar el formulario del
gafete (1 a 2 s), y (c) más en una salida en lote (cada salida congela otra vez).

Qué se hizo ya: la versión 1.8.12 saca los 88 comandos de Tauri del hilo
principal (`#[tauri::command(async)]`, PR #173). Es la explicación más probable
(un comando síncrono corre en el hilo que dibuja la ventana) pero **no estaba
medida**.

Qué falta para cerrarlo con datos:

1. Confirmar con quien lo reportó si dejó de pegarse con la 1.8.12.
2. Medir **cada comando** del escritorio: cuánto tarda y cuánto esperó el candado
   del núcleo (`state.core()`). Es la primera pieza que necesita la telemetría
   ajustable en el escritorio (fase 3).
3. Si en el ingreso sigue habiendo 1 a 2 s: es la consulta a la nube previa al
   registro (`registrar_ingreso_verificado`). La ventana ya no se congela, pero la
   espera existe. Falta un **indicador de progreso** en ese paso.
4. Salida en lote: hoy el frontend hace una llamada por cada salida
   (`confirmarSalidasSeleccionadas` en `Activos.tsx`, `confirmarGafete` en
   `SalidaModal.tsx`). Un comando de lote en el núcleo (una transacción) la
   volvería una sola espera.
5. Salida y cambio "instantáneos": **actualización optimista** (la fila desaparece
   al tocar y el guardado ocurre por detrás; si falla, vuelve con un aviso).
6. Destello verde de AG Grid en Activos (ver los pendientes del plan anterior): al
   dar una salida hay dos recargas y las celdas cambiadas parpadean.

## Diseño (resumen de lo acordado)

Detalle completo en `plan-herramienta-diagnostico.md`. Lo esencial:

- **Tabla de reglas** en staging: qué equipos (versión, plataforma, modelo, versión
  del sistema, o el UUID de la instalación), qué eventos, con qué nivel de detalle
  y hasta cuándo. Se crean y apagan desde el editor SQL de Supabase. Sin pantalla
  propia al principio; Grafana puede mostrarlas, de solo lectura.
- **La app lee solo las reglas que le tocan**, con la llave pública y de solo
  lectura. Escribir reglas exige las credenciales del desarrollador.
- **Sin sondeo.** Se descartó consultar las reglas cada tantos minutos (gasta
  recursos solo para tener un interruptor). La app las consulta **al abrir y
  después de cada login exitoso**, con un hilo corto en segundo plano (tope de
  10 s) y como mucho una vez cada 5 minutos. El cambio de una regla se nota en
  el siguiente arranque o login. Si la consulta falla queda como estaba; una
  respuesta sin reglas apaga; el vencimiento se vigila en la app, sin consultar.
- **Niveles:** métricas ligeras siempre disponibles; **diagnóstico** con detalle
  solo cuando una regla lo enciende. Lo pesado (StrictMode, vigilante de fugas)
  sigue solo en el build de diagnóstico: no se puede encender en caliente sin
  causar el tirón que se quiere medir.
- **Eventos libres:** `datos` ya es `jsonb`. Se pueden mandar eventos con cualquier
  campo, siempre pasando por el limpiador.
- **Errores con detalle** por el mismo canal pero en **otra tabla** (se limpia a los
  90 días y con acceso más restringido).
- **Grafana** es la pantalla y los avisos.

### Cómo cambia el código del escritorio

Hoy la telemetría del escritorio existe solo con la feature `telemetria`, que
**además** redirige la app a staging. Para que una regla remota pueda encenderla en
un release normal hay que separar las dos cosas:

- El **código de medición** va dentro de todo release, apagado.
- La **redirección a staging** (base y claves de pruebas) sigue siendo exclusiva del
  build de diagnóstico: una app de producción nunca debe tocar datos de prueba.
- La app de producción solo manda **telemetría** a staging (nunca datos de
  operación), igual que ya hace el móvil en modo ligero.

Es una decisión de producto además de técnica: significa que un release del cliente
lleva código de telemetría y la puede encender una regla. Hay que **conversarlo con
el cliente** antes de activarla en sus equipos (ver "Decisiones abiertas").

## Cómo llegan los datos: dos caminos

Pendiente de decidir (se necesita volumen real para elegir; ver las cifras abajo).

**Camino A: la app manda directo a Supabase** (como hoy), pero con límites.
**Camino B: un Worker de Cloudflare** recibe la telemetría y la guarda.

Variantes del camino B:

- **B1. El Worker valida y limita, y guarda en Supabase.** Grafana no cambia nada
  (sigue leyendo la misma tabla con `grafana_lector`). Las apps mandan al Worker
  (`lt.megabrisas.com/telemetria`) en vez de a Supabase con la llave pública, y se
  puede **quitar el INSERT público** de la tabla cuando las versiones viejas
  desaparezcan. Cierra el hueco de que cualquiera con la llave pública llene la
  tabla.
- **B2. El Worker guarda en Analytics Engine de Cloudflare** y no en Supabase. Ahorra
  almacenamiento y escala mejor, pero Grafana necesitaría otra fuente de datos
  (API SQL de Cloudflare con un token de solo lectura; no se confirmó qué plugin la
  soporta) y el historial quedaría repartido. Los errores con detalle se quedarían
  en Supabase.

Datos que pesan en la decisión (verificar en las páginas de Cloudflare antes de
decidir, los topes cambian):

- Analytics Engine: gratis 100 000 puntos escritos por día y 10 000 consultas por
  día; guarda 3 meses. Plan de pago (Workers Paid, unos 5 USD al mes): 10 millones
  de escrituras y 1 millón de lecturas al mes incluidos.
- Lo que pesa en Supabase es el tamaño de la base y la transferencia, no el número
  de llamadas. La alternativa barata es seguir en Supabase y borrar lo viejo cada
  noche (la retención todavía no existe).
- **Hyperdrive no aplica**: es un intermediario de conexiones a un Postgres externo,
  no guarda datos ni ahorra cuota.

Recomendación vigente: **B1** (cambia solo la ruta de entrada; tableros y alertas
quedan igual; cierra el hueco de seguridad). **B2** solo si el volumen lo exige.
Antes de elegir, medir cuántos eventos por día manda de verdad una flota con la
telemetría encendida.

## Fases, en orden

Cada fase termina cuando se cumple el criterio de la derecha. Una fase por PR (o
dos pequeñas), CI en verde, y se prueba en un equipo real.

| # | Fase | Criterio de terminado |
|---|---|---|
| 0 | Medir el escritorio: tiempo por comando y espera del candado, con el build de diagnóstico, y verificar si se pega con la 1.8.12. | Hay filas de `llamada_nucleo` del escritorio en staging y el tablero muestra los tiempos por comando. |
| 1 | **Tabla de reglas** en staging y la lectura desde la app (móvil y escritorio), al abrir y tras el login, sin sondeo. Apagada por defecto. Separar en el escritorio el código de medición de la redirección a staging. **Estado:** tabla, función y escritorio hechos (falta probarlos en vivo); el móvil falta. | Sin regla no se manda nada; con una regla para un UUID, esa instalación manda y las demás no; al vencer, deja de mandar. Hay una prueba automática de la evaluación de reglas en cada plataforma, con los mismos casos. |
| 2 | **Limpiador** de datos personales en el teléfono/PC y en el receptor, con pruebas. | Un mensaje con una cédula, un correo o una ruta con el usuario sale limpio; las pruebas lo cubren y fallan si se rompe. |
| 3 | **Medición por comando en el escritorio** como eventos `llamada_nucleo` (mismo contrato que el móvil) y la espera del candado. | El tablero compara el tiempo por comando de móvil y escritorio. |
| 4 | **Topes y entrada segura**: camino A con límites, o Worker (B1). Limpiador de retención (30 días de métricas). | Una inserción fuera de tamaño o de ritmo se rechaza; lo viejo se borra solo. |
| 5 | **Errores con detalle** en su tabla aparte (solo lo inesperado: `Interno`, `Apertura`, crashes), con receptor y retención de 90 días. | Un error inesperado de prueba aparece con su detalle y un error "esperado" (sin red, sin sesión) no. **Estado:** receptor `receptor-errores` (Edge Function, sólo staging), tabla `errores_app`, vista `errores_agrupados`, retención de 90 días (`limpiar_errores_app()`; programarla pide `pg_cron`) y tablero `Lattis · Errores` (`docs/grafana/`) hechos y probados con un evento de prueba. Escritorio y móvil apuntan al receptor con el secreto `SENTRY_DSN`; falta probarlo con un error real de cada uno, y la alerta de grupo nuevo. |
| 6 | Reescribir `docs/telemetria-contrato.md` (reglas de limpieza y niveles) y los encabezados de los scripts de Supabase. | El contrato describe lo que el código hace. |
| 7 | Traducción de trazas minificadas del móvil (guardar el `mapping.txt` de cada release y usar `retrace`). Es lo más caro; va al final. | Una traza de un release real sale legible. **Estado:** resuelto sin `retrace`: R8 ya no renombra (`-dontobfuscate`) y conserva archivo y línea, y el SDK marca como código propio `com.brisas.controlacceso` y `uniffi.control_acceso_mobile`. Si el repositorio pasa a privado y se vuelve a ofuscar, hay que retomar esta fase. Los fallos nativos del núcleo de Rust siguen sin símbolos. |

La fase 0 se puede hacer ya, con el build de diagnóstico, sin esperar a las demás.

## Pendientes que salieron en esta etapa y siguen abiertos

De la sesión del 2026-10-17 (además de los del plan anterior):

- **Notas de versión** en el actualizador móvil: campo opcional `notas` en el
  manifiesto (agregar un campo no sube `esquema`), escritas en un archivo del repo
  (por ejemplo `mobile/android/notas-version.txt`) que el workflow lee al publicar,
  mostradas en la tarjeta flotante con un "Ver novedades". El núcleo valida largo y
  contenido, como el resto del manifiesto.
- **Pendientes por enviar y hora de la última sincronización** en el móvil (para
  trabajar con mala señal): en el menú de la cuenta o al tocar el punto de conexión.
- **Notificación del sistema** de actualización obligatoria (opcional).
- **Instalar la versión con actualizador** a mano en los demás teléfonos y PCs.
- **Pasar el repositorio a privado** (GitHub Pro): verificar cómo se despliega el
  panel web y los minutos de Actions. Después, opcionalmente, borrar los Releases
  viejos de GitHub dejando el último de cada app. Página de descargas en
  `lt.megabrisas.com` (ver `docs/pendientes.md`).
- **Seguridad**: confirmar que el registro abierto de usuarios esté apagado en
  Supabase Auth, activar la protección de contraseñas filtradas, poner topes a las
  inserciones de `telemetria_diagnostico` (fase 4) y revisar tokens viejos de
  Cloudflare con permisos sobre todos los buckets.
- **Respaldo nocturno de Supabase a R2** (`pg_dump` desde una acción programada).
- **Alertas de Dependabot** que GitHub mostró al subir una rama: no se revisaron.
- **Alertas y tableros de Grafana:** ya están como JSON en `docs/grafana/`. Ajustar las tablas para teléfono y el color del gráfico de
  errores.
- **Posible fuga de hilos y descriptores en el móvil** (hipótesis: la conexión en
  vivo se reabre al volver a primer plano y algo de la anterior no se cierra,
  `NubeRealtime.kt`).
- **Sincronización sin sesión**: no debe correr ni reportar mientras no haya
  sesión (causa del error número uno de Sentry en escritorio).

## Decisiones abiertas

- **Camino de entrada:** A (directo con límites), B1 (Worker que guarda en
  Supabase) o B2 (Worker con Analytics Engine). Se decide con el volumen medido.
- **Producción mide por su cuenta cuando una regla lo enciende**, en el escritorio y
  el móvil: hay que conversarlo con el cliente.
- **Errores con detalle:** en el proyecto de staging o en uno propio.
- **Si Grafana debe leer los eventos de seguridad de producción** (exige darle
  lectura sobre producción).

## Cómo se trabaja (reglas del proyecto)

Están en `AGENTS.md`; las que más importan aquí:

- Comunicarse siempre en español, y **nunca** usar el término prohibido para el
  puesto de control (decir "puesto de control", "portería" o "punto de acceso").
- Commits a nombre de `Daniel Quintana <daniel.bleach1@gmail.com>`, como autor y
  como committer, **sin** `Co-Authored-By`, `Claude-Session` ni ninguna otra
  atribución, ni en commits ni en la descripción de los PRs. Si la herramienta de
  publicación agrega un pie "Generated by Claude Code" al crear un PR, quitarlo
  editando la descripción y comprobarlo.
- Un cambio, un commit bien documentado, y subirlo. No abrir PRs sin pedirlo.
- Verificar antes de afirmar: si no se pudo compilar o medir, decirlo.
