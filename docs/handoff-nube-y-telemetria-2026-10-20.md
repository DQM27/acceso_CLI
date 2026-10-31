# Traspaso: nube optimizada y telemetría (2026-10-20, noche)

Para retomar en una sesión nueva sin el contexto de la anterior.

## Reglas del dueño (obligatorias)

- Español siempre. Nunca el término prohibido para el puesto de control
  (decir "puesto de control", "portería" o "punto de acceso").
- Commits sólo a nombre de `Daniel Quintana <daniel.bleach1@gmail.com>`, sin
  `Co-Authored-By` ni ninguna otra atribución (tampoco en PR).
- **PR sólo cuando el dueño diga "vamos con un PR".** Mientras tanto, commits
  locales bien documentados y sin push. Nunca mergear ni taguear.
- Texto con tildes nunca por heredoc, `python -c` ni `git commit -m` desde la
  terminal (se guarda en cp1252): escribir el archivo con la herramienta de
  edición y usar `git commit -F archivo`.
- Mergear a `main` aplica `supabase/migrations/` en **producción**
  (`xidaepyaljzkpbsxrqsm`). Probar siempre antes en **staging**
  (`pmrytjktlyiuikxuuxpr`), que es el sandbox: se puede romper y rearmar.
- Ver `AGENTS.md`, sección "Supabase": GRANT explícitos en cada migración,
  nunca a `anon`, y cada migración con su batería en `supabase/tests/`.

## Dónde quedó cada cosa

| Qué | Dónde | Estado |
|---|---|---|
| Permisos explícitos, sin `anon` | #197 | Mergeado, en producción |
| Telemetría fase 1 (`telemetria_ordenes`) | #196 | Mergeado, en producción |
| Sincronización en pocos llamados: `catalogo_del_sitio`, `estado_del_sitio` (incluye conflictos por unidad), `verificar_entrada`, gzip | #199 | Mergeado, en producción (las tres funciones verificadas allí) |
| Rutas sin `vehiculos_ruta`, Brotli, precalentamiento con `OPTIONS` (sin 401), log del 401 del reloj, conexiones vivas en escritorio | #203, rama `perf/conexion-y-limpieza` | **Abierto**, esperando merge del dueño. Sin migraciones. Su CI falló por avisos de seguridad publicados ese día (`rustls-webpki`, `webpki-roots`); arreglado con `cargo update -p` en los tres `Cargo.lock` (commit `fix(deps)`, también en la rama del lote) |
| #203 + telemetría fase 2 (8 commits cherry-pickeados de `feat/telemetria-config-unidad`) + este traspaso | rama local `lote/telemetria-y-limpieza` | **Local, sin push.** Es la base del PR del final del día |

Verificado en `lote/telemetria-y-limpieza`: `cargo test --features
"nube,cifrado-secreto-dispositivo"` (671 del núcleo, 46 suites), clippy sin
avisos, `mobile/rust-core` `cargo test` (183), `desktop` `npx vitest run`
(436) y `npx tsc --noEmit`, `cargo check` del escritorio. Sin verificar:
escritorio y Android en un equipo real.

Cuando el dueño mergee #203: `git rebase --onto origin/main
perf/conexion-y-limpieza lote/telemetria-y-limpieza` deja sólo los commits
nuevos (telemetría + traspaso) encima de `main`.

## Lo que sigue (plan del dueño: un PR al final del día con versión nueva)

1. **Hecho (2026-10-20):** la página ya está en el lote, con buscador de
   equipos, y se vio contra el sandbox. Las versiones 1.9.0 (escritorio) y
   1.6.0 (móvil) salen sólo con lo de #203 (PR #204); la telemetría va en una
   versión posterior. Lo que se pedía:
   **Telemetría fase 3: página "Telemetría" en el panel web** (`web/`). El
   administrador enciende y apaga estudios (grupos `rendimiento`, `fluidez`,
   `sincronizacion`, `recursos`, `escaneo`, `depuracion`) por equipo, con
   vencimiento y nota, sobre `telemetria_ordenes` (RLS ya permite escribir
   sólo al admin global; el historial está en `telemetria_ordenes_historial`).
   El aviso a los equipos lo manda el trigger por Realtime; no hay que tocar
   las apps. Diseño completo: `docs/handoff-telemetria-por-estudios.md` y
   `docs/features-futuras/plan-telemetria-y-registro-de-errores.md` (rama
   `docs/plan-telemetria-y-errores`).
2. **Borrar `salidas_ruta` y `vehiculos_ruta`** (migración nueva con su
   batería; borrar también sus funciones de trigger huérfanas). **Sólo cuando
   todos los equipos tengan instalada una versión que ya no pide
   `vehiculos_ruta` (la de #203).** Si la versión de hoy es la primera en
   dejar de pedirla, el borrado espera a la siguiente: las apps viejas
   fallarían con 404 al sincronizar. Las rutas de empuje (`cola.rs`) sólo
   mandan si alguien crea filas, y la UI de rutas está oculta.
3. **Subir la versión de escritorio y de móvil**, cada una a su propio
   siguiente número (el tag de release no es la versión interna).
4. **Del dueño:** confirmar que el secreto `SENTRY_DSN` de GitHub apunta al
   receptor propio (telemetría fase 0) y avisar al cliente que con la fase 2
   todos los equipos mandan `base` y `rendimiento` al instalar.
5. Después (no hoy): telemetría fases 4 (captura propia de errores, la más
   grande), 5 y 6.

## Pendientes chicos de la nube

- Dos interruptores del dashboard de producción, del dueño: cerrar el
  registro abierto de Auth (`disable_signup` está en `false`) y encender la
  protección de contraseñas filtradas.
- 401 `PGRST301` en `hora_servidor_ms` (JWT que la nube no valida): no
  afecta los registros (el reloj sigue corregido con el header `Date`), pero
  falta la causa. Con #203 la app deja el motivo exacto en el log; leerlo en
  un equipo con esa versión antes de tocar nada.
- Diferido a propósito: cron de salud de la flota (umbral acordado: 12 h sin
  conexión; canal del aviso sin definir), renombre de `encargados_ruta` a
  `personal_kof`, regla en el servidor contra registros con la hora muy
  corrida (decisión de negocio).

## Trampas conocidas

- Las baterías SQL que crean auxiliares en `pg_temp` necesitan
  `grant execute on all functions in schema pg_temp to authenticated;`
  (desde los permisos explícitos ninguna función nueva es ejecutable por
  `public`).
- Para probar una migración en staging con su batería: poner la migración
  justo después del `begin;` de la batería y dejar el `rollback;` final.
- En la sesión del 2026-10-20 la app rechazó varias veces los permisos de
  `apply_migration`/`execute_sql`; desde VS Code funcionó. Si vuelve a pasar,
  pegar el SQL en el SQL Editor del dashboard de staging.
- `cargo fmt` reformatea todo el workspace: revisar `git status` después.
- Clippy del proyecto es estricto (`too_many_lines`, `struct_excessive_bools`):
  partir funciones largas en ayudantes en vez de silenciar.
