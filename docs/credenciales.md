# Credenciales del proyecto — dónde vive cada una

Índice de referencia rápida: para CADA credencial dice dónde está guardada
y cómo rotarla/recuperarla, nunca el valor en sí. Ninguna credencial real
vive en este archivo ni en ningún otro archivo de este repo -- si alguna
vez aparece un valor real acá, es una fuga y hay que rotarlo de inmediato.

Para el detalle completo de reconstruir Supabase desde cero (incluye la
tabla de secrets de Edge Functions), ver `docs/recuperacion-supabase.md` --
este documento sólo resume/enlaza, no lo duplica.

## GitHub Actions (Settings → Secrets and variables → Actions)

Encriptados por GitHub, nadie puede leer el valor de vuelta, sólo
reemplazarlo (`gh secret set NOMBRE`). Usados por `.github/workflows/release.yml`:

| Secret | Para qué | Respaldo local |
| --- | --- | --- |
| `ANDROID_KEYSTORE_BASE64` | Firma el APK de release (`build-android`) | `mobile/android/app/keystore/release.keystore` (gitignored) |
| `ANDROID_KEYSTORE_PASSWORD` | Contraseña de ese keystore | `mobile/android/keystore.properties` (gitignored) |
| `ANDROID_KEY_ALIAS` | Alias de la llave dentro del keystore | ídem |
| `ANDROID_KEY_PASSWORD` | Contraseña de esa llave | ídem |
| `TAURI_SIGNING_PRIVATE_KEY` | Firma los instaladores de escritorio (`build-gui`) -- el updater verifica contra `plugins.updater.pubkey` en `desktop/src-tauri/tauri.conf.json` | `desktop/src-tauri/keystore/tauri-signing.key` (gitignored) |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Contraseña de esa llave | `desktop/src-tauri/keystore/keystore.properties` (gitignored) |

Verificado el 2026-09-12: la huella SHA-256 del certificado que firmó el
APK de la release `v1.5.0` (`a2c5e4a4...`) coincide exactamente con
`mobile/android/app/keystore/release.keystore` -- confirma que es ese
archivo, no ningún otro, el que CI usa de verdad.

## Supabase (proyecto `control-acceso-nube`, ref `xidaepyaljzkpbsxrqsm`)

- **URL + `anon`/publishable key**: públicas por diseño (Supabase las
  protege con RLS, no con secreto) -- hardcodeadas en `src/nube/mod.rs`
  (desktop + móvil) y `web/src/lib/supabase.ts`/`web-visitas/src/lib/supabase.ts`.
  Si el proyecto cambia de `ref`, hay que actualizar los tres lugares (ver
  `docs/recuperacion-supabase.md`, sección 5).
- **`SUPABASE_SERVICE_ROLE_KEY`**: la inyecta Supabase solo en cada Edge
  Function, no hay que cargarla a mano ni existe en ningún archivo.
- **Secrets propios de Edge Functions** (`DEVICE_SIGNING_KEY`,
  `CF_API_TOKEN`, `CF_ACCOUNT_ID`, `CF_ACCESS_APP_ID`,
  `CF_ACCESS_POLICY_ID`): cargados con `supabase secrets set`, viven del
  lado de Supabase (no en git) -- tabla completa con el propósito de cada
  uno en `docs/recuperacion-supabase.md`.
- **Vault** (Project Settings → Vault en el dashboard, distinto de los
  secrets de arriba): `sync_access_policy_apikey`,
  `sync_access_policy_webhook_secret`.
- **Auth (Google OAuth Client ID/Secret)**: sólo en el dashboard de
  Supabase (Authentication → Providers), no versionable.

## Cloudflare Access ("Panel Brisas")

Los 4 IDs/token (`CF_API_TOKEN`, `CF_ACCOUNT_ID`, `CF_ACCESS_APP_ID`,
`CF_ACCESS_POLICY_ID`) están arriba, cargados como secrets de Supabase --
Cloudflare en sí no guarda nada aparte, sólo consume la política que
escribe `sync-access-policy`.

## Windows DPAPI / Android Keystore (clave de cifrado de la base local)

No hay nada que guardar/rotar acá a mano: la clave de la base SQLCipher se
genera aleatoria por dispositivo y queda protegida por DPAPI (`Current
User`, en desktop) o Android Keystore (en móvil) -- atada a esa
instalación de Windows/ese dispositivo, no exportable ni recuperable si se
pierde el perfil de usuario o se resetea el teléfono. Ver
`docs/decisiones-tecnicas.md` para el detalle de diseño.

## Historial relevante

- **2026-09-12**: se regeneró la llave de firma de Tauri (la vieja no
  tenía ningún respaldo local conocido, sólo el secret de GitHub) y se
  reemplazaron `TAURI_SIGNING_PRIVATE_KEY`/`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
  por la nueva. Se actualizó `plugins.updater.pubkey` en
  `desktop/src-tauri/tauri.conf.json` a juego. Confirmado con el usuario
  antes de rotar: ninguna instalación real de escritorio dependía de la
  llave vieja (todo lo publicado hasta `v1.5.1` fue de prueba) -- si
  hubiera existido una, esta rotación la habría dejado sin poder recibir
  más actualizaciones automáticas.
- Se eliminó el 2026-09-12 un keystore de Android viejo y sin usar
  (`mobile/android/keystore/release.keystore.jks`, contraseña distinta a
  la actual) que la documentación vieja de `mobile/README.md` marcaba
  como "el real" por error -- confirmado con el usuario que ningún
  dispositivo real dependía de esa clave antes de borrarla.
