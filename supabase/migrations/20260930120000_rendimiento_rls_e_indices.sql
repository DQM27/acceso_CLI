-- Rendimiento del panel y de la sincronización, a partir del análisis de
-- producción del 2026-09-30 (estadísticas de uso de tablas e índices y
-- `pg_stat_statements`). No cambia qué puede ver o escribir nadie.
--
-- 1. RLS: `private.es_admin_global()` se evaluaba una vez POR FILA en cada
--    política que la usaba sin envolver. En producción eso dejó 227.000
--    lecturas secuenciales de `administradores_panel`, la tabla más
--    consultada de todas, y la vista `panel_movimientos` la dispara tres
--    veces por fila (ingresos, sitios, dispositivos). Envuelta en
--    `(select ...)`, Postgres la calcula una sola vez por consulta (initPlan),
--    igual que ya se hace con `auth.jwt()` y `private.dispositivo_vigente()`.
--    Se reescriben todas las políticas de `public` que la usan, así también
--    las que se agreguen en ramas que lleguen antes que esta migración.
--
-- 2. Sincronización incremental: los equipos piden
--    `updated_at > marca` (y `sitio_id = ?` en las tablas por sitio) unas
--    10.000 veces al día por tabla. `ingresos`, `ingresos_proveedor` y
--    `prestamos_gafete_provisional` ya tienen su índice
--    (`indices_historial_y_sincronizacion`); faltaban los catálogos. En
--    producción `contratistas` sumaba 5,1 millones de filas leídas por
--    recorridos completos para 415 filas reales.
--
-- 3. Contratistas en el panel: ordena por `nombre` (la consulta más cara del
--    panel, 20 ms de media).
--
-- 4. Cédula única en forma normalizada: `panel_crear_contratista` revisaba
--    la repetición con un recorrido completo y sin protección ante dos altas
--    simultáneas con distinto formato (`1-1234-0567` y `112340567`). El
--    índice único sobre `normalizar_cedula(identificacion)` cierra esa
--    carrera y convierte la revisión en una búsqueda por índice. Las
--    identificaciones que la función rechaza dan `null` y no chocan entre sí.
--    En producción y en staging no hay repetidas al normalizar.
--
-- 5. Índices redundantes: un índice sólo por `sitio_id` sobra cuando otro
--    índice de la misma tabla empieza por `sitio_id` (también cubre la
--    clave foránea). Cada índice de más cuesta en cada escritura.
--
-- Fuera de esta migración, a propósito: la unicidad de "gafete en uso" en
-- proveedores y KOF. La cola de salida de los equipos sólo sabe resolver ese
-- choque en `ingresos` (`es_conflicto_gafete_activo`); en las otras tablas
-- la fila quedaría reintentando hasta fallar. Primero hay que preparar las
-- apps.

-- 1. RLS -------------------------------------------------------------------

do $$
declare
  politica record;
  -- Sólo las llamadas que no están ya dentro de un `SELECT`: así la
  -- migración se puede volver a correr sin anidar subconsultas.
  patron constant text := '(?<!SELECT )private\.es_admin_global\(\)';
  reemplazo constant text := '(select private.es_admin_global())';
begin
  for politica in
    select schemaname, tablename, policyname, qual, with_check
    from pg_policies
    where schemaname = 'public'
      and (coalesce(qual, '') || coalesce(with_check, '')) ~ patron
  loop
    execute format(
      'alter policy %I on %I.%I%s%s',
      politica.policyname,
      politica.schemaname,
      politica.tablename,
      case when politica.qual is null then ''
           else format(' using (%s)', regexp_replace(politica.qual, patron, reemplazo, 'g')) end,
      case when politica.with_check is null then ''
           else format(' with check (%s)', regexp_replace(politica.with_check, patron, reemplazo, 'g')) end
    );
  end loop;
end $$;

-- 2. Sincronización incremental de los catálogos ---------------------------

-- Catálogos globales: se piden sólo por `updated_at`.
create index if not exists contratistas_updated_at_idx on public.contratistas (updated_at);
create index if not exists empresas_updated_at_idx on public.empresas (updated_at);
-- Por sitio.
create index if not exists gafetes_sitio_updated_at_idx on public.gafetes (sitio_id, updated_at);
create index if not exists empresas_proveedor_sitio_updated_at_idx
  on public.empresas_proveedor (sitio_id, updated_at);

-- 3. Orden del panel ---------------------------------------------------------

create index if not exists contratistas_nombre_idx on public.contratistas (nombre);

-- 4. Cédula única normalizada -----------------------------------------------

create unique index if not exists contratistas_cedula_normalizada_key
  on public.contratistas (public.normalizar_cedula(identificacion));

-- 5. Índices redundantes ---------------------------------------------------

-- Cubierto por `gafetes_sitio_id_numero_tipo_key (sitio_id, numero, tipo)`.
drop index if exists public.gafetes_sitio_idx;
-- Cubiertos por `(sitio_id, hora_entrada desc)` y `(sitio_id, updated_at)`.
drop index if exists public.ingresos_sitio_idx;
drop index if exists public.ingresos_proveedor_sitio_idx;
-- Cubierto por `(sitio_id, hora_entrega desc)` y `(sitio_id, updated_at)`.
drop index if exists public.idx_prestamos_gafete_provisional_sitio;
