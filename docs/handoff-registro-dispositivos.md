# Traspaso: registro de dispositivos por código + clave

> Documento vivo para retomar este trabajo en otra sesión. Rama:
> `feat/registro-dispositivos-seguro` (sale de `main`). Diseño y motivos:
> `docs/features-futuras/propuesta-registro-dispositivos.md`.
> Última actualización: 2026-09-30.

## Reglas de esta tarea (decididas por el usuario)

- **Supabase: sólo staging (sandbox)**, proyecto `pmrytjktlyiuikxuuxpr`.
  Producción (`xidaepyaljzkpbsxrqsm`) NO se toca hasta que el usuario lo
  apruebe explícitamente ("pasamos a nube").
- **Sin compatibilidad con el secreto de dispositivo.** Todos los equipos se
  reinstalan y se vinculan con código; no hay migración automática ni camino
  legado en ninguna capa.
- Commits sólo a nombre de Daniel Quintana, sin líneas de atribución.
- Nunca usar el término prohibido para el puesto de control (ver `AGENTS.md`).

## Cómo funciona (resumen)

```
Panel web ──(admin-provision-device / admin-crear-codigo-vinculacion)──► código XXXX-XXXX-XX + QR
                                                                          (15 min, un solo uso)
Equipo ── genera par EC P-256 (la privada nunca sale) ──► device-vincular { codigo, clave_publica_jwk }
          ◄── primer token (JWT 1 h con sitio_id, tipo, huella)
Equipo ── device-auth { desafio: true } ──► desafío (JWT del servidor, 120 s)
Equipo ── device-auth { asercion: JWS(desafío) firmado, kid = huella } ──► token
Postgres ── política RESTRICTIVA "solo dispositivos vigentes" en toda tabla con RLS:
            el token sólo sirve si el dispositivo no está revocado/suspendido
            y la huella del token es la de su clave vigente.
Suspender / revocar / re-vincular ──► corta al instante + broadcast `dispositivo_expulsado`
```

- **Código**: 10 caracteres de `23456789ABCDEFGHJKLMNPQRSTUVWXYZ`; se guarda
  sólo su SHA-256. QR: `brisas-acceso://vincular?codigo=XXXXXXXXXX`.
- **Huella**: RFC 7638 (SHA-256 del JWK canónico, base64url). Es el `kid` de
  la aserción y el claim `huella` del token.
- **Re-vincular**: código nuevo para el MISMO `dispositivo_id` (conserva
  sitio e historial). Al canjearlo, la clave anterior y sus tokens mueren.
  Un equipo con datos locales manda `dispositivo_esperado`: un código de
  otro dispositivo se rechaza sin gastarse.
- **Eventos de seguridad** (`eventos_seguridad_dispositivos`, visibles en el
  panel): `codigo_inexistente`, `codigo_usado`, `codigo_vencido`,
  `codigo_anulado`, `codigo_de_otro_dispositivo`, `firma_invalida`,
  `hardware_distinto`.

## Mapa de archivos

| Capa | Archivos |
| --- | --- |
| Migraciones | `supabase/migrations/20260929200000_revocacion_efectiva_dispositivos.sql` (clave_huella, `private.dispositivo_vigente()`, política restrictiva en loop, trigger de expulsión por Realtime), `20260929200100_vinculacion_dispositivos_por_codigo.sql` (quita `secret_hash`, `codigos_vinculacion`, `eventos_seguridad_dispositivos`, RPC `canjear_codigo_vinculacion`) |
| Tests SQL | `supabase/tests/dispositivos_vigentes_y_vinculacion.sql` + los `*_autorizacion.sql` (dispositivos con clave y claim `huella`) |
| Edge Functions | `supabase/functions/_shared/{http,admin,dispositivos}.ts`, `device-vincular`, `device-auth`, `admin-provision-device`, `admin-crear-codigo-vinculacion`, `admin-list-devices`, `admin-revoke-device` (+ refactor a `_shared` del resto de `admin-*`) |
| Núcleo Rust | `src/nube/firmante.rs` (trait `FirmanteDispositivo`, aserción, `FirmanteArchivo` con DPAPI), `cache_token.rs` (`vincular`, `autenticar_con_cache`, `vinculado`), `cliente.rs` (`vincular_con_codigo`, `autenticar_con_firmante`), `credenciales.rs` (sólo almacenamiento protegido), `application/nube.rs` (`vincular_dispositivo_inicial`), `application/con_nube.rs` (reciben `&CacheTokenDispositivo`) |
| Pruebas núcleo | `examples/probar_vinculacion.rs` (E2E contra staging), `tests/nube_smoke.rs` (manual, `#[ignore]`) |
| Escritorio | `desktop/src-tauri/src/{lib.rs,estado.rs,comandos/nube.rs}`; frontend `PrimerArranque.tsx`, `VincularEquipoModal.tsx`, `MenuUsuario.tsx` ("Vincular este equipo", sólo ROOT), `expulsionNube.ts`, `nubeRealtime.ts` |
| Panel web | `web/src/api/dispositivos.ts`, `pantallas/Dispositivos.tsx` (alta por sitio, Re-vincular, columna Vinculación, "Intentos y alertas"), `componentes/CodigoVinculacionEmitido.tsx` (QR con `uqr`, cuenta regresiva) |
| Móvil Rust | `mobile/rust-core/src/firmante.rs` (callback UniFFI `AlmacenClaveDispositivo`), `nube.rs` (`vincular_dispositivo_inicial`, `revincular_dispositivo`, `nube_configurada`), APIs `*_verificado` en `ingresos.rs`/`gafetes.rs`/`proveedores.rs` |
| Android | `AlmacenClaveKeystore.kt` (alias `control_acceso_identidad_dispositivo`, secp256r1), `PantallaEscanearCodigoVinculacion.kt` (CameraX + ML Kit), `PantallaPrimerArranque.kt`, `PrimerArranqueViewModel.kt`, `ExpulsionNube.kt`, `NubeRealtime.kt`; bindings regenerados en `app/src/main/java/uniffi/control_acceso_mobile/` |

Retirado: `SecretoDispositivoStore.kt`, `scripts/generar_secreto_dispositivo.mjs`,
la feature `cifrado-secreto-dispositivo-portable` y `aes-gcm`, todas las APIs
`*_con_secreto` del núcleo móvil, `autenticar_dispositivo` del núcleo,
`NubeDelDispositivo`, y el estado `secreto_legado` del panel y del escritorio.

## Estado

| Parte | Estado |
| --- | --- |
| Migraciones | Aplicadas en staging (el retiro de `secret_hash` se aplicó como delta con `execute_sql`, idéntico a los archivos) |
| Tests SQL en staging | 9 de 10 en verde. `administradores_panel_autorizacion.sql` falla **desde antes** (ver Problemas conocidos) |
| Edge Functions | Las 11 desplegadas en staging con el código de la rama |
| E2E real contra staging | 8/8: canje, clave marcada, autenticación por aserción, lectura RLS, código de un solo uso, re-vinculación con otra clave, clave anterior rechazada, token anterior sin acceso |
| Núcleo Rust | `cargo clippy --all-targets --features nube,cifrado-secreto-dispositivo` limpio; `cargo test` con esas features en verde (482 lib + integración) |
| Escritorio | clippy Windows (`x86_64-pc-windows-gnu`) limpio; `tsc`, `eslint`, vitest 271/271 |
| Panel web | `tsc -b`, vitest 78/78 (Playwright 10/10 en la pasada anterior) |
| Móvil `rust-core` | clippy limpio, 128 tests |
| Android | bindings regenerados; `testDebugUnitTest` 333/333 |
| Producción | **Pendiente de aprobación del usuario** |

## Datos de prueba en staging

- Sitio "Prueba vinculación E2E" `baaf7b95-de9d-4ad9-a9e7-1517c4e66f8f`,
  con el gafete 424242 (para que la prueba de re-vinculación tenga qué leer).
- Dispositivo "PC prueba vinculación E2E" `e5a92524-1e2e-40f3-ab52-4cb5afab52aa`
  (vinculado a una clave temporal de la última corrida del ejemplo).
- "Celular legado E2E" `d5f6a1fd-a3e2-4412-a1e3-a20122156850` (quedó sin
  vincular al retirar el secreto).
- Códigos ya usados: `PRUEBA2345`, `PRUEBA6789`, `PRUEBAK2M3`, `PRUEBAW4X5`.

Se pueden reutilizar para otra corrida o borrar; no afectan a nadie.

## Cómo se prueba

- Núcleo: una vez `cargo build --release --manifest-path sqlite3mc-vendor-lib/Cargo.toml`,
  luego `cargo test --features nube,cifrado-secreto-dispositivo`.
- E2E contra staging (crear antes dos códigos del mismo dispositivo, ver SQL
  abajo):

  ```bash
  CONTROL_ACCESO_SUPABASE_URL=https://pmrytjktlyiuikxuuxpr.supabase.co \
  CONTROL_ACCESO_SUPABASE_APIKEY=sb_publishable_29DwMvfyj8Jq--LBcqxtBA_pTwWrDH4 \
  cargo run --example probar_vinculacion --features nube -- <codigo> <codigo-revinculacion>
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

## Pasos para producción (cuando el usuario lo apruebe)

Todo equipo pierde la conexión a la nube desde el paso 1 hasta que se
re-vincula (el trabajo local sigue funcionando y la bandeja de salida se
conserva). Conviene hacerlo en una ventana acordada.

1. Integrar la rama a `main` (PR y revisión).
2. Producción `xidaepyaljzkpbsxrqsm`: aplicar las migraciones
   `20260929200000_revocacion_efectiva_dispositivos` y
   `20260929200100_vinculacion_dispositivos_por_codigo`.
   Confirmar antes que el secret `DEVICE_SIGNING_KEY` existe (ya lo usa
   `device-auth`).
3. Desplegar las 11 Edge Functions de la tabla de arriba (con `_shared/`),
   respetando `verify_jwt`. `admin-crear-codigo-vinculacion` es nueva.
4. Correr los tests SQL contra producción (hacen `rollback`) y revisar los
   advisors de seguridad.
5. Publicar el panel web nuevo.
6. Publicar las builds nuevas de escritorio y Android.
7. Por cada equipo: panel → Dispositivos → **Re-vincular** → instalar la
   versión nueva → canjear el código (el teléfono puede escanear el QR).
8. Verificar en el panel que cada equipo aparece "Vinculado" y que no hay
   alertas inesperadas en "Intentos y alertas".

## Pendientes

- **Producción** (arriba), con aprobación del usuario.
- **Android: UI para re-vincular** un teléfono ya en uso. El núcleo ya expone
  `Nucleo::revincular_dispositivo` (sólo ROOT); falta la pantalla.
- **Reconstruir sin código** un equipo que perdió la base pero conserva su
  clave (hoy la pantalla inicial siempre pide código; ver
  `docs/recuperacion-sitio-local.md`).
- **Realtime**: la autorización de `realtime.messages` mira sólo `sitio_id`.
  Un equipo revocado con un token todavía vigente (máx. 1 h) podría
  suscribirse al canal del sitio. Evaluar sumar `dispositivo_vigente()` a
  esa política; el aviso `dispositivo_expulsado` no se pierde, porque el
  equipo ya está suscrito cuando se lo expulsa.
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
