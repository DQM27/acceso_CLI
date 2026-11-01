# Traspaso: telemetría y errores a producción (2026-11-01)

> **Actualización (2026-11-01, tarde):** el estado al día (qué se probó, las
> dos correcciones de esta rama y lo que falta) está en
> `docs/handoff-observabilidad-errores.md`, en la rama
> `feat/observabilidad-fase1b-errores`. Resumen para esta rama: migraciones
> probadas en staging (también la primera instalación de `pg_cron`), Deno,
> núcleo, panel y escritorio en verde; corregidos `least`/`greatest` con
> prefijo y las pruebas del escritorio que no compilaban. Falta repetir
> `testDebugUnitTest` del móvil, abrir el PR, desplegar la Edge Function y
> mergear.

Para retomar en la sesión de la PC sin el contexto de la anterior. Horas en hora
de Costa Rica. Reglas del proyecto en `AGENTS.md` (español, commits sólo a nombre
de Daniel Quintana, sin atribuciones, el término prohibido del puesto de control).

## Qué se decidió (no reabrir)

- La telemetría y los errores pasan de **staging a producción**, a un esquema
  `observabilidad` que la API no expone. Escriben los equipos con **su propio
  token**: la nube pone el equipo y la unidad.
- **Transporte:** lote **columnar** + **brotli 11** + **HTTP/2**, por una Edge
  Function sin privilegios (`observabilidad`), porque la puerta de entrada de
  Supabase no descomprime cuerpos. Medido: un lote real de 200 filas pasa de
  75 KB a ~3 KB. Sin pérdida (ida y vuelta probada) y sin duplicados (`lote_id`).
- **Móvil "a caballo":** la telemetría sale sólo cuando la radio ya está
  despierta por otra petición del núcleo (ventana de 5 s), sola tras 15 min o
  con 400 eventos. Escritorio: cada 60 s.
- **Retención:** 30 días de detalle, resumen diario 1 año, errores 180 días,
  tope de 600.000 filas de detalle (producción está en el plan gratuito de
  Supabase, 500 MB; hoy usa 24 MB). Todo en `observabilidad.config`.
- `pg_cron` en `pg_catalog` (no en `public`, por el linter). Funciones nuevas sin
  sumar avisos `security definer`: la real en `observabilidad`, un envoltorio
  `security invoker` en `public`.
- **Sin diagnóstico remoto ni modo mantenimiento** (puertas que nadie vigila).
- **Grafana se retira** después de pasar las alertas a correo (Resend).

Planes: `docs/features-futuras/plan-observabilidad-en-produccion.md`,
`plan-red-movil-por-el-nucleo.md`, `plan-verificacion-cruzada-mas-rapida.md`.

## Ramas (todas subidas, ninguna con PR)

| Rama | Sobre `main` | Contenido |
|---|---|---|
| `fix/telemetria-cola-fallidos` | 4 commits | `cola_fallidos` en el catálogo (se perdía desde la 1.8.12); purga de conflictos de `cola_salida` de más de 7 días; versión y metadata del equipo en **cada** renovación del token (el panel mostraba la del día de la vinculación); preferencia de hora de Costa Rica en `AGENTS.md`. Independiente de las otras. |
| `docs/plan-observabilidad-produccion` | 7 commits | Los tres planes. Ya incluida en la rama siguiente. |
| `feat/observabilidad-fase1a-telemetria` | 15 commits | Todo lo de la telemetría (abajo). Parte de la rama de docs. |

Sugerido: un PR para `fix/telemetria-cola-fallidos` y otro para
`feat/observabilidad-fase1a-telemetria` (que trae los planes).

## Qué tiene `feat/observabilidad-fase1a-telemetria`

| Pieza | Archivos | Estado |
|---|---|---|
| Migración telemetría | `supabase/migrations/20261101120000_observabilidad_telemetria.sql`, batería `supabase/tests/observabilidad_telemetria.sql` | Escrita. **Sin probar en staging** (la llamada se cortó esperando la aprobación del servidor de Supabase). |
| Migración panel | `supabase/migrations/20261101130000_observabilidad_panel.sql`, batería `supabase/tests/observabilidad_panel.sql` | Escrita. **Sin probar en staging.** |
| Núcleo | `src/telemetria/{lote,cola,envio,disparador}.rs`; `nube::cliente_http` público; `CacheTokenDispositivo` marca la actividad de red | Hecho. `cargo test` 1.198 pasan, clippy limpio con y sin `nube`. |
| Edge Function | `supabase/functions/observabilidad/` (+ `config.toml`: `verify_jwt = false`) | Escrita. Lógica verificada con Node (29 comprobaciones). **Falta `deno check/lint/test` (CI `reglas.yml`) y desplegar.** |
| Escritorio | `desktop/src-tauri/src/telemetria.rs`, `lib.rs` | Hecho. `cargo check` en Linux con GTK simulado: sin errores nuevos. Falta el CI en Windows. |
| Móvil | `mobile/rust-core/src/telemetria_nube.rs`, `Telemetria.kt`, bindings | Hecho. `testDebugUnitTest` 483 pasan, bindings idénticos a los del CI. |
| Panel | `web/src/pantallas/TelemetriaDatos.tsx`, `api/telemetriaDatos.ts`, vistas Flota y Rendimiento en `Telemetria.tsx` | Hecho. lint, 500 pruebas, build, diseño. |

**No publicar el escritorio ni el móvil de esta rama antes de aplicar las
migraciones en producción y desplegar la Edge Function**: la telemetría quedaría
guardada en los equipos (hasta 8 MB) sin llegar a ningún lado.

## Próximos pasos, en orden

1. **Probar las dos migraciones en staging** (`pmrytjktlyiuikxuuxpr`), cada una
   con su batería, dentro de una transacción que termina en `rollback`:
   `begin;` + el SQL de la migración + el cuerpo de la batería (sin su propio
   `begin;`) + `rollback;`. La del panel necesita la de telemetría antes, en la
   misma transacción. Al aprobar la pregunta del servidor de Supabase, confirmar
   que es staging. Si una prueba falla, el mensaje dice cuál (`'4: ...'`).
   Verificar después que no quedó nada (`select to_regnamespace('observabilidad')`
   debe dar nulo). Nunca en producción (`xidaepyaljzkpbsxrqsm`).
2. **CI:** abrir el PR y revisar `reglas.yml` (Deno), `ci.yml` (núcleo, bindings,
   `testDebugUnitTest`, Windows) y `web.yml`.
3. **Desplegar la Edge Function `observabilidad`** en producción y en staging
   (las funciones se suben a mano, no las despliega la integración con GitHub).
   Probar con un token de equipo real: sin migración responde 503
   (`receptora_no_lista`), con ella 200.
4. **Mergear a `main`** (aplica las migraciones en producción) y confirmar en
   `cron.job` la tarea `observabilidad-mantenimiento-diario` (03:07 de Costa Rica).
5. **Publicar** el escritorio y el móvil, y ver en el panel (Telemetría → Flota)
   que llegan lotes.

## Lo que falta después

- **Errores en producción (fase 1b):** tablas `observabilidad.errores` y
  `errores_grupo` (con regresión: un error resuelto que vuelve pasa a `nuevo`),
  ruta `/errores` en la Edge Function (limpiar texto de nuevo, como
  `supabase/functions-staging/receptor-errores/logica.ts`), el registro de
  errores del núcleo (`src/errores/`) mandando con el token del equipo en vez de
  la clave del DSN, y vista Errores en el panel. Hoy los errores siguen a staging.
- **Alertas por Resend (fase 5):** el dueño ya tiene Resend configurado; los
  eventos se definen con él. Base propuesta: error nuevo o regresión, filas de
  la cola con error real, equipo sin conectarse X horas en horario de operación,
  equipo nuevo en vinculación, y `mantenimiento_registro.accion = 'tope'`.
  `pg_net` ya está instalado.
- **Retiro (fase 6):** Grafana, el receptor de staging y las tablas viejas,
  cuando ninguna versión en uso mande allá.
- **Tareas aparte (documentadas):** push y actualizaciones por el núcleo
  (manifiesto con `ETag`/304, descarga con `Range`/`If-Range`, regla de caché de
  Cloudflare para `/android/*.apk` en R2); verificación cruzada por adelantado
  (hoy `registrar_ingreso` ~276 ms de p50 en el móvil); las 24 advertencias
  `security definer` del linter; mover `pg_net` fuera de `public`; actualizar
  `docs/telemetria-contrato.md` al esquema 3 (cabecera de sesión, columnar).

## Datos útiles

- Producción ya hecho a mano esta sesión: se revocó el dispositivo "Visor web
  historial" (no lo usa el panel).
- El móvil todavía no expone el commit en `BuildConfig`; la sesión lo manda vacío
  (la nube lo acepta).
- Los equipos actuales: Celular Brisas (1.8.13) y PC Brisas (1.10.7) en uso;
  Cartago Prueba sin uso.
- Cómo preparar Android, el escritorio y Deno en una sesión en la nube: `AGENTS.md`,
  "Entorno de pruebas en sesiones en la nube".
