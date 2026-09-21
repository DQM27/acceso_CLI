# Proyecto de staging (`control-acceso-staging`)

Creado 2026-09-18 vía el MCP de Supabase (`create_project`), siguiendo
`docs/recuperacion-supabase.md` -- este documento es la bitácora de ESE
proyecto en particular: qué se hizo, qué falta a mano, y qué se descartó
a propósito. Pensado para poder destruirlo y reconstruirlo las veces que
haga falta, probando cambios de esquema/RLS/Edge Functions antes de
tocar producción.

- **Project ref:** `pmrytjktlyiuikxuuxpr`
- **Región:** `us-east-1` (misma que producción)
- **URL:** `https://pmrytjktlyiuikxuuxpr.supabase.co`

## Qué ya quedó hecho

- **Esquema completo** -- las 75 migraciones de `supabase/migrations/`
  aplicadas en orden (con los 2 fixes de `docs/recuperacion-supabase.md`
  para el esquema `private` y la política de broadcast). Verificado 1:1
  contra producción: mismas 20 tablas, mismo `rls_enabled` en todas.
- **Las 10 Edge Functions**, desplegadas con el código REAL en
  producción (no siempre igual al de git -- ver "Drift encontrado" abajo),
  mismo `verify_jwt` que cada una tiene en producción.
- Dos URLs que estaban hardcodeadas a producción dentro de funciones SQL
  (`sync_access_policy()`, en dos versiones distintas a lo largo del
  historial de migraciones) se corrigieron a apuntar a este proyecto en
  vez de a `xidaepyaljzkpbsxrqsm`.

## Drift encontrado entre git y producción (corregido acá)

`supabase/functions/admin-list-devices/index.ts` en git todavía tenía
`select("id, nombre, direccion, created_at")` -- la columna `direccion`
se eliminó de `sitios` en la migración `elimina_direccion_de_sitios`
(2026-09-12), y la función DESPLEGADA en producción ya tenía el fix
(`select("id, nombre, created_at")`), pero nadie subió ese cambio a git.
Corregido en este mismo commit -- si se hubiera desplegado el archivo
viejo de git a producción alguna vez, esa función habría empezado a
fallar con un 500 real.

**Lección para no repetir esto:** cuando se despliega un hotfix de una
Edge Function a mano (dashboard o `supabase functions deploy` directo,
sin pasar por CI), el archivo en git tiene que actualizarse en el mismo
momento -- son dos acciones separadas que nada las mantiene sincronizadas
solas.

## Drift #2: triggers de realtime faltantes en el esquema versionado (2026-09-21)

Al probar el aviso en vivo (Realtime broadcast, `desktop/src/nubeRealtime.ts`)
contra este proyecto, el fallback periódico de 2 minutos funcionaba pero el
aviso instantáneo NUNCA llegaba para altas/bajas de **contratistas, empresas,
gafetes e ingresos** -- el flujo central de entrada/salida, la parte más
crítica de todo el sistema.

**Causa:** los triggers `contratistas_emitir_cambio_nube`,
`empresas_emitir_cambio_nube`, `gafetes_emitir_cambio_nube` e
`ingresos_emitir_cambio_nube` (todos ejecutan
`private.emitir_cambio_nube_sitio()`, ver
`avisa_cambio_nube_segun_quien_escribe_no_quien_creo_la_fila.sql`) existían en
producción pero **en NINGÚN archivo de `supabase/migrations/`** -- se habían
creado a mano, fuera de una migración versionada, antes de que el resto de
las tablas adoptara el hábito de agregar su propio trigger en la misma
migración que las crea (comparar con `avisa_cambio_nube_en_usuarios.sql` o
`avisa_cambio_nube_en_cita_sitios.sql`, que sí quedaron documentadas). Este
proyecto de staging, reconstruido replicando sólo las migraciones del repo,
nunca los tuvo -- confirmado comparando
`information_schema.triggers` entre los dos proyectos.

**Por qué importa más que un bug puntual de sandbox:** si algún día hace
falta reconstruir PRODUCCIÓN desde cero a partir de `supabase/migrations/`
(desastre real, no solo un sandbox de prueba), esos mismos 4 triggers
faltarían ahí también -- el esquema versionado en git no bastaba para
reproducir el 100% de la infraestructura real. Es la misma categoría de
problema que el drift de Edge Functions de la sección de arriba (código real
≠ código en git), pero en DDL de base de datos, más difícil de notar porque
no rompe con un error -- simplemente el realtime queda mudo y todo sigue
"funcionando" vía el pulso de 2 minutos, silencioso hasta que alguien nota la
demora.

**Arreglado:** migración `20260921140000_agrega_triggers_faltantes_emitir_cambio_nube.sql`
agregada al repo (usa `drop trigger if exists` + `create trigger`, segura de
aplicar también en producción sin duplicar) y aplicada en ambos proyectos.

**Lección para no repetir esto:** cualquier `CREATE TRIGGER`/`CREATE POLICY`/
cambio de esquema que se aplique a mano contra producción (dashboard o SQL
directo) tiene que convertirse en una migración committeada en el MISMO
momento -- nunca "ya lo aplico y después escribo la migración", porque
"después" es exactamente lo que no pasó acá. Si en algún momento se sospecha
drift de nuevo, comparar `information_schema.triggers`/`pg_policies`/
`information_schema.routines` completos entre `xidaepyaljzkpbsxrqsm` y un
proyecto reconstruido desde migraciones es la forma de confirmarlo (así se
encontró este caso).

## Falta a mano (no lo puede hacer el MCP)

### 1. `DEVICE_SIGNING_KEY` -- bloqueante para cualquier prueba real

Sin esto, **ningún dispositivo puede activarse ni sincronizar** contra
este proyecto -- `device-auth` no puede firmar tokens.

Par de llaves ES256 generado con `openssl` para este proyecto -- el JWK
completo (incluye la clave PRIVADA, campo `d`) **nunca va a este archivo
ni a ningún commit** -- se le pasó directo al usuario en el chat de la
sesión que armó este proyecto (2026-09-18), no queda registrado acá.
Si se perdió, no hay forma de recuperarlo -- hay que generar uno nuevo
(`openssl ecparam -name prime256v1 -genkey -noout` + extraer `x`/`y`/`d`
en base64url, o `supabase gen signing-key --algorithm ES256` si se tiene
el CLI) y repetir los dos pasos de abajo con el nuevo.

Dos pasos, los dos manuales:

1. **Secret de la Edge Function** -- `supabase secrets set
   DEVICE_SIGNING_KEY='<el JSON de arriba, en una sola línea>'
   --project-ref pmrytjktlyiuikxuuxpr` (o pegarlo en el dashboard,
   Edge Functions → Secrets).
2. **Importar y rotar en Auth → JWT Signing Keys** (dashboard,
   `/dashboard/project/pmrytjktlyiuikxuuxpr/settings/jwt`) -- sin esto,
   Postgres/PostgREST no confía en tokens firmados con esta clave, aunque
   la Edge Function sí pueda firmarlos. Pasos: **Create a new key** →
   pegar el mismo JSON de arriba como standby key → **Rotate keys** para
   activarla. Ver `docs/auditorias/plan-qa-buenas-practicas-2026-09-17.md`
   si hace falta el detalle completo de por qué son dos sistemas
   separados (JWT secret legacy vs. signing keys).

### 2. Google OAuth -- cerrado (2026-09-18)

Cliente OAuth "Cliente web 2" creado en el proyecto de Google Cloud
`mega-brisas`, con el único redirect URI necesario:
`https://pmrytjktlyiuikxuuxpr.supabase.co/auth/v1/callback` (y
`http://localhost:5173` como JavaScript origin autorizado, para probar el
panel local). Client ID/Secret cargados en **Authentication → Providers →
Google** del proyecto de staging (el secret no se documenta acá, solo
vive en Supabase y en la consola de Google -- si hace falta rotarlo,
Google Cloud Console → Clientes → ese cliente → regenerar).

Probado de punta a punta: login real contra `http://localhost:5173`
(panel apuntado a staging vía `web/.env.local`, ver más abajo) entra
correctamente y muestra el panel (vacío de datos, como corresponde a un
proyecto nuevo).

`administradores_panel` en staging tiene un solo correo autorizado por
ahora (el del probador). Insertar más así:
`insert into public.administradores_panel (correo, creado_en) values ('correo@ejemplo.com', now());`
-- pero antes hace falta el paso 3 de abajo (los dos secretos de Vault),
si no el trigger `sync_access_policy()` corta el INSERT con una
excepción.

### 3. Cloudflare Access -- sigue fuera de alcance, pero el trigger ya no bloquea

`sync-access-policy` (la Edge Function) sigue desplegada sin sus secrets
reales (`CF_API_TOKEN`/`CF_ACCOUNT_ID`/`CF_ACCESS_APP_ID`/`CF_ACCESS_POLICY_ID`).
Sí se cargaron los dos secretos de **Vault** que el trigger de Postgres
necesita para no abortar la transacción
(`sync_access_policy_apikey`/`sync_access_policy_webhook_secret`, valores
aleatorios generados con `gen_random_bytes`, sin relación con los
secrets reales de Cloudflare) -- el trigger dispara el `net.http_post`
de forma asíncrona (`pg_net`), así que el INSERT/UPDATE/DELETE en
`administradores_panel` funciona igual aunque esa llamada después falle
puertas adentro (401, porque `WEBHOOK_SHARED_SECRET` no está seteado
como secret de la función). Cuando se decida armar Cloudflare Access de
verdad para staging, hay que reemplazar esos dos secretos de Vault por
los reales y sí setear los `CF_*`/`WEBHOOK_SHARED_SECRET` de la función
(`supabase secrets set ...`).

## Cómo apuntar las apps a este proyecto

Ya implementado (punto 11 del plan de QA, cerrado 2026-09-18) -- ver
`docs/auditorias/plan-qa-buenas-practicas-2026-09-17.md`. En resumen:

- **Desktop/mobile:** variables de entorno
  `CONTROL_ACCESO_SUPABASE_URL`/`CONTROL_ACCESO_SUPABASE_APIKEY` (leídas
  por `src/nube/mod.rs`, `OnceLock`, sin recompilar nada distinto --
  solo hay que exportarlas antes de correr/compilar). Sin ellas, cae al
  default de producción.
- **Web/web-visitas:** crear `.env.local` (gitignored, no tocar el
  `.env` versionado) con:
  ```
  VITE_SUPABASE_URL=https://pmrytjktlyiuikxuuxpr.supabase.co
  VITE_SUPABASE_PUBLISHABLE_KEY=sb_publishable_29DwMvfyj8Jq--LBcqxtBA_pTwWrDH4
  ```
  Vite lo recoge solo con `npm run dev`/`npm run build`.

## Cómo probar de punta a punta una vez cargado el `DEVICE_SIGNING_KEY`

1. Llamar `admin-provision-device` con `sitio_nombre`/`tipo`/`etiqueta`
   para generar el primer secreto de dispositivo -- ya se puede hacer
   desde el panel real (`http://localhost:5173` apuntado a staging,
   logueado con Google) en vez de necesitar el `service_role` key
   directo, ahora que `administradores_panel` tiene al menos un correo
   autorizado.
2. Activar un dispositivo (desktop o mobile, apuntado a este proyecto,
   ver arriba) con ese secreto.
3. Confirmar que trae el catálogo (vacío, es un proyecto nuevo) y que
   puede crear contratistas/ingresos/etc. sin que ninguna política RLS
   los rechace.

## Para destruir y reconstruir

No hay nada especial que preservar -- es sandbox puro. Borrar el proyecto
desde el dashboard o `pause_project`/eliminarlo, y repetir
`docs/recuperacion-supabase.md` con un nombre nuevo (o el mismo, Supabase
no reutiliza el `project ref` de uno borrado). El `DEVICE_SIGNING_KEY` de
este documento queda inválido para un proyecto nuevo -- hay que generar
uno distinto (nunca reciclar el mismo par de llaves entre dos proyectos,
ni siquiera dos de staging).

## Aislar también lo local (2026-09-20)

Apuntar a staging cambia solo la nube -- por defecto la app sigue
escribiendo en el `control_acceso.db` real y reusando el secreto de
activación real. Para un aislamiento completo hacen falta además:

- **`CONTROL_ACCESO_DB`** (ver `src/database/connection.rs`,
  `DATABASE_PATH_ENV`): ruta absoluta a un archivo distinto de
  `%LOCALAPPDATA%\ControlAcceso\control_acceso.db`.
- **`APPDATA`**: `dispositivo-nube.secret` (el secreto de activación de
  este dispositivo) y `db_key.dat` (clave SQLCipher en escritorio) viven
  en `%APPDATA%\ControlAcceso` -- separado de `%LOCALAPPDATA%` a
  propósito (ver `src/nube/credenciales.rs`, `ROAMING_APP_DATA_ENV`).
  Ese módulo no tiene su propia variable de override, así que la única
  forma de aislarlo sin tocar código es sobreescribir `APPDATA` mismo
  para la sesión de terminal que corre el sandbox -- eso también hace
  que cualquier otro programa lanzado desde esa misma terminal vea el
  `APPDATA` distinto, sin efecto fuera de esa terminal.

`scripts/activar_sandbox.ps1` pone las tres variables
(`CONTROL_ACCESO_SUPABASE_URL`/`APIKEY`, `CONTROL_ACCESO_DB`, `APPDATA`)
de una sola vez. Uso: `. .\scripts\activar_sandbox.ps1` (con el punto y
espacio al inicio) y correr `cargo run`/`npm run tauri dev` desde esa
misma terminal.

**Mobile/APK (cerrado 2026-09-20):** `mobile/android` ya fija
`CONTROL_ACCESO_SUPABASE_URL`/`APIKEY` antes de la primera llamada al
núcleo -- `AplicacionControlAcceso.kt` (nueva `Application`, registrada
en `AndroidManifest.xml` con `android:name=".AplicacionControlAcceso"`)
llama `android.system.Os.setenv(...)` en `onCreate()`, que corre antes de
cualquier Activity/ViewModel. Android no hereda variables de entorno de
shell como Windows, así que `Os.setenv` es el único equivalente --
mismo criterio de fondo que `scripts/activar_sandbox.ps1`.

Gateado por `BuildConfig.DEBUG` (mismo patrón que el `FLAG_SECURE` de
`MainActivity.kt`): **todo build de debug apunta a staging
automáticamente, sin nada que activar a mano** -- un `assembleDebug`/`run`
normal desde Android Studio o `./gradlew installDebug` nunca toca
producción. Un build de **release** (firmado, el que sale por GitHub
Releases) sigue apuntando a producción sin cambios. No hay override en
sentido contrario (forzar staging en release o producción en debug) --
si algún día hace falta, agregar un `buildConfigField` en
`app/build.gradle.kts` en vez de tocar el `Os.setenv` a mano.

Verificado con `./gradlew :app:compileDebugKotlin` (compila limpio); no
se corrió aún en un dispositivo/emulador real.

**`applicationIdSuffix` en debug (2026-09-20):** al instalar el primer
APK de prueba en un teléfono real, Android lo ofreció como
"actualización" en vez de instalación nueva -- señal de que debug y
release compartían `applicationId` (`com.dqm27.lattis`, sin sufijo).
Sin esto, instalar un build de prueba corre el riesgo real de
reemplazar la app de producción instalada en ese mismo teléfono.
Agregado `applicationIdSuffix = ".debug"` al bloque `debug {}` de
`app/build.gradle.kts` -- debug y release quedan como dos apps
distintas (`com.dqm27.lattis.debug` / `com.dqm27.lattis`), coexisten sin
pisarse. No afecta el `.so`/bindings de UniFFI (van por `namespace`, no
por `applicationId`) -- solo hizo falta recompilar el APK, no el núcleo
Rust.

## Generador local de secretos de dispositivo (2026-09-20)

`scripts/generar_secreto_dispositivo.mjs` genera, sin red, el mismo
formato de secreto que `admin-provision-device` (`uuid+uuid`, hash
SHA-256 hex) e imprime el SQL para insertarlo a mano contra el proyecto
de staging. Pensado para activar dispositivos de prueba sin pasar por el
panel/Google OAuth. Uso:

```
node scripts/generar_secreto_dispositivo.mjs --sitio "Sitio de prueba" --tipo pc --etiqueta "PC recepcion"
```

## Clonado de datos de producción → staging (2026-09-20)

A pedido explícito del usuario ("tener información con qué trabajar y no
ensuciar el otro"): se copiaron datos reales de `control-acceso-nube`
(`xidaepyaljzkpbsxrqsm`) a `control-acceso-staging`
(`pmrytjktlyiuikxuuxpr`), tabla por tabla vía el MCP de Supabase
(`execute_sql`), sin `pg_dump`/CLI (no instalados en esta máquina).

**Copiado completo:** `sitios` (2), `dispositivos` (7 -- solo para
integridad de FK, ver abajo), `empresas` (45), `empresas_proveedor` (4),
`gafetes` (45), `rutas` (86), `ingresos` (73),
`prestamos_gafete_provisional` (2).

**Copiado parcial (muestra, no la tabla completa):**
- `contratistas`: 150 de 402 filas de producción.
- `encargados_ruta`: 100 de 1438 filas de producción.

Motivo del corte: cada llamada a `execute_sql` tiene un límite de tamaño
de resultado (~25-30k caracteres); mover el resto habría significado
~15-20 llamadas más solo para esas dos tablas. Si se necesita el resto
en algún momento, repetir el mismo patrón con `order by id limit X
offset Y` sobre producción y pegar el INSERT resultante en staging.

**Deliberadamente NO copiado:** `usuarios`, `administradores_panel`,
`anfitriones`, `citas`/`cita_sitios`/`cita_visitantes`,
`movimientos_visita` -- todas ligadas a identidades de Supabase Auth o a
funcionalidad de visitas (V2, no prioridad actual). Clonar sus filas
crearía referencias rotas (`auth_user_id` apuntando a un usuario de Auth
que no existe en este proyecto) sin ganar nada útil para probar
contratistas/rutas/gafetes.

**FKs con `session_replication_role = replica`:** algunas filas de
`gafetes`/`ingresos`/`prestamos_gafete_provisional` referencian
`contratista_id`/`encargado_id` que quedaron fuera de la muestra parcial
de arriba. Para no bloquear el insert completo por esas pocas filas, se
desactivaron temporalmente los triggers de FK (`SET
session_replication_role = replica; ... SET session_replication_role =
default;`) solo durante esos inserts. Resultado: unas pocas filas de
`gafetes`/`ingresos`/`prestamos_gafete_provisional` en staging apuntan a
un `contratista_id`/`encargado_id` que no existe ahí -- inofensivo para
pruebas (son sandbox, no hay integridad que proteger), pero una consulta
con `inner join` a `contratistas`/`encargados_ruta` puede devolver menos
filas de las que aparecen sueltas en esas tablas.

**Nota sobre `dispositivos`:** se clonó completa (con `secret_hash`
real de producción) solo para que las FK de las demás tablas
(`dispositivo_origen_id`) no rompan -- esos hashes no dan acceso útil
por sí solos (el proyecto de staging tiene su propio
`DEVICE_SIGNING_KEY`, distinto del de producción) y no se puede
recuperar el secreto en texto plano desde el hash.

**Usuario ROOT de prueba (creado a mano, 2026-09-20):** cédula `1`,
contraseña `daniel`, sitio `Brisas`. Insertado directo en `auth.users` +
`auth.identities` (bcrypt vía `pgcrypto`, `crypt(...,gen_salt('bf'))`) +
`public.usuarios` con `rol='ROOT'` -- no vino de `admin-create-usuario`
porque esa Edge Function pide login de administrador del panel primero.
Mismo email sintético que usa esa función:
`emailSinteticoParaCedula()` → `1@brisas.local`. Si se necesita otro
usuario de prueba, repetir el mismo patrón (o usar el panel real una vez
que tenga datos).

**Trampa real al insertar en `auth.users` a mano:** dejar
`confirmation_token`/`recovery_token`/`email_change_token_new`/
`email_change`/`email_change_token_current`/`reauthentication_token` en
`NULL` (su default en el esquema) rompe el login -- el código Go de
GoTrue las escanea como `string` no-nullable, no como `sql.NullString`.
Error real visto en los logs de auth (`query_logs`, `source=auth_logs`):
`error finding user: sql: Scan error on column index 3, name
"confirmation_token": converting NULL to string is unsupported` (500,
que el cliente Rust reporta como "El servidor devolvió una respuesta
inesperada" porque el body de error no matchea `RespuestaToken`). Fix:
poner esas seis columnas en `''` en vez de `NULL` al insertar (o con un
`UPDATE` después, como se hizo acá). Si se repite este patrón para otro
usuario de prueba, incluir el `''` desde el `INSERT` directamente.

**Drift de esquema pendiente (ya documentado antes de este cambio):**
staging todavía no tiene las 2 migraciones más nuevas de `main`
(`instala_extension_unaccent`, `unicidad_nombre_empresas_ignora_mayusculas_y_tildes`)
-- no bloqueó este clonado porque ninguna tabla tocada depende de esas
migraciones, pero sigue pendiente aplicarlas si se prueba algo que sí las
necesite.
