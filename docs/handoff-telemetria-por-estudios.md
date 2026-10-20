# Traspaso: telemetría por estudios y registro de errores

Estado al 2026-10-20. Sirve para retomar en una sesión nueva sin el contexto de
la anterior. El diseño completo está en
`docs/features-futuras/plan-telemetria-y-registro-de-errores.md` (rama
`docs/plan-telemetria-y-errores`, aún sin mezclar a `main`).

## Qué se quiere

Datos reales de producción para mejorar la app, sin muestreo, con **el mismo
código en todos los builds** (sin flags ni variantes de compilación).

- **Errores:** siempre encendidos y fuera del panel. Captura propia (sin el SDK
  de Sentry) hacia el receptor propio `receptor-errores`.
- **Telemetría por estudios:** siete grupos (`base`, `rendimiento`, `fluidez`,
  `sincronizacion`, `recursos`, `escaneo`, `depuracion`). `base` va siempre;
  `rendimiento` va por defecto; el resto se enciende **por equipo desde un
  panel web**, con vencimiento. La orden viaja panel → nube → equipo, el equipo
  la guarda en su SQLite y el núcleo la tiene en memoria.

## Ramas (todas subidas; ninguna con PR, a pedido del dueño)

| Rama | Contenido | Estado |
|---|---|---|
| `docs/plan-telemetria-y-errores` | El plan (reemplaza a `plan-herramienta-diagnostico.md` y `plan-telemetria-ajustable.md`, que quedan como historia) | Hecha |
| `fix/errores-sin-dsn-de-respaldo` (`548e2cc`) | Fase 0: quita el DSN de respaldo a sentry.io en escritorio y móvil, pasa el secreto en los 3 workflows con aviso si falta | Hecha en código. **Falta que el dueño confirme que el secreto `SENTRY_DSN` de GitHub apunta al receptor propio** y probar un error real de cada plataforma |
| `feat/telemetria-ordenes-nube` (`0ffeb8d`) | Fase 1: migración `20261020120000_telemetria_ordenes_por_unidad.sql` + batería `supabase/tests/telemetria_ordenes.sql` | Hecha. Probada en staging dentro de una transacción con `ROLLBACK`. **No aplicada** a staging ni a producción |
| `feat/telemetria-config-unidad` (`783e136`, `663666a`, `0061159`) | Fase 2 (parcial): núcleo, nube + escritorio, móvil | Ver abajo |

## Las 7 fases

| # | Fase | Estado |
|---|---|---|
| 0 | Errores al receptor | Código hecho; falta confirmar el secreto |
| 1 | Orden en la nube | Hecha |
| 2 | Orden en la unidad | **En curso** |
| 3 | Panel (página "Telemetría" para el administrador) | Pendiente |
| 4 | Captura propia de errores (la más grande) | Pendiente |
| 5 | Ajustes de rendimiento (commit en la versión del móvil, histogramas, red vs local, HTTP/2) | Pendiente |
| 6 | Recortes y contrato | Pendiente |

## Fase 2: lo hecho

- **Núcleo** (`783e136`): migración 59 del esquema local (tabla `config_telemetria`,
  una fila), `src/telemetria/{grupos,config}.rs` (grupos, catálogo evento→grupo,
  estado atómico con vencimiento y versión), `src/database/config_telemetria.rs`
  (guardar/leer/`cargar_en`).
- **Nube y escritorio** (`663666a`): `src/nube/sincronizacion/telemetria.rs`
  (`aplicar_aviso_telemetria`, `recibir_orden_telemetria_propia`), enganchado al
  final de la sincronización completa; comando Tauri `aplicar_orden_telemetria`;
  `desktop/src/ordenTelemetria.ts`, `api/nube.ts` y el handler en `nubeRealtime.ts`.
- **Móvil** (`0061159`): `Nucleo.aplicar_orden_telemetria`, bindings Kotlin
  regenerados, `OrdenTelemetriaNube.kt` y el handler en `NubeRealtime.kt`.

## Fase 2: lo que falta

1. **Cargar la orden guardada al abrir el núcleo** (`database::config_telemetria`
   ya tiene `cargar_en` con comprobación de equipo; al abrir aún no se conoce el
   `dispositivo_id`, así que probablemente haga falta una variante que cargue la
   guardada sin comprobar y deje que la primera sincronización la corrija).
2. **Compuerta por grupo:** cada punto de medición pregunta
   `telemetria::config().permite(grupo_de_evento(tipo), ahora_unix)`. Retirar la
   feature de Cargo `telemetria`, `BuildConfig.TELEMETRIA`/`TELEMETRIA_LIGERA`,
   `-PtelemetriaEnProduccion`, la variable `TELEMETRIA_EN_PRODUCCION`, la variante
   `diagnostico`, `telemetria_reglas` y las reglas remotas del escritorio
   (`telemetria.rs`: `Reglas`, `revisar_reglas`, `SIEMPRE_ENCENDIDA`). Archivos:
   `desktop/src-tauri/src/telemetria.rs`, `desktop/src/telemetria.ts`,
   `mobile/android/.../Telemetria.kt`, `mobile/android/app/build.gradle.kts`,
   workflows `release*.yml` y `build-test-mobile.yml`.
3. **Confirmación:** que el equipo informe la versión aplicada en el evento `base`.

**Decisión pendiente del dueño:** retirar los flags hace que todos los equipos
manden `base` y `rendimiento` en cuanto se publique. Es lo que se quiere, pero
conviene avisar al cliente antes de publicar.

## Decisiones tomadas (no reabrir)

- Captura propia de errores, de una sola vez; **no** se parcha el SDK de Sentry.
  El receptor se reutiliza y acepta ambos formatos mientras queden versiones
  viejas.
- Orden **por equipo** (`dispositivos.id`), no por sitio. Escribe sólo el
  administrador del panel (`es_admin_global`; los administradores se dan de alta
  por SQL). Cada equipo lee sólo su fila. Sin DELETE.
- Sin fila de orden = `base` + `rendimiento`. Al vencer una orden, el equipo
  vuelve a ese estado aunque no tenga red.
- No convertir todavía en crate: el módulo `telemetria` se mantiene sin
  dependencias del resto para poder extraerlo después.

## Cómo probar en este entorno

- **Núcleo Rust:** primero `cargo build --release --manifest-path
  sqlite3mc-vendor-lib/Cargo.toml` (una vez). Después, con las features del CI:
  `cargo test --features "nube,cifrado-secreto-dispositivo"` y
  `cargo clippy --all-targets --features "nube,cifrado-secreto-dispositivo"`.
  **Sin `--features nube` no corre nada de `src/nube/`.**
- **Núcleo móvil:** `cd mobile/rust-core && cargo test`. Los bindings Kotlin se
  regeneran con `cargo build --lib && cargo run --features bindgen --bin
  uniffi-bindgen -- generate --library target/debug/libcontrol_acceso_mobile.so
  --language kotlin --no-format --out-dir <dir>`; el CI exige que no haya diff.
- **JS del escritorio:** `cd desktop && npm ci && npx vitest run`, `npx tsc
  --noEmit`, `npx eslint <archivos>`.
- **SQL de Supabase:** probar en **staging** (`pmrytjktlyiuikxuuxpr`) con
  `execute_sql` dentro de `begin; ... rollback;`. Nunca en producción
  (`xidaepyaljzkpbsxrqsm`). Verificar después que no quedó nada.
- **Lo que NO se puede verificar aquí:** compilar la parte Tauri del escritorio
  (faltan las librerías GTK) ni Kotlin/Android (no hay SDK). Queda para el CI y
  para pruebas en equipos reales; decirlo siempre en el mensaje del commit.

## Datos útiles

- Grafana: la fuente PostgreSQL de staging es `eg1fe86yip534f`. Alertas en la
  carpeta `lattis-alertas`: "Error nuevo en las apps" y "Equipo nuevo en la
  pantalla de vinculación"; contacto `correo-daniel`.
- Hallazgo que originó todo: los 8 pánicos del escritorio del 18-oct
  (`tokio .../blocking/shutdown.rs:51`) y 3 `ReferenceError` de JS no llegaron a
  `errores_app` (sólo tiene 3 mensajes de prueba), porque un build sin el secreto
  `SENTRY_DSN` caía al DSN de sentry.io.
- No se pudo concluir si el HTTP/2 mejoró algo en el móvil (sin diferencia clara)
  y en escritorio faltan muestras de `preparar_ingreso`/`registrar_ingreso`; el
  plan (fase 5) agrega commit en la versión, histogramas e indicador de red/HTTP2.
- El motivo de salida `DESCONOCIDO_16` del móvil corresponde a
  `REASON_PACKAGE_UPDATED` (inofensivo) y se agrega con nombre en la fase 6.

## Reglas del proyecto (`AGENTS.md`)

Español siempre. **Nunca** usar el término prohibido para el puesto de control
(decir "puesto de control", "portería" o "punto de acceso"). Todos los commits a
nombre de `Daniel Quintana <daniel.bleach1@gmail.com>` (autor y committer), sin
`Co-Authored-By`, `Claude-Session` ni otra atribución, tampoco en PR. No abrir PR
sin que lo pidan. Un cambio completo = un commit bien documentado, subido.
Verificar antes de afirmar; decir lo que no se pudo comprobar.
