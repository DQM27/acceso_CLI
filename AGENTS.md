# Notas del proyecto

## Preferencias del usuario

- **NUNCA usar el término prohibido para el puesto de control** en conversación, código,
  comentarios ni documentación. Para referirse al puesto de control de acceso usar
  términos como "puesto de control", "portería" o "punto de acceso".
- Comunicarse siempre en español.
- Siempre que se complete un cambio exitoso, hacer un commit bien documentado y subirlo.
- **Todos los commits van sólo a nombre de `Daniel Quintana <daniel.bleach1@gmail.com>`**,
  como autor y como committer (`git config user.name`/`user.email` antes del primer
  commit). Sin líneas `Co-Authored-By`, `Claude-Session` ni ninguna otra atribución a
  asistentes en commits ni en pull requests.

## Supabase

- **Permisos explícitos** (desde `20261020130000_permisos_api_explicitos`): lo que crea
  una migración en `public` ya NO recibe permisos solo, y ninguna función nueva (de
  ningún esquema) es ejecutable por `public`. Cada migración da a mano, junto con su RLS:
  - tablas: `grant ... on public.x to authenticated` (lo que usen apps/panel) **y**
    `to service_role` (Edge Functions de administración; salta RLS pero no los GRANT);
  - funciones RPC: `grant execute ... to authenticated, service_role`; las de `private`
    que use la RLS, `to authenticated`;
  - Realtime necesita `select` para `authenticated` en la tabla publicada.
  - **Nunca** a `anon`: nadie entra sin sesión (los equipos usan su JWT con rol
    `authenticated`). Única excepción: la telemetría de staging (`supabase/scripts/`).
- Cada migración lleva su batería en `supabase/tests/`, probada en staging dentro de una
  transacción con `rollback`. Mergear a `main` aplica las migraciones en producción.

## Entorno de pruebas en sesiones en la nube

Lo que hace falta preparar en un contenedor nuevo para compilar y probar todo (nada
de esto va al repo):

- **Android (Kotlin, `testDebugUnitTest`)**: Java 21 ya viene. Bajar las
  command-line tools de `https://dl.google.com/android/repository/commandlinetools-linux-13114758_latest.zip`
  a `/opt/android-sdk/cmdline-tools/latest`, aceptar licencias y
  `sdkmanager "platforms;android-36" "build-tools;36.0.0" "platform-tools"`. Correr
  Gradle con `ANDROID_HOME=/opt/android-sdk` (sin crear `local.properties`). Maven
  Central responde 429 a las IPs de la nube: un init script en
  `~/.gradle/init.d/` que cambie `repo.maven.apache.org`/`repo1.maven.org` por el
  espejo de Google `https://maven-central.storage-download.googleapis.com/maven2/`.
  No hace falta `google-services.json` ni el NDK para compilar Kotlin y correr las
  pruebas unitarias.
- **Escritorio (`cargo check` en Linux)**: faltan las librerías GTK. Con archivos
  `.pc` falsos (Version 99, sin Libs) en un directorio y `PKG_CONFIG_PATH` apuntando
  ahí, `cargo check` compila el código de la app; quedan 4 errores de código sólo de
  Windows (`zeroize`, `intentar_abrir_nucleo`), que no son del cambio.
- **Deno (Edge Functions)**: descargarlo lo bloquea el clasificador de seguridad
  del entorno; lo valida el workflow `reglas.yml`. La lógica que sólo usa
  `node:zlib`/`node:buffer` se puede correr con Node 22 (`--experimental-strip-types`).
