# Recuperación del proyecto Supabase (`control-acceso-nube`)

Runbook para reconstruir el receptor en la nube desde cero si el proyecto
se pierde/corrompe, o para levantar uno nuevo (ej. de staging). No es
Terraform -- se evaluó y se descartó por ahora (ver `docs/pendientes.md`):
el provider oficial de Supabase todavía cubre flojo justo lo que acá
requiere configuración manual (Auth, secrets de Edge Functions), así que
sumarlo hoy sería una herramienta más para mantener sincronizada sin ganar
mucho sobre este runbook + las migraciones que ya existen.

## Qué reconstruye cada parte

- **Tablas, RLS, triggers, funciones** -- 100% en
  `supabase/migrations/*.sql`, ya versionado. `supabase db push` las aplica
  todas en orden.
- **Edge Functions** -- el código está en `supabase/functions/*`, pero los
  *secrets* que necesitan (ver tabla abajo) no viven en git a propósito y
  hay que volver a cargarlos a mano.
- **Auth (Google OAuth + plantilla de correo)** -- sólo configuración de
  dashboard, no hay forma de versionarlo en este repo.
- **Cloudflare Access** ("Panel Brisas") -- infraestructura aparte, ni
  siquiera es Supabase; se sincroniza sola desde `administradores_panel`
  una vez que `sync-access-policy` tiene sus secrets.

## 1. Crear el proyecto y aplicar el esquema

```sh
supabase projects create control-acceso-nube --org-id <tu-org>
supabase link --project-ref <el-ref-que-te-dio-el-comando-anterior>
supabase db push
```

Esto corre las ~35 migraciones de `supabase/migrations/` en orden --
tablas, políticas RLS, triggers de aviso en vivo (`emitir_cambio_nube_sitio`),
la extensión `pg_net`, todo. Verificá al final con
`supabase/tests/*.sql` (ver `docs/realtime-verificado.md` para cómo
correrlos) que las políticas quedaron como se espera.

## 2. Desplegar las Edge Functions y sus secrets

```sh
supabase functions deploy device-auth
supabase functions deploy admin-list-devices
supabase functions deploy admin-provision-device
supabase functions deploy admin-revoke-device
supabase functions deploy admin-suspend-device
supabase functions deploy admin-move-device
supabase functions deploy sync-access-policy
```

`SUPABASE_URL`/`SUPABASE_SERVICE_ROLE_KEY` los inyecta Supabase solo en
todas. Los secrets **propios** que hay que cargar a mano
(`supabase secrets set NOMBRE=valor`, guardados en un gestor de
contraseñas, nunca en el repo):

| Secret | Función que lo usa | De dónde sale |
| --- | --- | --- |
| `DEVICE_SIGNING_KEY` | `device-auth` | Par de llaves ES256 (JWK) generado una vez -- firma los tokens de dispositivo. Si se pierde, hay que regenerarlo Y volver a provisionar todos los dispositivos (sus tokens viejos ya no calzan con la llave pública nueva). |
| `CF_API_TOKEN` | `sync-access-policy` | Token de Cloudflare con permiso "Access: Apps and Policies" Edit. |
| `CF_ACCOUNT_ID` | `sync-access-policy` | ID de la cuenta de Cloudflare. |
| `CF_ACCESS_APP_ID` | `sync-access-policy` | ID de la app de Access "Panel Brisas". |
| `CF_ACCESS_POLICY_ID` | `sync-access-policy` | ID de la política dentro de esa app. |

Las cuatro últimas están auto-documentadas en el propio código de
`supabase/functions/sync-access-policy/index.ts`.

## 3. Configuración manual en el dashboard de Supabase

No hay forma de versionar esto -- checklist:

- **Authentication → Sign In / Providers → Google**: habilitar, cargar
  Client ID/Secret de la app de Google Cloud correspondiente, y agregar la
  URL del proyecto a los "Authorized redirect URIs" de esa app en Google
  Cloud Console.
- **Authentication → Emails → Magic Link**: cambiar la plantilla para que
  use `{{ .Token }}` en vez de `{{ .ConfirmationURL }}` -- sin esto,
  `useVerificacionPorCorreo.ts` (step-up del panel web) manda un link en
  vez del código de 6 dígitos que la UI espera.
- **Authentication → Policies**: revisar que "Leaked password protection"
  quede como se decidió (hoy desactivado a propósito -- el panel es sólo
  Google OAuth, no hay login por contraseña que proteger; ver auditoría de
  seguridad de esta sesión).

## 4. Cloudflare Access ("Panel Brisas")

Aparte de Supabase por completo. Necesita, en el dashboard de Cloudflare
Zero Trust:

1. Una aplicación de Access apuntando al dominio del panel web.
2. Una política vacía (el contenido real lo escribe
   `sync-access-policy` en cuanto tenga sus secrets -- no hace falta
   poblarla a mano, sólo que exista para tener un `CF_ACCESS_POLICY_ID`).
3. Cargar esos 4 IDs como secrets de la función (paso 2).
4. Disparar la función una vez a mano (`supabase functions invoke
   sync-access-policy`) o hacer cualquier cambio en `administradores_panel`
   para que el trigger la dispare sola y la política quede poblada.

## 5. Si el proyecto es NUEVO (URL distinta a la actual)

Tres lugares hardcodean la URL/clave pública del proyecto -- actualizar
los tres si el `project ref` cambió:

- `src/nube/mod.rs` (`BASE_URL`/`APIKEY`) -- lo usan desktop y móvil, que
  reexportan este mismo crate.
- `web/src/lib/supabase.ts` (`SUPABASE_URL`/`SUPABASE_PUBLISHABLE_KEY`).

## 6. Repoblar datos

- Estructura vacía y sana: no hace falta nada más que lo de arriba.
- Para vaciar datos de prueba conservando el sitio "Brisas" y el admin
  principal: `supabase/scripts/resetear_datos_prueba.sql`.
- Para contratistas/empresas reales (semi-producción, hoy en
  `contratistas_base_final_limpia_v15.sql` en la raíz del repo): **no**
  es un script de Supabase -- se importa contra la base local de un
  dispositivo real con `cargo run --example importar_catalogo_limpio` y
  se deja que la sincronización normal lo suba (ver
  `supabase/scripts/poblar_catalogo.sql` para el detalle exacto). Eso
  respeta `dispositivo_origen_id`/`sitio_id`, que un INSERT directo a
  Postgres no puede resolver solo.
- Gafetes (sin catálogo real todavía): plantilla de INSERT directo en el
  mismo `supabase/scripts/poblar_catalogo.sql`.
