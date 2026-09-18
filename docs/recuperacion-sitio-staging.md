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
