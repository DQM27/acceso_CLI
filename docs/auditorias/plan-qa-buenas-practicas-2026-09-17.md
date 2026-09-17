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

## 🚧 Plan definido — requieren decisión o más alcance

### 5. Observabilidad — completar la instrumentación

**Estado real tras el punto 3:** el mecanismo existe y ya prueba un caso
real. Falta cubrirlo donde más importa.

**Plan, en orden de impacto:**

| Fase | Qué | Criterio de aceptación |
|---|---|---|
| 5.1 | Loguear cada `Err(...)` que un comando Tauri le devuelve al frontend (`desktop/src-tauri/src/comandos/*.rs`) — hoy el error viaja al toast del usuario pero no queda registrado | Provocar un error real (ej. cédula duplicada) y confirmar la línea en el log |
| 5.2 | Loguear los reintentos agotados/fallidos de `cola_salida` (la cola de sync offline) | Simular sitio sin internet, confirmar que cada intento fallido deja rastro, no solo el contador `intentos` en la DB |
| 5.3 | Logging para el fallo fatal de arranque (`mostrar_error_fatal_y_salir`) — hoy corre ANTES de que el plugin de logs exista, así que un fallo ahí (base dañada, candado de instancia tomado) sigue sin dejar archivo | Escribir directo a un archivo simple (sin depender del plugin, que todavía no está inicializado en ese punto) antes de mostrar el diálogo |
| 5.4 (decisión de negocio) | Evaluar un servicio externo de error-tracking (ej. Sentry) para que un fallo real llegue como notificación en vez de esperar que alguien revise el log de un sitio | Requiere elegir proveedor y aceptar el costo/exposición de datos — no se decide unilateralmente acá |

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

### 7. Runbook de recuperación — base local corrupta

**Qué existe hoy:** `docs/recuperacion-supabase.md` cubre muy bien el lado
nube (reconstruir el proyecto Supabase entero desde las migraciones
versionadas). **Lo que falta es el espejo para el sitio**: qué hacer si el
`.db` local de una PC se corrompe o el disco falla, dado que — punto 4 —
no hay respaldo local de ningún tipo.

**Plan:**
1. Documentar el camino real de recuperación hoy: reinstalar, dejar que
   `AppCore::abrir` cree una base nueva, y confiar en que la sincronización
   con la nube (`nube::recibir_*`) repuebla lo que ese sitio necesita ver.
2. Determinar y documentar QUÉ se pierde en ese camino (¿historial que solo
   vivía local? ¿algo que no viaja a la nube?) — esto requiere revisar qué
   tablas tienen contraparte remota y cuáles no.
3. Recién con (2) resuelto, decidir si vale la pena un respaldo local
   liviano (ej. copiar el `.db` cifrado a una carpeta de red 1 vez al día)
   o si depender de la nube es aceptable como está.

**Por qué no se implementa ya:** el paso 2 es información que no está en
el código — requiere que alguien del equipo confirme qué se considera
aceptable perder. No es una decisión técnica que se tome sola.

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

### 9. Compatibilidad hacia atrás entre dispositivos

**Qué falta:** con sync multi-dispositivo real (varios sitios, cada uno
pudiendo estar en una versión de app distinta), no hay ningún chequeo de
versión mínima. Si un dispositivo viejo manda un payload en un formato que
uno nuevo ya no espera (o viceversa), no hay nada que lo detecte antes de
que falle en producción.

**Plan:** agregar `version_minima_compatible` a la respuesta de
autenticación de dispositivo (`device-auth`, Edge Function ya existente) y
que el cliente rechace sincronizar con un aviso claro si está por debajo —
mejor que un error de deserialización críptico. Requiere decidir la
política de versiones soportadas (¿cuántas versiones atrás?) antes de
implementar.

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
| 5 | Observabilidad completa | 🚧 | Fase 5.4 sí (proveedor externo) |
| 6 | Diagnóstico exportable | 🚧 | No |
| 7 | Runbook recuperación base local | 🚧 | Sí (qué se acepta perder) |
| 8 | CODEOWNERS + branch protection | 🚧 | Sí (cambio de configuración del repo) |
| 9 | Compatibilidad multi-versión | 🚧 | Sí (política de versiones soportadas) |
| 10 | Cifrado secreto Android | ✅ (ya estaba hecho, nota vieja corregida) | Test con Robolectric: sí, si se quiere sumarlo |
| 11 | Config por ambiente (staging) | 🚧 | Sí (costo de un 2º proyecto Supabase) |
