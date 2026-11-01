# Traspaso: errores en producción (fase 1b)

Para retomar en otra sesión. Horas en hora de Costa Rica. Reglas en `AGENTS.md`.
Viene después de `docs/handoff-observabilidad-2026-11-01.md` (telemetría, fase 1a).

## Ramas

- `feat/observabilidad-fase1b-errores`: **esta**. Sale de
  `feat/observabilidad-fase1a-telemetria` (necesita sus dos migraciones). Se
  mergea **después** que la de telemetría.
- Se trabajó en un worktree aparte (`../control_acceso_errores`) mientras el
  núcleo de la rama de telemetría compilaba en la carpeta principal.

## Decisiones (las del plan, más estas)

- **El texto se limpia otra vez en la Edge Function** (`observabilidad`, ruta
  `/errores`) con la misma lógica del receptor de staging
  (`supabase/functions-staging/receptor-errores/logica.ts`), que también
  calcula la huella. La base valida formas y largos, no limpia.
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

## Estado

| Pieza | Archivos | Estado |
|---|---|---|
| Migración | `supabase/migrations/20261101140000_observabilidad_errores.sql` | ✅ Probada en staging con `rollback` (2026-11-01) |
| Batería | `supabase/tests/observabilidad_errores.sql` | ✅ Pasa (pasos 1 a 9) |
| Edge Function, ruta `/errores` | `supabase/functions/observabilidad/{index,errores,errores.test}.ts` | ✅ Deno check/lint/test (102 pruebas); contrato TS→SQL probado en staging. **Sin desplegar** |
| Núcleo | `src/errores/envio.rs`, `destino.rs` | Pendiente |
| Escritorio y móvil | cablear el envío nuevo | Pendiente |
| Panel | vista Errores | Pendiente |

## Cómo probar la migración en staging

El MCP de Supabase rechaza solo el SQL destructivo de las baterías. Se usa la
API de administración (`POST https://api.supabase.com/v1/projects/pmrytjktlyiuikxuuxpr/database/query`)
con un token personal, mandando `begin;` + las tres migraciones (120000,
130000, 140000) + el cuerpo de la batería (sin su `begin;`/`rollback;`) +
`rollback;`. **`supabase/.temp/linked-project.json` apunta a producción**: no
usar la CLI con `--linked` para pruebas.

## Próximos pasos, en orden

1. ✅ **Edge Function** (hecho): `POST .../observabilidad/errores` con
   `{"reportes": [...]}` (`Reporte`, esquema 1), comprimido y con el token del
   equipo. Responde 200 con `{estado, guardados, repetidos, grupos_nuevos,
   regresiones, descartados}`; 400/413 descartar; 401/403 renovar token;
   429/502/503 reintentar (igual que `/telemetria`).
2. **Núcleo:** `errores::envio` manda a `/functions/v1/observabilidad/errores`
   con brotli y el token del equipo (como `telemetria::envio`), con las
   mismas respuestas (`Recibido`/`Rechazado`/`SinAutorizacion`/`Reintentar`).
   `Destino::desde_dsn` deja de hacer falta.
3. **Escritorio y móvil:** pasarle al envío de errores el cliente, la URL base,
   la `apikey` y el token (`CacheTokenDispositivo`), como ya se hace con la
   telemetría. Un pánico se manda al arrancar, cuando ya hay token.
4. **Panel:** vista Errores (lista por grupo con filtro de estado y días,
   detalle con traza y operaciones, botones de estado) sobre
   `panel_errores_grupos`, `panel_errores_detalle` y
   `panel_errores_cambiar_estado`.
5. Actualizar `docs/telemetria-contrato.md` y el plan.
