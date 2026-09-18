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

### 2. Google OAuth -- diferido a propósito

El panel web (`web/`) no se prueba contra staging por ahora -- pedido
explícito del usuario (2026-09-18), es la parte más pesada de configurar
(un Client ID/Secret de Google Cloud aparte, apuntando a esta URL) y
staging es principalmente para probar sync de escritorio/mobile
(contratistas, ingresos, gafetes, rutas, proveedores), no el panel.

### 3. Cloudflare Access -- fuera de alcance

`sync-access-policy` está desplegada pero sin sus secrets
(`CF_API_TOKEN`/`CF_ACCOUNT_ID`/`CF_ACCESS_APP_ID`/`CF_ACCESS_POLICY_ID`,
`WEBHOOK_SHARED_SECRET`). Cualquier INSERT/UPDATE/DELETE en
`administradores_panel` va a fallar (`sync_access_policy()` hace `raise
exception` si faltan los secretos de Vault) -- irrelevante mientras nadie
inserte administradores acá, que es el caso mientras Google OAuth siga
diferido.

## Cómo apuntar las apps a este proyecto

Todavía no implementado -- `src/nube/mod.rs` (`BASE_URL`/`APIKEY`,
compartido por desktop y móvil) y `web/src/lib/supabase.ts` tienen la URL
de producción hardcodeada. Ver punto 11 de
`docs/auditorias/plan-qa-buenas-practicas-2026-09-17.md` -- variables de
entorno para poder elegir el proyecto sin editar código, ya decidido que
sí pero pendiente de implementar.

Mientras tanto, para una prueba puntual: cambiar a mano `BASE_URL`/`APIKEY`
en `src/nube/mod.rs` a los de este proyecto
(`https://pmrytjktlyiuikxuuxpr.supabase.co` / la publishable key de acá),
compilar, probar, y **revertir antes de commitear** -- no dejar nunca un
commit real apuntando a staging.

## Cómo probar de punta a punta una vez cargado el `DEVICE_SIGNING_KEY`

1. Llamar `admin-provision-device` (con un JWT de un admin en
   `administradores_panel` -- pero esa tabla está vacía en staging y
   Google OAuth está diferido, así que por ahora esta función sólo se
   puede probar con el `service_role` key directo, no desde el panel
   real) con `sitio_nombre`/`tipo`/`etiqueta` para generar el primer
   secreto de dispositivo.
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
