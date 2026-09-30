# Traspaso: registro de dispositivos por código + clave

> Documento vivo para retomar este trabajo en otra sesión. Rama:
> `feat/registro-dispositivos-seguro` (sale de `main`). Diseño y motivos:
> `docs/features-futuras/propuesta-registro-dispositivos.md`.
> Última actualización: 2026-09-30.

## Reglas de esta tarea (decididas por el usuario)

- **Supabase: sólo staging (sandbox)**, proyecto `pmrytjktlyiuikxuuxpr`.
  Producción (`xidaepyaljzkpbsxrqsm`) NO se toca. Antes hay que probarlo
  con equipos reales (ver "Prueba con equipos reales") y el usuario tiene
  que aprobarlo explícitamente.
- **Sólo instalaciones nuevas** (decisión 2026-09-30): cada equipo se
  instala de cero y se vincula con un código del panel.
- **Sólo dos acciones: Registrar y Retirar** (decisión 2026-09-30, "la
  solución más simple es la mejor"). No hay suspensión temporal ni
  re-vinculación: un equipo reinstalado se registra como dispositivo nuevo
  y el anterior se retira; los datos vuelven en la primera sincronización.
- **Sin compatibilidad con el secreto de dispositivo.** Todos los equipos se
  reinstalan y se vinculan con código; no hay migración automática ni camino
  legado en ninguna capa.
- Commits sólo a nombre de Daniel Quintana, sin líneas de atribución.
- Nunca usar el término prohibido para el puesto de control (ver `AGENTS.md`).

## Cómo funciona (resumen)

```
Panel web ── Registrar (admin-provision-device) ──► código XXXX-XXXX-XX + QR
                                                     (15 min, un solo uso)
Equipo ── genera par EC P-256 (la privada nunca sale) ──► device-vincular { codigo, clave_publica_jwk }
          ◄── primer token (JWT 1 h con sitio_id, tipo, huella)
Equipo ── device-auth { desafio: true } ──► desafío (JWT del servidor, 120 s)
Equipo ── device-auth { asercion: JWS(desafío) firmado, kid = huella } ──► token
Postgres ── política RESTRICTIVA "solo dispositivos vigentes" en toda tabla con RLS:
            el token sólo sirve si el dispositivo no está retirado
            y la huella del token es la de su clave vigente.
Retirar (admin-revoke-device) ──► corta al instante + broadcast `dispositivo_expulsado`
                                   (el equipo cierra el canal y no reintenta)
```

- **Código**: 10 caracteres de `23456789ABCDEFGHJKLMNPQRSTUVWXYZ`; se guarda
  sólo su SHA-256. QR: `brisas-acceso://vincular?codigo=XXXXXXXXXX`.
- **Huella**: RFC 7638 (SHA-256 del JWK canónico, base64url). Es el `kid` de
  la aserción y el claim `huella` del token.
- **Canje**: `canjear_codigo_vinculacion` sólo vincula un dispositivo recién
  registrado (sin clave) y no retirado. Un código no puede reemplazar la
  clave de un dispositivo ya vinculado.
- **Eventos de seguridad** (`eventos_seguridad_dispositivos`, visibles en el
  panel): `codigo_inexistente`, `codigo_usado`, `codigo_vencido`,
  `codigo_anulado`, `firma_invalida`,
  `hardware_distinto`.

## Mapa de archivos

| Capa | Archivos |
| --- | --- |
| Migraciones | `supabase/migrations/20260929200000_revocacion_efectiva_dispositivos.sql` (clave_huella, `private.dispositivo_vigente()`, política restrictiva en loop, trigger de expulsión por Realtime), `20260929200100_vinculacion_dispositivos_por_codigo.sql` (quita `secret_hash`, `codigos_vinculacion`, `eventos_seguridad_dispositivos`, RPC `canjear_codigo_vinculacion`) |
| Tests SQL | `supabase/tests/dispositivos_vigentes_y_vinculacion.sql` + los `*_autorizacion.sql` (dispositivos con clave y claim `huella`) |
| Edge Functions | `supabase/functions/_shared/{http,admin,dispositivos}.ts`, `device-vincular`, `device-auth`, `admin-provision-device`, `admin-list-devices`, `admin-revoke-device` (+ refactor a `_shared` del resto de `admin-*`) |
| Núcleo Rust | `src/nube/firmante.rs` (trait `FirmanteDispositivo`, aserción, `FirmanteArchivo` con DPAPI), `cache_token.rs` (`vincular`, `autenticar_con_cache`, `vinculado`), `cliente.rs` (`vincular_con_codigo`, `autenticar_con_firmante`), `credenciales.rs` (sólo almacenamiento protegido), `application/nube.rs` (`vincular_dispositivo_inicial`), `application/con_nube.rs` (reciben `&CacheTokenDispositivo`) |
| Pruebas núcleo | `examples/probar_vinculacion.rs` (E2E contra staging), `tests/nube_smoke.rs` (manual, `#[ignore]`) |
| Escritorio | `desktop/src-tauri/src/{lib.rs,estado.rs,comandos/nube.rs}`; frontend `PrimerArranque.tsx`, `expulsionNube.ts`, `nubeRealtime.ts` |
| Panel web | `web/src/api/dispositivos.ts`, `pantallas/Dispositivos.tsx` (Registrar por sitio, Retirar, columna Vinculación, "Intentos y alertas"), `componentes/CodigoVinculacionEmitido.tsx` (QR con `uqr`, cuenta regresiva) |
| Móvil Rust | `mobile/rust-core/src/firmante.rs` (callback UniFFI `AlmacenClaveDispositivo`), `nube.rs` (`vincular_dispositivo_inicial`, `descartar_token_nube`, `nube_configurada`), APIs `*_verificado` en `ingresos.rs`/`gafetes.rs`/`proveedores.rs` |
| Android | `AlmacenClaveKeystore.kt` (alias `control_acceso_identidad_dispositivo`, secp256r1), `PantallaEscanearCodigoVinculacion.kt` (CameraX + ML Kit), `PantallaPrimerArranque.kt`, `PrimerArranqueViewModel.kt`, `ExpulsionNube.kt`, `NubeRealtime.kt`; bindings regenerados en `app/src/main/java/uniffi/control_acceso_mobile/` |

Retirado: `SecretoDispositivoStore.kt`, `scripts/generar_secreto_dispositivo.mjs`,
la feature `cifrado-secreto-dispositivo-portable` y `aes-gcm`, todas las APIs
`*_con_secreto` del núcleo móvil, `autenticar_dispositivo` del núcleo,
`NubeDelDispositivo`, y el estado `secreto_legado` del panel y del escritorio.

Retirado en la simplificación (2026-09-30): la suspensión temporal
(`suspended_at`, `admin-suspend-device`, `NubeError::DispositivoSuspendido`)
y la re-vinculación (`admin-crear-codigo-vinculacion`, `dispositivo_esperado`,
el evento `codigo_de_otro_dispositivo`, `revincular_dispositivo` en
escritorio y móvil, `VincularEquipoModal.tsx`, la huella en el aviso de
expulsión). Las Edge Functions `admin-suspend-device` y
`admin-crear-codigo-vinculacion` siguen desplegadas en staging y hay que
borrarlas desde el dashboard.

## Estado

| Parte | Estado |
| --- | --- |
| Migraciones | Aplicadas en staging (el retiro de `secret_hash` se aplicó como delta con `execute_sql`, idéntico a los archivos) |
| Tests SQL en staging | 10 de 11 en verde (incluye `realtime_autorizacion.sql`: un retirado no entra al canal). `administradores_panel_autorizacion.sql` falla **desde antes** (ver Problemas conocidos) |
| Edge Functions | Las 10 de la rama desplegadas en staging |
| E2E real contra staging | Canje, clave marcada, autenticación por aserción, lectura RLS, código de un solo uso (`examples/probar_vinculacion.rs`) |
| Equipos reales | PC y teléfono vinculados en staging con los builds de diagnóstico, 0 errores; canal en vivo funcionando |
| Núcleo Rust | `cargo clippy --all-targets --features nube,cifrado-secreto-dispositivo` limpio; `cargo test` con esas features en verde |
| Escritorio | clippy Windows (`x86_64-pc-windows-gnu`) limpio con y sin `telemetria`; `tsc`, `eslint`, vitest 269/269 |
| Panel web | `tsc -b`, vitest 78/78, Playwright 10/10 |
| Móvil `rust-core` | clippy limpio, tests en verde |
| Android | bindings regenerados; `testDebugUnitTest` 331/331 |
| Producción | **Pendiente de aprobación del usuario** |

## Datos de prueba en staging

- Sitio "Prueba vinculación E2E" `baaf7b95-de9d-4ad9-a9e7-1517c4e66f8f`,
  con el gafete 424242.
- Dispositivo "PC prueba vinculación E2E" `e5a92524-1e2e-40f3-ab52-4cb5afab52aa`
  (vinculado a una clave temporal de la última corrida del ejemplo).
- "Celular legado E2E" `d5f6a1fd-a3e2-4412-a1e3-a20122156850` (quedó sin
  vincular al retirar el secreto).
- Códigos ya usados: `PRUEBA2345`, `PRUEBA6789`, `PRUEBAK2M3`, `PRUEBAW4X5`.

Se pueden reutilizar para otra corrida o borrar; no afectan a nadie.

## Cómo se prueba

- Núcleo: una vez `cargo build --release --manifest-path sqlite3mc-vendor-lib/Cargo.toml`,
  luego `cargo test --features nube,cifrado-secreto-dispositivo`.
- E2E contra staging (registrar antes un dispositivo en el panel, o crear
  un código para un dispositivo sin clave con el SQL de abajo):

  ```bash
  CONTROL_ACCESO_SUPABASE_URL=https://pmrytjktlyiuikxuuxpr.supabase.co \
  CONTROL_ACCESO_SUPABASE_APIKEY=sb_publishable_29DwMvfyj8Jq--LBcqxtBA_pTwWrDH4 \
  cargo run --example probar_vinculacion --features nube -- <codigo>
  ```

  ```sql
  insert into public.codigos_vinculacion (dispositivo_id, codigo_hash, creado_por, expira_en)
  values ('<dispositivo_id>', encode(sha256('<CODIGO>'::bytea), 'hex'),
          'prueba@staging', now() + interval '30 minutes');
  ```

- Tests SQL: cada archivo de `supabase/tests/` es una transacción que termina
  en `rollback`; se ejecuta entero con `execute_sql` (MCP) contra staging.
- Edge Functions: `deno check <funcion>/index.ts`. Se despliegan con el MCP de
  Supabase subiendo `index.ts` más los `../_shared/*.ts` que importe.
  `verify_jwt`: `false` en `device-auth`, `device-vincular`,
  `admin-list-devices`, `admin-create-usuario`, `admin-reset-password-usuario`
  (autorizan ellas mismas); `true` en el resto.
- Escritorio desde Linux: `rustup target add x86_64-pc-windows-gnu`,
  `apt-get install gcc-mingw-w64-x86-64` y
  `cargo clippy --target x86_64-pc-windows-gnu --all-targets` en
  `desktop/src-tauri` (el crate sólo compila para Windows). Frontend:
  `npx tsc --noEmit -p .`, `npx eslint src`, `npx vitest run` en `desktop/`.
- Móvil: `cargo test` / `cargo clippy --all-targets` en `mobile/rust-core`.
  Bindings: `cargo build` y luego
  `cargo run --features bindgen --bin uniffi-bindgen generate --library target/debug/libcontrol_acceso_mobile.so --language kotlin --out-dir <tmp>`,
  copiando `uniffi/control_acceso_mobile/control_acceso_mobile.kt` a
  `mobile/android/app/src/main/java/uniffi/control_acceso_mobile/`.
- Android: `ANDROID_HOME=/opt/android-sdk ./gradlew testDebugUnitTest --max-workers=1`
  en `mobile/android` (Maven Central a veces responde 429: reintentar).
- Panel: `npx tsc -b`, `npx vitest run`, y Playwright con el Chromium
  preinstalado (`launchOptions.executablePath: /opt/pw-browsers/chromium`).

### Qué es Deno y para qué se usó

Deno es el runtime de JavaScript/TypeScript sobre el que corren las Edge
Functions de Supabase (el código de `supabase/functions/` se ejecuta en Deno
en los servidores de Supabase). En esta sesión se instaló localmente
(`/root/.local/bin/deno`) sólo como herramienta de desarrollo: `deno check`
para verificar tipos de las funciones antes de desplegarlas y `deno run` para
calcular con `jose` la huella RFC 7638 que usa un test de Rust. No forma parte
del producto ni de las apps.

## Prueba con equipos reales (staging, con telemetría)

Los builds de prueba apuntan a staging y mandan telemetría a
`telemetria_diagnostico` (staging). Ninguno toca producción ni la app real
instalada en el equipo:

| Build | Cómo se obtiene | Aislamiento |
| --- | --- | --- |
| Android `diagnostico` | Workflow manual **Build de prueba (mobile y escritorio)**, variante `diagnostico` (APK en Artifacts) | Paquete `com.dqm27.lattis.diag`, staging |
| Escritorio diagnóstico | Mismo workflow, variante `escritorio-diagnostico` (instalador NSIS en Artifacts) | "Lattis Diagnostico" `com.dqm27.lattis.desktop.diag`, staging, base y claves propias, sin updater |

Pasos:

1. Panel web apuntado a staging (`web/.env.local`, ver
   `docs/recuperacion-sitio-staging.md`) → Dispositivos → alta de un
   dispositivo por equipo de prueba (PC y teléfono), cada uno en el sitio de
   prueba.
2. Instalar los builds de diagnóstico **de cero** y vincular: el teléfono
   escanea el QR; la PC escribe el código.
3. Probar: login, un ingreso, sincronizar, ver el cambio en vivo en el otro
   equipo. Luego, desde el panel, retirar uno: debe avisar al instante,
   dejar de recibir avisos en vivo y no volver a sincronizar.
4. Verificar en staging:

```sql
-- Estado de los dispositivos de prueba
select etiqueta, tipo, vinculado_en, last_seen_at, revoked_at,
       left(clave_huella, 8) as huella, plataforma, app_version
from public.dispositivos where sitio_id = '<sitio de prueba>' order by created_at;

-- Intentos rechazados / alertas
select ocurrido_en, tipo, dispositivo_id, detalle
from public.eventos_seguridad_dispositivos order by ocurrido_en desc limit 50;

-- Llamadas de nube por equipo (vincular, sincronizar, etc.): llamadas y errores
select version_app, datos->>'nombre' as operacion,
       sum((datos->>'llamadas')::int) as llamadas, sum((datos->>'errores')::int) as errores
from public.telemetria_diagnostico
where tipo = 'llamada_nucleo' and recibido_en > now() - interval '1 day'
group by 1, 2 order by errores desc, llamadas desc;

-- Canal en vivo: conexiones, avisos y latencia
select recibido_en, version_app, datos
from public.telemetria_diagnostico
where tipo = 'realtime' and recibido_en > now() - interval '1 day'
order by recibido_en desc limit 50;
```

Los nombres exactos de los campos de cada evento están en
`mobile/android/docs/telemetria-diagnostico.md` y
`desktop/docs/telemetria-diagnostico.md`; si alguna columna difiere, revisar
ahí antes de dar por mala una prueba.

## Pasos para producción (cuando el usuario lo apruebe)

Todo equipo pierde la conexión a la nube desde el paso 2 hasta que se
reinstala y se registra de nuevo. Lo que no se haya sincronizado antes se
pierde con la reinstalación: sincronizar cada equipo justo antes. Conviene
hacerlo en una ventana acordada.

1. Integrar la rama a `main` (PR y revisión).
2. Producción `xidaepyaljzkpbsxrqsm`: aplicar las migraciones
   `20260929200000_revocacion_efectiva_dispositivos` y
   `20260929200100_vinculacion_dispositivos_por_codigo` y
   `20260930020000_realtime_solo_dispositivos_vigentes`.
   Confirmar antes que el secret `DEVICE_SIGNING_KEY` existe (ya lo usa
   `device-auth`).
3. Desplegar las Edge Functions de la rama (con `_shared/`), respetando
   `verify_jwt`, y borrar `admin-suspend-device`.
4. Correr los tests SQL contra producción (hacen `rollback`) y revisar los
   advisors de seguridad.
5. Publicar el panel web nuevo.
6. Publicar las builds nuevas de escritorio y Android.
7. Por cada equipo: panel → Dispositivos → **Registrar** uno nuevo en su
   sitio → instalar la versión nueva de cero → canjear el código (el
   teléfono puede escanear el QR) → **Retirar** el dispositivo viejo.
8. Verificar en el panel que cada equipo aparece "Vinculado" y que no hay
   alertas inesperadas en "Intentos y alertas".

## Pendientes

- **Producción** (arriba), con aprobación del usuario y después de la
  prueba con equipos reales.
- Borrar de staging `admin-suspend-device` y `admin-crear-codigo-vinculacion`
  (dashboard de Supabase → Edge Functions).
- Repetir la prueba con equipos reales con builds de diagnóstico nuevos
  (Registrar y Retirar).
- F3 opcional de la propuesta: clave en TPM (Windows) y *key attestation*
  (Android).

## Problemas conocidos

- `supabase/tests/administradores_panel_autorizacion.sql` falla en staging
  al insertar un admin: la migración
  `20260909192636_retira_escritura_directa_de_administradores_panel` quitó a
  propósito las políticas de INSERT/DELETE y el test no se actualizó. No tiene
  relación con este cambio (anotado en `docs/pendientes.md`).
- Advisors de seguridad de staging: sólo avisos previos (`plegar_texto`
  sin `search_path`, `pg_net` en `public`, protección de contraseñas
  filtradas desactivada).
