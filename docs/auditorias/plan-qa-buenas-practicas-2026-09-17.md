# Plan QA — buenas prácticas de software (2026-09-17)

> Estado: en curso. Rama `qa`. Este documento es la continuación con código
> de la auditoría conversacional del mismo día — cada punto se verificó
> contra el código real del repo (no contra comentarios ni documentación
> vieja), y cada uno que se cerró acá tiene un test o una verificación
> concreta que lo prueba, no solo una promesa.
>
> Formato pensado para mostrarse a un cliente: qué se buscó, por qué importa
> para *este* proyecto en particular (no "porque un programa profesional
> debería tenerlo"), qué se hizo, y cómo se verifica.

## Cómo se usa este documento

Cada punto tiene un estado:

- ✅ **Cerrado** — implementado en esta rama, con test o verificación
  reproducible citada.
- 🚧 **Plan definido** — el problema es real, hay un plan con fases y
  criterio de aceptación, pero implementarlo requiere una decisión de
  negocio (costo, proveedor externo) o un cambio de arquitectura que no se
  toma a la ligera en un solo pase.
- ➖ **Fuera de alcance, justificado** — se evaluó y NO aplica para este
  proyecto. Se explica por qué, siguiendo el mismo criterio que ya usa el
  repo: un componente que no aporta aislamiento, seguridad, testabilidad o
  flexibilidad real es complejidad disfrazada de buena práctica.

Solo se documentan acá los puntos donde había algo real que corregir o
decidir. Todo lo que ya estaba bien (arquitectura, migraciones, cifrado,
auditoría, tests, offline-first, etc. — ver el detalle de los 70 puntos en
la conversación del mismo día) no se repite acá.

---

## ✅ Cerrado en esta rama

### 1. Registro de versión instalada

**Qué se buscaba:** que alguien reportando un problema pueda decir qué
versión tiene sin ir a buscarla, y que un desarrollador pueda confirmar qué
build está corriendo un sitio.

**Por qué importa acá:** hay actualizaciones automáticas
(`tauri_plugin_updater`, ver `desktop/src-tauri/tauri.conf.json`) corriendo
en sitios sin supervisión técnica constante — sin la versión a la vista, un
reporte de soporte ("no me deja hacer X") no dice si el sitio ya tiene el
fix o todavía no le llegó la actualización.

**Qué se hizo:** `desktop/src/componentes/VersionFooter.tsx` — muestra la
versión real leída de Tauri (`getVersion()`, refleja `tauri.conf.json`, hoy
`1.5.3`) en la esquina inferior del sidebar. A propósito NO se usa
`package.json` como fuente — ese campo está congelado en `"0.0.0"` y no es
lo que termina empaquetado.

**Cómo se verifica:**
```sh
cd desktop && npm test -- VersionFooter
```
`desktop/src/componentes/VersionFooter.test.tsx` — 3 tests: muestra la
versión, la esconde (pero deja el tooltip) cuando el sidebar está
colapsado, y no renderiza nada mientras la versión todavía no llegó.

---

### 2. Formateo automático realmente aplicado (no solo disponible)

**Qué se buscaba:** que `cargo fmt` no sea una herramienta que existe sino
una que se exige.

**Por qué importa acá — hallazgo real, no hipotético:** el job `test`
(crate raíz) corre `cargo fmt --check` desde siempre, pero **`test-android`
(`mobile/rust-core`) y `test-gui` (`desktop/src-tauri`) nunca lo corrieron**
— cada uno es un crate Rust separado con su propio `cargo fmt`. Resultado
concreto: **30 archivos** entre esos dos crates llegaron a `main` sin
formatear (PR #14, 2026-09-17) y nadie lo notó hasta correrlo a mano.

**Qué se hizo:** `.github/workflows/ci.yml` — se agregó `cargo fmt --check`
a `test-android` (working-directory `mobile/rust-core`) y a `test-gui`
(working-directory `desktop/src-tauri`), en el mismo punto del job que ya
usa `test` para el crate raíz.

**Cómo se verifica:** el próximo push a esta rama corre los 3 jobs con el
gate nuevo — si alguno de los dos crates tiene algo sin formatear, ahora
sí falla. Verificado localmente antes de commitear:
```sh
cd mobile/rust-core && cargo fmt --check   # limpio
cd desktop/src-tauri && cargo fmt --check  # limpio
```

---

### 3. Logs en producción y un fallo silencioso real, corregido

**Qué se buscaba:** que un fallo en un sitio real deje algún rastro, en vez
de depender 100% de que el usuario lo note y lo describa bien por teléfono.

**Por qué importa acá:** es el hueco más serio de los 70 puntos evaluados.
Sin esto, "algo falló en el sitio 4" es un callejón sin salida.

**Qué se hizo (primer paso, acotado a propósito):**

1. `desktop/src-tauri/src/lib.rs` (`configurar_plugins_condicionales`): el
   plugin `tauri_plugin_log` **antes solo corría en `debug_assertions`** —
   en la app empaquetada real, nunca se activaba. Ahora corre siempre;
   nivel `Info` en debug (sin cambios), `Warn` en release (para no llenar
   el archivo con el ruido de cada operación exitosa). Los *targets* por
   defecto del plugin ya escriben a un archivo rotado en el directorio de
   logs de la app (tope 40 KB, conserva 1 backup) además de stdout — no
   hizo falta configurar nada de eso a mano, ya viene así de fábrica.
2. Un fallo silencioso real, encontrado al revisar el código que ahora sí
   quedaría logueado: `iniciar_sincronizacion_automatica` (mismo archivo)
   descartaba en silencio TANTO un fallo de sincronización real
   (`Ok(Err(error))`, ej. token vencido y no renovable, sin conexión) COMO
   un panic de la tarea en background (`Err(_)`) — ninguno de los dos casos
   llegaba a ningún lado, ni log ni aviso. Ahora ambos se loguean
   (`log::warn!`/`log::error!` respectivamente).

**Honestidad sobre el alcance** (para no hablar de más de lo que se hizo):
- Esto cubre fallos que ocurren **después** de que la app terminó de
  arrancar (`.setup()` de Tauri ya corrió). Un fallo fatal **antes** de eso
  (`mostrar_error_fatal_y_salir` — base dañada, candado de instancia,
  etc.) sigue sin dejar rastro en archivo, porque en ese punto el plugin
  todavía no existe. Queda como punto 4 del plan de abajo.
- Antes de este cambio había **cero** llamadas a `log::warn!`/`log::error!`
  en todo `desktop/src-tauri/src/` — activar el plugin sin instrumentar
  nada no hubiera servido de nada. El punto de arriba es el primer caso
  real instrumentado, no el único que hace falta (ver plan, punto 4).
- No se pudo compilar `desktop/src-tauri` en este sandbox (Linux, la app es
  Windows-only por diseño — ver PR #14). Se verificó `cargo fmt --check`
  localmente y se validó el tipo de cada rama del `match` a mano (`String`
  y `tokio::task::JoinError` implementan `Display`); la compilación real la
  confirma el job `test-gui` (Windows) en el próximo push.

---

### 4. Backups — código muerto encontrado y limpiado, doc corregido

**Qué se buscaba:** confirmar el estado real del "respaldo puntual
pre-migración" que `docs/pendientes.md` daba por vivo desde 2026-09-13, sin
haberlo re-verificado en ese momento (la propia nota lo admitía: "no
verificado de nuevo en este pase").

**Hallazgo:** es falso. `grep -rn "TipoRespaldo\|respaldar_antes_de_migrar"
src/` no encuentra ninguna función ni struct con ese nombre en todo el
crate. Lo único que quedaba eran:
- Una variante de error nunca construida en ningún lado:
  `SchemaError::RespaldoPreMigracionFallido` (`src/database/schema.rs`) —
  invisible para Clippy porque es parte de un `enum` **público**
  (`dead_code` sólo avisa sobre ítems privados).
- 3 doc-comments que la mencionaban como si existiera.

Es exactamente la clase de problema que motivó esta auditoría: documentación
y comentarios que sobreviven al código que describían.

**Qué se hizo:**
- Se eliminó la variante muerta y los 3 comentarios que la referenciaban
  (`src/database/schema.rs`).
- Se corrigió la nota en `docs/pendientes.md` ("Respaldo, base local y
  dominio") con el hallazgo verificado.

**Cómo se verifica:**
```sh
cargo clippy --all-targets   # limpio, sin warnings de código muerto nuevos
cargo test --test migraciones  # 23 passed — nada dependía de esa variante
grep -rn "TipoRespaldo\|RespaldoPreMigracionFallido" src/   # sin resultados
```

**Decisión que esto confirma (no es un pendiente, es el estado real):** no
hay ningún archivo de respaldo local, ni general ni pre-migración. Lo único
que protege contra una migración de esquema a medio aplicar es la
atomicidad transaccional (`fallo_tardio_revierte_todas_las_migraciones_pendientes`,
`tests/migraciones.rs`) — si falla, revierte sola, no deja el esquema a
medias. Eso es real y está probado; un archivo de respaldo recuperable
ante corrupción del `.db` en sí (disco dañado, etc.) es otra cosa y **no
existe** — ver punto 7 del plan.

---

### 12. Chequeo de versión mínima entre dispositivos (escritorio)

**Qué se buscaba:** que un dispositivo que quedó muy atrás de versión no
pueda seguir sincronizando contra un backend que ya evolucionó más allá de
lo que esa versión entiende, en vez de fallar con un error críptico en
producción.

**Hallazgo de diseño, antes de implementar:** `app_version` sólo viajaba en
la activación inicial del dispositivo (`MetadatosDispositivo`, ver el
propio doc-comment en `src/nube/cliente.rs` -- es deliberado, no un
descuido). Un chequeo que sólo mirara eso se evaluaría UNA vez en la vida
del dispositivo, nunca más -- inútil contra un dispositivo que se atrasa
con el tiempo. Se resolvió sin tocar ese diseño (no se manda la metadata
completa más seguido): se agregó un campo separado, liviano, que sí viaja
en cada renovación.

**Qué se hizo:**
1. `supabase/functions/device-auth/index.ts` -- secret opcional
   `VERSION_MINIMA_ACEPTADA` (no una tabla nueva -- cambia rara vez, no
   amerita una consulta a la base en cada login). Si no está seteado, no
   cambia nada del comportamiento actual. Compara con `versionPorDebajoDe`
   (major.minor.patch, mismo esquema que ya usa el proyecto) y responde
   `426 Upgrade Required` si corresponde. Fail-open a propósito: sin
   versión mínima configurada, o sin que el cliente la mande, nunca
   rechaza -- evita dejar afuera de golpe a dispositivos que todavía no
   mandan la versión en cada renovación el día que esto se active.
2. `src/nube/cliente.rs` -- dos variantes nuevas de `NubeError`
   (`DispositivoSuspendido`, `VersionDesactualizada`) mapeadas desde los
   status HTTP 403/426 de `device-auth`. De paso, `device_suspended` (403)
   dejó de caer en el genérico `NubeError::Red` -- ya existía en el
   backend pero el cliente nunca lo distinguía.
3. `src/mensajes.rs` -- mensaje específico para cada una (uno dice
   "contactá a un administrador", el otro "actualizá la app" -- no deben
   compartir texto, apuntan a acciones distintas).
4. `src/application/mod.rs`/`nube.rs` -- `AppCore::establecer_version_app`
   (nuevo, opcional) hace que **cada** renovación de token mande
   `app_version`, no sólo la activación -- sin llamarlo, cero cambio de
   comportamiento.
5. `desktop/src-tauri/src/lib.rs` (`preparar_nucleo`) -- llama a ese
   setter con `env!("CARGO_PKG_VERSION")` de `desktop/src-tauri/Cargo.toml`
   (`1.5.3`, igual al de `tauri.conf.json` hoy -- son campos separados que
   no se sincronizan solos, tenerlo presente si alguna vez divergen).

**Cómo se verifica:**
```sh
cargo test --lib --features "nube,cifrado-secreto-dispositivo"
# 365 passed, incluye:
#   nube::cliente::tests::dispositivo_suspendido_se_reporta_como_tal
#   nube::cliente::tests::version_desactualizada_se_reporta_como_tal
#   mensajes::tests::dispositivo_suspendido_y_version_desactualizada_no_comparten_mensaje
```
La función `versionPorDebajoDe` (Deno/TypeScript, sin dependencias de Deno)
se verificó con 10 casos manuales por `node` -- no se instaló Deno ni un
framework de test nuevo para una sola función (el propio criterio de este
documento: no abrir un sistema nuevo sin necesidad real).

**Lo que falta -- mobile:** ver el punto 9 más abajo (🚧), que sigue con el
detalle completo de por qué no se tocó Kotlin todavía.

---

## 🚧 Plan definido — requieren decisión o más alcance

### 5. Observabilidad — completar la instrumentación

**Estado real tras el punto 3:** el mecanismo existe y ya prueba un caso
real. Falta cubrirlo donde más importa.

**Plan, en orden de impacto:**

| Fase | Qué | Criterio de aceptación |
|---|---|---|
| 5.1 ✅ | Loguear cada `Err(...)` que un comando Tauri le devuelve al frontend, y su equivalente mobile (`NucleoError::Interno`) | Ver detalle abajo |
| 5.2 ✅ | Loguear los reintentos agotados/fallidos de `cola_salida` (la cola de sync offline) | Ver detalle abajo |
| 5.3 ✅ | Logging para el fallo fatal de arranque (`mostrar_error_fatal_y_salir`) | Ver detalle abajo |
| 5.4 ✅ | Error-tracking externo (Sentry) — cerrado 2026-09-17, ver detalle abajo | -- |

### 5.1/5.2/5.3 — instrumentación de logs, escritorio y mobile (2026-09-17)

**Criterio general (evita ruido):** sólo se loguean fallos técnicos/inesperados
(base de datos, red, E/S, parseo) -- un rechazo de negocio normal (cédula
duplicada, sesión no autorizada, gafete ocupado) es el flujo esperado, no un
fallo, y loguear cada uno sería puro ruido que tapa lo que sí importa.

**5.1, escritorio -- `src/mensajes.rs` (compartido con mobile):** este
archivo ya era el cuello de botella por donde pasa todo error de servicio
antes de convertirse en un mensaje para una interfaz -- y ya distinguía
`Database(_)`/técnico de negocio en cada `match` (para no exponer detalles
de `SQLite` en pantalla). Se agregó `log::error!`/`log::warn!` exactamente
en esas ramas técnicas, capturando el error real (antes se descartaba con
`_`). Se agregó `log = "0.4"` al crate raíz.

**5.1, escritorio -- comandos que no pasan por `mensajes.rs`:** 34 sitios
en `desktop/src-tauri/src/comandos/*.rs` hacían
`.map_err(|error| error.to_string())` a mano (`rusqlite::Error` suelto,
errores de parseo de fecha, E/S) -- por definición, sin variante de negocio
detrás, categóricamente técnicos. Se agregó `comandos::mensaje_generico`
(loguea y convierte) y se reemplazaron los 34 sitios por
`.map_err(super::mensaje_generico)`. Convención documentada en
`comandos/mod.rs`.

**5.1, mobile -- `mobile/rust-core/src/lib.rs`:** mismo criterio, mismo
problema: 44 sitios construían `NucleoError::Interno { mensaje: error.to_string() }`
a mano. Se agregó la función `interno()` (loguea y devuelve el `String`) y
se reemplazaron los 44 con `mensaje: interno(error)` (dos mensajes de
negocio hardcodeados, sin error real detrás, quedaron sin tocar a
propósito). Para que esos `log::` no caigan en el vacío, se agregó
`android_logger` (sólo para `cfg(target_os = "android")`) inicializado en
`Nucleo::abrir()` -- el primer método que llama Kotlin, tag
`control_acceso_mobile` (`adb logcat -s control_acceso_mobile`), mismo
criterio de nivel que escritorio (`Info` en debug, `Warn` en release).

**5.2 -- `src/nube/sincronizacion.rs`, drenado de `cola_salida`:** el
`Err(error)` del envío de cada fila sólo quedaba en la columna
`ultimo_error` de la propia fila -- nadie se enteraba salvo que fuera a
mirar la cola a mano. Ahora, además: `log::warn!` en cada reintento
transitorio (todavía le quedan intentos) y `log::error!` cuando agota los
[`INTENTOS_ANTES_DE_FALLO_PERMANENTE`] y queda `fallido` de forma
permanente -- ese es el caso realmente crítico (esa fila no se reintenta
más sola).

**5.3 -- `mostrar_error_fatal_y_salir` (escritorio):** corre antes de que
exista una `AppHandle` (por lo tanto, antes de que `tauri_plugin_log` pueda
inicializarse), así que no podía usar el mecanismo normal. Se agregó
`registrar_fallo_fatal_en_archivo` (sólo Windows): escribe directo a
`%LOCALAPPDATA%\com.dqm27.controlaccesobrisas.desktop\logs\fallo-fatal-arranque.log`,
mismo directorio donde el plugin deja los suyos una vez que arranca.
Cualquier fallo dentro de esa función (no se pudo leer `LOCALAPPDATA`, no
se pudo crear el archivo) se descarta en silencio a propósito -- no puede
convertirse ella misma en un segundo punto de fallo en el peor momento del
arranque. Además, como `inicializar_sentry()` ya corrió antes de
`preparar_nucleo()` (ver 5.4), este mismo punto ahora también manda
`sentry::capture_message(..., Level::Fatal)` -- un fallo fatal de arranque
en un sitio real llega como notificación, no sólo como archivo local.

**Cómo se verificó (en esta PC):**
```sh
# Crate raíz + comandos de escritorio
cargo clippy --no-default-features --features "nube,sqlite-plano" --all-targets   # limpio
cargo fmt --check                                                                  # limpio
cargo test-plano --lib --features "nube,cifrado-secreto-dispositivo"               # 369 passed
cd desktop/src-tauri && cargo clippy --all-targets && cargo fmt --check            # limpio

# mobile/rust-core
cd mobile/rust-core
cargo clippy --no-default-features --features sqlite-plano --all-targets          # limpio
cargo fmt --check                                                                  # limpio
cargo ndk -t aarch64-linux-android build --release --no-default-features --features sqlite-plano
# Finished -- confirma que android_logger cross-compila de verdad para Android,
# no sólo para el host.

cd mobile/android && ./gradlew :app:assembleDebug   # BUILD SUCCESSFUL
```
`cargo test-mobile-plano` corrió 22 tests, 21 passed -- el único que falló
(`registrar_salida_y_retorno_de_ruta_redondea_el_viaje`) es un bug de
timing preexistente y no relacionado: el fixture arma `fecha_documento` con
`chrono::Utc::now()` (calendario UTC) mientras la regla de negocio compara
contra "hoy en Costa Rica" (UTC-6) -- falla sólo durante la ventana
~00:00-06:00 UTC en que ambos calendarios difieren. No se tocó como parte
de este cambio.

### 5.4 — Sentry, cerrado (2026-09-17)

**Qué se hizo:** cuenta creada por el usuario, dos proyectos vía el MCP de
Sentry (`create_project`, team `dqm27`):

- **Escritorio** (`control-acceso-desktop`, plataforma Rust) —
  `desktop/src-tauri/Cargo.toml` agrega `sentry = "0.36"`.
  `desktop/src-tauri/src/lib.rs`: función nueva `inicializar_sentry()`
  (separada de `run()` por el mismo motivo que `preparar_nucleo` --
  tope de líneas de Clippy), guarda `sentry::ClientInitGuard` viva durante
  toda la vida del proceso (hace flush al salir). Captura panics
  automáticamente (feature `panic`, default). Los dos fallos silenciosos
  reales de `iniciar_sincronizacion_automatica` (punto 3 de este documento)
  ahora también van a Sentry con `sentry::capture_message`, no solo al log
  local. `traces_sample_rate = 0.0` a propósito: solo error-tracking, nada
  de performance tracing (cuida cuota del plan gratis).
- **Mobile** (`control-acceso-mobile`, plataforma Android) --
  `mobile/android/app/build.gradle.kts` agrega
  `io.sentry:sentry-android:8.9.0`; init 100% declarativo vía `meta-data`
  en `AndroidManifest.xml` (sin tocar ninguna Activity/Application) --
  captura crashes no manejados desde el primer arranque tras instalar.
  `sentryEnvironment` viaja por `manifestPlaceholders`
  (`debug`→"development", `release`→"production"), mismo criterio que
  `cfg!(debug_assertions)` del lado de escritorio.

El DSN de cada proyecto queda embebido en el cliente a propósito -- no es
secreto (mismo criterio que la publishable key de Supabase en `web/`), es
lo que el SDK necesita para saber a dónde mandar los eventos.

**Cómo se verificó (en esta PC, no en un sandbox sin Windows/Android SDK
como se pensó en un primer momento):**
```sh
cd desktop/src-tauri && cargo fmt --check && cargo clippy --all-targets && cargo audit
# limpio; cargo audit sigue con los mismos 7 avisos "unmaintained"
# pre-existentes documentados en la auditoría de endurecimiento, ninguno
# nuevo por sentry.
cd mobile/android && ./gradlew :app:assembleDebug
# BUILD SUCCESSFUL -- confirma que sentry-android 8.9.0 resuelve, compila
# y empaqueta libsentry-android.so/libsentry.so en el APK.
cargo test-plano --lib --features "nube,cifrado-secreto-dispositivo"
# 369 passed -- nada del crate raíz se rompió.
```

**Ojo con la cuenta de Sentry (no técnico, para no perderlo de vista):** al
crearla quedó en un trial del plan Business (13 días, sin tarjeta
cargada) en vez de caer directo en el plan gratis. Sin tarjeta no hay
riesgo de cobro automático -- al vencer el trial baja sola al plan
gratuito real. Ojo con no tocar el botón "Confirmar" de la pantalla de
selección de plan mientras tanto, porque activaría el plan pago.

### 6. Diagnóstico exportable para soporte

**Qué falta:** cuando alguien reporta un problema, hoy no hay forma rápida
de pedirle "mandame el archivo de diagnóstico" — hay que guiarlo a mano
por carpetas de Windows.

**Plan:** un comando Tauri nuevo (`comandos::soporte::exportar_diagnostico`)
que arme un `.zip` con: el log rotado (punto 3), la versión (`getVersion()`,
punto 1), y un resumen de `cola_salida` (cuántos pendientes/fallidos, sin
datos de personas). Un botón en la UI que lo genere y abra la carpeta.

**Criterio de aceptación:** un test de integración que arme el zip contra
una DB de prueba y confirme que contiene los 3 archivos esperados, sin
cédulas ni nombres reales en el JSON de `cola_salida`.

### 7. ✅ Runbook de recuperación — base local corrupta (cerrado 2026-09-17)

**Decisión del usuario:** aceptable perder `auditoria_cambios`/
`auditoria_contratistas`/`gafetes_incidentes` y lo que quedara sin subir en
`cola_salida` si un sitio tiene que reconstruirse desde cero -- queda
anotado como "a valorar más adelante", no un pendiente activo.

**Qué se hizo:** `docs/recuperacion-sitio-local.md` -- runbook completo,
espejo de `docs/recuperacion-supabase.md`. El hallazgo importante que
justificaba el punto 8 de este plan (definir el "método de seguridad" de
la restauración, no sólo el mecánico): en escritorio, `db_key.dat` Y
`dispositivo-nube.secret` viven en la MISMA carpeta que `control_acceso.db`
(`%LOCALAPPDATA%\ControlAcceso\`) -- un evento que destruye la base
normalmente destruye también el secreto de dispositivo. La recuperación
real no es "reinstalar y listo": ese dispositivo queda sin forma de
autenticarse como sí mismo, y hay que tratarlo como un dispositivo nuevo
vía `admin-provision-device` (mismo `sitio_nombre`, resuelve por `upsert`
al `sitio_id` que ya tenía toda la historia). Esto es una propiedad de
seguridad correcta, no un defecto: no existe ningún atajo de recuperación
que evite pasar por la misma autorización de administrador que dar de alta
un dispositivo nuevo.

El runbook también documenta la secuencia real de qué vuelve cuándo:
catálogo completo inmediato en la activación
(`recibir_catalogo_del_sitio`/`recibir_catalogo_rutas_del_sitio`), historial
de movimientos dentro de los ~2 minutos siguientes vía la sincronización
automática en segundo plano (`iniciar_sincronizacion_automatica`) -- no
hace falta ninguna acción manual extra para eso, sólo tener el dispositivo
activado y la app abierta.

### 8. Revisión de código — falta un gate estructural

**Qué falta:** no hay `CODEOWNERS` ni ninguna revisión obligatoria
configurada en GitHub — el flujo de PR existe, pero nada impide mergear
sin que nadie más lo haya mirado.

**Plan:**
1. Agregar un `CODEOWNERS` mínimo (dueño del repo como reviewer por
   defecto).
2. En GitHub → Settings → Branches, activar "Require a pull request before
   merging" + "Require approvals" sobre `main`. **Esto no se hace desde acá
   sin pedirlo explícitamente** — es un cambio de configuración de acceso
   del repositorio, no de código, y podría bloquear al propio dueño si se
   configura mal.

### 9. Compatibilidad hacia atrás entre dispositivos -- ✅ escritorio, 🚧 mobile

**Movido a "Cerrado" para escritorio, ver esa sección más abajo con el
detalle completo de implementación y tests.** Queda acá solo lo que falta.

**Lo que falta -- mobile (Android):** el mecanismo del lado del receptor
(`device-auth`) y del núcleo Rust compartido ya sirve a los dos, sin
cambios adicionales. Lo único que falta es que **Kotlin le pase su propia
versión real a Rust** (`BuildConfig.VERSION_NAME`, no la del crate
`mobile/rust-core`, que no es la misma cosa) -- hace falta:
1. Exponer un método UniFFI nuevo (algo como
   `Nucleo.establecerVersionApp(version: String)`) que llame al
   `AppCore::establecer_version_app` que ya existe.
2. Llamarlo una vez desde Kotlin, en `AplicacionViewModel.kt`, justo después
   de `Nucleo.abrir(...)`.

**Por qué no lo hice ya:** toca Kotlin, que no puedo compilar ni correr en
este sandbox (ver el resto de este documento y `docs/auditorias/auditoria-calidad-2026-09.md`
para el historial de por qué ahí conviene pedir confirmación antes de
tocar, aunque este cambio puntual es mucho más chico y de menor riesgo que
el de cifrado). Mientras tanto, mobile sigue con el comportamiento de
siempre: manda su versión sólo al activarse, no en cada renovación --
no es una regresión, es exactamente el estado actual.

### 10. ~~Cifrado del secreto de dispositivo en Android~~ — ya resuelto, este plan tenía la nota vieja

**Corrección 2026-09-17:** este punto decía, citando
`docs/auditorias/auditoria-calidad-2026-09.md`, que el secreto de
dispositivo en Android seguía sin cifrar y que había una decisión
pendiente (Keystore + `EncryptedFile`). Es una nota desactualizada — el
propio documento de auditoría quedó viejo. **Ya está resuelto**, desde
antes de esta rama: `mobile/android/app/src/main/java/com/brisas/controlacceso/SecretoDispositivoStore.kt`
(commit `7aed199`, 2026-09-10) implementa Android Keystore directo (no la
librería Jetpack `EncryptedFile` que recomendaba el doc, pero la misma
garantía real: la clave AES vive y se genera enteramente dentro del
Keystore, nunca sale en claro).

Verificado de punta a punta hoy, no solo el archivo suelto:
- La clave ya **no depende de `ANDROID_ID`** — eso era justo la causa raíz
  del incidente original (identificador cambia → clave distinta → ya no
  descifra). `ANDROID_ID` solo se usa hoy para descifrar, una única vez,
  el secreto legado de quien no había migrado todavía
  (`cargar_secreto_dispositivo_legado`, `mobile/rust-core/src/lib.rs:2009`
  — confirmado que existe y está implementada), y el archivo viejo se
  borra después de migrar.
- Conectado de verdad en `AplicacionViewModel.kt:52` (no es código
  muerto/sin usar).

**El error fue mío, primera vez que se preguntó por esto**: busqué
específicamente `EncryptedFile`/`androidx.security` (la solución que el
audit recomendaba) y al no encontrarla asumí que no se había hecho nada
— sin considerar que el equipo pudo haber implementado la misma garantía
con el API de Keystore directo, sin esa librería puntual.

**Lo único que sigue siendo un gap real:** no hay ningún test
automatizado de `AndroidKeystoreSecretoDispositivoStore` — no se puede
sin Robolectric (simula APIs de Android en tests de JVM), que el proyecto
no tiene instalado. Es exactamente la advertencia que el propio doc de
auditoría dejaba ("probarlo bien... no sólo unitarias") — el código
está bien, pero nadie le puso una prueba automática detrás. Si se decide
sumar Robolectric, el criterio de aceptación sería: un test que guarde un
secreto, lo recupere, y otro que confirme la migración+borrado del
archivo legado — sin tocar un dispositivo real.

### 11. Configuración por ambiente

**Qué falta:** `web/src/lib/supabase.ts` y `web-visitas/` tienen la URL y
la *publishable key* de Supabase hardcodeadas — un solo ambiente (producción)
sin forma de apuntar a un proyecto de staging para probar antes de un
release. (La clave en sí no es el problema — es pública a propósito, ver
el punto de seguridad #14 del análisis original — el problema es no poder
apuntar a otro proyecto sin editar código.)

**Plan:** variables de entorno de Vite (`import.meta.env.VITE_SUPABASE_URL`
/ `VITE_SUPABASE_PUBLISHABLE_KEY`), con el valor actual como default en
`.env.production` versionado (no es secreto) y la posibilidad de un
`.env.local` (gitignored) para apuntar a un proyecto de staging al
desarrollar. Requiere decidir si vale la pena mantener un segundo proyecto
Supabase de staging (costo) — `docs/recuperacion-supabase.md` ya deja el
runbook listo para levantarlo el día que se decida.

---

## ➖ Fuera de alcance, justificado

No se implementan porque no hay una necesidad real detrás, siguiendo el
mismo criterio que ya se usó para no sumar capas de arquitectura sin
justificación (ver el análisis de `AppCore`, sesión previa):

- **Backups tradicionales (respaldo periódico a archivo).** Fue una
  decisión de arquitectura ya tomada y documentada (`docs/pendientes.md`),
  no un olvido: incompatible con SQLCipher tal como estaba implementado, y
  se reemplazó por la sincronización con la nube como respaldo efectivo.
  Reabrir esto es el punto 7 de arriba, no un ítem separado.
- **Pruebas de carga clásicas (miles de usuarios concurrentes).** Lattis es
  una app desktop/mobile por sitio contra SQLite local, no un servidor
  multi-tenant. El benchmark que sí existe (`examples/benchmark_3way.rs`)
  mide el motor de cifrado de la base, que es la comparación que
  efectivamente importa acá.
- **Feature flags de producto (tipo LaunchDarkly/GrowthBook).** No hay
  evidencia de una necesidad real (rollouts graduales, A/B testing) — el
  mecanismo de *Cargo features* que ya existe (`nube`,
  `cifrado-sqlcipher`/`sqlite-plano`/`cifrado-sqlite3mc`) resuelve lo que
  el proyecto necesita hoy: variantes de build, no flags en runtime.
  Construir esto ahora sería exactamente "una capa que agrega archivos e
  interfaces sin aportar aislamiento real" — se reabre el día que aparezca
  un caso de uso concreto.
- **Migraciones "down"/reversibles.** El diseño actual (atómicas,
  transaccionales, con test de rollback ante fallo) ya cubre el riesgo real
  — una migración que falla no deja el esquema a medias. "Bajar" de versión
  de esquema en un sistema con datos reales en producción no es algo que
  se haga con una migración automática de todos modos; si hiciera falta,
  sería una intervención manual puntual, no una feature genérica a
  mantener.

---

## Resumen para decidir qué sigue

| # | Punto | Estado | Requiere decisión externa |
|---|---|---|---|
| 1 | Versión visible en UI | ✅ | No |
| 2 | `cargo fmt --check` en todos los jobs | ✅ | No |
| 3 | Logs en producción + 1 fallo silencioso corregido | ✅ (alcance acotado) | No |
| 4 | Backups: código muerto limpiado, doc corregido | ✅ | No |
| 5 | Observabilidad completa (5.1/5.2/5.3 logs + 5.4 Sentry) | ✅ | No |
| 6 | Diagnóstico exportable | 🚧 | No |
| 7 | Runbook recuperación base local | ✅ | No |
| 8 | CODEOWNERS + branch protection | ✅ CODEOWNERS / 🚧 el toggle | El toggle sí (vos, en Settings del repo) |
| 9 / 12 | Compatibilidad multi-versión | ✅ escritorio / 🚧 mobile | Mobile: no, sólo falta tocar Kotlin (confirmalo si querés que avance) |
| 10 | Cifrado secreto Android | ✅ (ya estaba hecho, nota vieja corregida) | Test con Robolectric: sí, si se quiere sumarlo |
| 11 | Config por ambiente (staging) | 🚧 proyecto creado (`docs/recuperacion-sitio-staging.md`), falta la variable de entorno para apuntar las apps ahí sin editar código | No -- ya dijiste que sí |
