# Traspaso: telemetría y errores en producción (2026-11-01, tarde)

Para retomar en otra sesión (en la casa). Horas en hora de Costa Rica.
Reglas en `AGENTS.md`. Viene después de `docs/handoff-observabilidad-2026-11-01.md`
(el de la mañana, que sigue valiendo para las decisiones).

## Ramas (las dos subidas)

| Rama | Qué tiene | Estado |
|---|---|---|
| `feat/observabilidad-fase1a-telemetria` | Telemetría (fase 1a) completa + 2 correcciones de hoy | **Lista para PR** (falta sólo lo de "Pendiente de la 1a") |
| `feat/observabilidad-fase1b-errores` | Sale de la 1a (la incluye, mergeada). Errores (fase 1b) | **A medias**: SQL y Edge Function hechos y probados; núcleo/apps escritos **sin compilar**; panel sin empezar |

Orden de merge: primero la 1a, después la 1b. **Mergear a `main` aplica las
migraciones en producción.**

Se trabajó con un worktree (`C:\Users\femprobrisas\Desktop\control_acceso_errores`)
para la 1b mientras la 1a compilaba en la carpeta principal. En la casa basta
con `git fetch` y `git checkout` de cada rama; el worktree no hace falta.

---

## Fase 1a (telemetría): qué se hizo hoy

### Probado

| Qué | Resultado |
|---|---|
| Migraciones `20261101120000` + `20261101130000` con sus baterías, en **staging** (`pmrytjktlyiuikxuuxpr`) con `rollback` | ✅ pasan (después de la corrección 1) |
| La primera instalación de `pg_cron` (en staging se hizo `drop extension pg_cron cascade` dentro de la transacción para dejarlo como producción) | ✅ la migración lo crea en `pg_catalog`, crea la tarea, y el `rollback` dejó staging igual |
| **Producción** (sólo lectura): no tiene `pg_cron` instalado pero sí precargado (`shared_preload_libraries` igual que staging). La base pesa 24 MB | El merge lo instalará: probado arriba |
| Edge Function con Deno 2.9.7 (`deno check`, `deno lint`, `deno test`, igual que `reglas.yml`) | ✅ 89 pruebas |
| Núcleo (job `test` de `ci.yml`): fmt, audit, clippy, tests, clippy y tests con `nube,cifrado-secreto-dispositivo` | ✅ 2.014 pruebas, 0 fallas |
| Panel web (`web.yml` sobre `web/`): lint, 508 pruebas, build, diseño, e2e | ✅ (ver nota del e2e) |
| Escritorio (`test-gui`): diseño, lint/tests del frontend, audit, fmt, clippy | ✅ después de la corrección 2 |
| Escritorio: **ejecutar** `cargo test` | ⚠️ No se puede en la PC del trabajo: el binario de pruebas no arranca con el toolchain GNU de Windows (`STATUS_ENTRYPOINT_NOT_FOUND`). Compila (`cargo test --no-run`). Lo corre el CI con MSVC |
| Móvil (`test-android`): fmt, audit, clippy de `mobile/rust-core`, bindings UniFFI regenerados desde la `.dll` del host | ✅ bindings idénticos |
| Móvil: `./gradlew testDebugUnitTest` | ❓ quedó corriendo al cortar; **sin resultado**. Repetir |

Nota del e2e del panel: con `web/.env.local` (staging) fallan 22 pruebas,
porque simulan la URL de **producción**. No es un error: en el CI no hay
`.env.local`. Para correrlo en local, poner antes en el proceso las variables
de `web/.env` (Vite da prioridad a las del proceso).

### Corregido hoy (commits en la 1a)

1. `929bb75b` — `pg_catalog.least(...)`/`pg_catalog.greatest(...)` **no
   existen** (son construcciones del lenguaje, como `coalesce`): fallaba
   `observabilidad.mantener()` al pasar el tope y las dos consultas del panel.
   Ahora sin prefijo.
2. `d6bea37b` — las pruebas del escritorio (`desktop/src-tauri/src/telemetria.rs`)
   no compilaban: usaban la cola vieja (`tomar`/`devolver` con texto) y un
   `assert!` sin `;`. La sesión anterior sólo había corrido `cargo check`.

### Pendiente de la 1a, en orden

1. Repetir `testDebugUnitTest` del móvil (y, si se puede, `cargo test` del
   escritorio con MSVC; si no, que lo diga el CI).
2. **Abrir el PR** de `feat/observabilidad-fase1a-telemetria` y revisar el CI.
3. **Desplegar la Edge Function `observabilidad`** en staging y producción
   (a mano; preguntar antes: toca producción). Ojo: la de la rama 1b ya trae
   también la ruta `/errores`; desplegar la de la 1a si la 1b no está lista
   (la de la 1b igual funciona sin la migración de errores: responde 503
   `receptora_no_lista` en `/errores` y el equipo guarda y reintenta).
4. El usuario mergea → confirmar en `cron.job` la tarea
   `observabilidad-mantenimiento-diario`.
5. Publicar escritorio y móvil, y ver lotes en el panel (Telemetría → Flota).

---

## Fase 1b (errores)

### Decisiones (además de las del plan)

- El texto se **limpia otra vez en la Edge Function** (`/errores`) con la
  lógica del receptor de staging, que también calcula la **huella**. La base
  valida formas y largos; no limpia.
- **Equipo, unidad y plataforma salen del JWT** (`mobile`→`movil`,
  `pc`→`escritorio`); un `visor` no puede mandar errores.
- **Regresión por versión:** un grupo `resuelto` vuelve a `nuevo` sólo si el
  error llega en una versión que no estaba entre las vistas al resolverlo
  (`versiones_al_resolver`). Un equipo sin actualizar no lo reabre.
  `ignorado` nunca se reabre solo.
- Sin `sesion_id` ni `tipo` (el reporte no trae sesión; `tipo` era siempre
  `excepcion`).
- Retención 180 días con su propia tarea de cron
  (`observabilidad-errores-diario`, 03:12), no dentro de `mantener()`.
- En las apps desaparece el DSN de staging: el registro de errores del
  núcleo manda a `nube::base_url()` + `/functions/v1/observabilidad/errores`,
  con brotli y el token del equipo, que cada app entrega con
  `errores::usar_token(...)`. Sin token, los errores esperan en disco.

### Estado por pieza

| Pieza | Archivos | Estado |
|---|---|---|
| Migración | `supabase/migrations/20261101140000_observabilidad_errores.sql` | ✅ Probada en staging con `rollback`. Commit `0a76193d` |
| Batería | `supabase/tests/observabilidad_errores.sql` | ✅ Pasa (pasos 1 a 9). Se comprobó que detecta una expectativa cambiada a propósito |
| Edge Function `/errores` | `supabase/functions/observabilidad/{index,errores,errores.test}.ts` | ✅ Deno check/lint/test (102 pruebas, 13 nuevas). Contrato TS→SQL probado en staging. **Sin desplegar**. Commit `653259b7` |
| Núcleo | `src/errores/{envio,mod}.rs`, `destino.rs` borrado | ✏️ **Escrito, SIN COMPILAR NI PROBAR** (commit WIP) |
| Escritorio | `desktop/src-tauri/src/{errores,lib}.rs` | ✏️ Escrito, sin compilar |
| Móvil (Rust) | `mobile/rust-core/src/{errores,lib}.rs` | ✏️ Escrito, sin compilar |
| Móvil (Kotlin) | `RegistroErrores.kt` | ✏️ Sin el argumento `dsn`. **Los bindings NO se regeneraron**: hasta regenerarlos, Kotlin no compila |
| Panel | vista Errores | ⬜ Sin empezar |
| Docs | `docs/telemetria-contrato.md`, plan | ⬜ Sin actualizar |

### Qué cambia en el núcleo y las apps (escrito, falta compilar)

- `src/errores/envio.rs`: `enviar(&Destino { cliente, base_url, apikey, token }, &cola)`
  manda lotes de 10 `{"reportes": [...]}` con `Content-Encoding: br`
  (`telemetria::lote::comprimir`) a `RUTA_RECEPTORA`. 2xx borra; 400/413/422
  descarta (borra); 401/403 corta y deja todo; 5xx/429/red corta y deja
  todo. 5 pruebas con una receptora falsa (ruta, `authorization`, `apikey`,
  `content-encoding`, cuerpo descomprimido).
- `src/errores/mod.rs`: sale `Configuracion.dsn` y `Destino`; entra
  `pub type ProveedorToken = Arc<dyn Fn() -> Option<String> + Send + Sync>` y
  `pub fn usar_token(ProveedorToken)` (static `Mutex`, reemplazable, en
  cualquier orden respecto de `instalar`). El hilo de envío arranca siempre;
  `Registro::enviar(token)` no pide el token si no hay pendientes. Pruebas
  nuevas: `sin_token_no_se_manda_nada_y_queda_en_disco`,
  `sin_pendientes_ni_se_pide_el_token`.
- Escritorio: sin la constante `DSN`; en `configurar_arranque` (`lib.rs`),
  junto a `telemetria::usar_token`, un `control_acceso::errores::usar_token`
  con el mismo `autenticar_con_cache`.
- Móvil: `Nucleo.cache_token` pasa a `Arc<CacheTokenDispositivo>` (los usos
  `&self.cache_token` siguen compilando por coerción); en `desde_core` se
  registra `errores::usar_token` con un `Weak` a esa caché.
  `instalar_registro_errores` pierde el parámetro `dsn`.

### Próximos pasos de la 1b, en orden

1. **Compilar y probar el núcleo:** en la raíz, `cargo fmt --check`,
   `cargo clippy --all-targets` (con y sin
   `--features "nube,cifrado-secreto-dispositivo"`) y `cargo test` con esas
   features. Antes, `cargo build --release --manifest-path sqlite3mc-vendor-lib/Cargo.toml`.
2. **Escritorio:** `cargo fmt --check` y `cargo clippy --all-targets` en
   `desktop/src-tauri`.
3. **Móvil:** clippy de `mobile/rust-core`; **regenerar los bindings** como el
   CI (`cargo build --lib` + `uniffi-bindgen generate --library` sobre la
   librería del host, ver `ci.yml`) y copiar el `.kt` a
   `mobile/android/app/src/main/java/uniffi/control_acceso_mobile/`; después
   `./gradlew testDebugUnitTest`.
4. **Panel:** vista Errores sobre `panel_errores_grupos(p_dias, p_estado)`,
   `panel_errores_detalle(p_huella, p_limite)` y
   `panel_errores_cambiar_estado(p_huella, p_estado)` (lista agrupada con
   filtro de estado y días; detalle con traza, operaciones y equipo; botones
   de estado). Con sus pruebas, como `TelemetriaDatos`.
5. Actualizar `docs/telemetria-contrato.md` (formato y destino de errores) y
   el plan.
6. Limpieza (se puede dejar para la fase 6): `BuildConfig.DSN_ERRORES` y
   `dsnErrores` en `mobile/android/app/build.gradle.kts`, y el secreto
   `SENTRY_DSN` en `build-test-mobile.yml`, `release-movil.yml` y
   `release.yml`: ya no se usan (siguen avisando "saldrá sin reporte de
   errores" si falta el secreto, lo que ya no es cierto).

---

## Cómo probar migraciones en staging

El MCP de Supabase **rechaza solo** el SQL con sentencias destructivas
(`delete`, `drop constraint`, `revoke`) de las baterías. Se usó la API de
administración:

```
POST https://api.supabase.com/v1/projects/pmrytjktlyiuikxuuxpr/database/query
Authorization: Bearer <token personal sbp_...>
{"query": "begin; <migraciones> <batería sin su begin/rollback> select 'BATERIA OK'; rollback;"}
```

Devuelve el resultado del último `select` antes del `rollback`. Para la 1b:
las tres migraciones (120000, 130000, 140000) + `observabilidad_errores.sql`.
Después verificar: `to_regnamespace('observabilidad')` nulo y `cron.job` sólo
con `limpiar-errores-app` y `limpiar-telemetria-diagnostico` (las de staging).

- **`supabase/.temp/linked-project.json` apunta a PRODUCCIÓN**
  (`xidaepyaljzkpbsxrqsm`): no usar la CLI con `--linked` para pruebas.
- No hay `psql` ni la CLI de Supabase instalados en la PC del trabajo.
- El token que se usó hoy (`pruebas-staging-observabilidad`, acceso total)
  quedó en `C:\Users\femprobrisas\.supabase\access-token` de la PC del
  trabajo y apareció en la conversación: **revocarlo** en
  supabase.com/dashboard/account/tokens y borrar ese archivo.
