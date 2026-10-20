-- El catálogo del sitio en un solo llamado.
--
-- Cada sincronización pedía el catálogo con seis GET (empresas,
-- contratistas, usuarios, gafetes, rutas y empresas de proveedor), lanzados
-- en ráfaga. Las consultas son baratas (<7 ms), pero la ráfaga obliga al
-- servidor a abrir varias conexiones a la base, casi siempre "frías", y por
-- eso algunas respuestas tardaban 180-210 ms. Esta función devuelve lo mismo
-- en una sola respuesta y una sola conexión.
--
-- Mismo contenido que los GET, fila por fila:
-- - Sólo lo que cambió: cada grupo filtra por su marca (`updated_at >
--   p_desde`; los gafetes por la suya, `p_desde_gafetes`). Sin marca (primer
--   sync del equipo) trae todo.
-- - Primera página de cada tabla, ordenada por `id` y con el mismo tamaño
--   que pagina el núcleo (`p_limite`, hoy 500). Si una tabla llena la página,
--   el núcleo sigue pidiendo las siguientes con los GET de siempre.
-- - `security invoker`: corre con el rol y la RLS de quien llama, igual que
--   un GET. Las tablas del sitio (gafetes, rutas, empresas de proveedor) se
--   filtran además por el `sitio_id` del token, como hacían los GET.
-- - Mismas columnas y nombres que los `select=` de los GET, así el núcleo
--   las lee con los mismos tipos.

create or replace function public.catalogo_del_sitio(
  p_desde timestamptz default null,
  p_desde_gafetes timestamptz default null,
  p_limite integer default 500
)
returns jsonb
language sql
stable
security invoker
set search_path = ''
as $$
  with parametros as (
    select
      least(greatest(coalesce(p_limite, 500), 1), 1000) as limite,
      ((select auth.jwt()) ->> 'sitio_id')::uuid as sitio_id
  )
  select pg_catalog.jsonb_build_object(
    'empresas', coalesce((
      select pg_catalog.jsonb_agg(f order by f.id) from (
        select e.id, e.nombre, e.activa, e.updated_at
          from public.empresas e
         where p_desde is null or e.updated_at > p_desde
         order by e.id
         limit (select limite from parametros)
      ) f), '[]'::jsonb),
    'contratistas', coalesce((
      select pg_catalog.jsonb_agg(f order by f.id) from (
        select c.id, c.nombre, c.identificacion, c.empresa_id, c.empresa_nombre, c.activo,
               c.tipo_ingreso, c.fecha_vencimiento_praind, c.es_personal_ruta, c.updated_at
          from public.contratistas c
         where p_desde is null or c.updated_at > p_desde
         order by c.id
         limit (select limite from parametros)
      ) f), '[]'::jsonb),
    'usuarios', coalesce((
      select pg_catalog.jsonb_agg(f order by f.id) from (
        select u.id, u.cedula, u.nombre, u.rol, u.activo, u.updated_at
          from public.usuarios u
         where p_desde is null or u.updated_at > p_desde
         order by u.id
         limit (select limite from parametros)
      ) f), '[]'::jsonb),
    'gafetes', coalesce((
      select pg_catalog.jsonb_agg(f order by f.id) from (
        select g.id, g.numero, g.tipo, g.estado, g.contratista_portador_id,
               g.contratista_portador_nombre, g.visita_portador_id, g.updated_at
          from public.gafetes g
         where g.sitio_id = (select sitio_id from parametros)
           and (p_desde_gafetes is null or g.updated_at > p_desde_gafetes)
         order by g.id
         limit (select limite from parametros)
      ) f), '[]'::jsonb),
    'rutas', coalesce((
      select pg_catalog.jsonb_agg(f order by f.id) from (
        select r.id, r.numero, r.activo, r.updated_at
          from public.rutas r
         where r.sitio_id = (select sitio_id from parametros)
           and (p_desde is null or r.updated_at > p_desde)
         order by r.id
         limit (select limite from parametros)
      ) f), '[]'::jsonb),
    'empresas_proveedor', coalesce((
      select pg_catalog.jsonb_agg(f order by f.id) from (
        select ep.id, ep.nombre, ep.activa, ep.updated_at
          from public.empresas_proveedor ep
         where ep.sitio_id = (select sitio_id from parametros)
           and (p_desde is null or ep.updated_at > p_desde)
         order by ep.id
         limit (select limite from parametros)
      ) f), '[]'::jsonb)
  );
$$;

comment on function public.catalogo_del_sitio(timestamptz, timestamptz, integer) is
  'Catálogo del sitio en un solo llamado (lo que cambió desde cada marca, primera página de cada tabla). Corre con la RLS de quien llama. Ver 20261020140000_catalogo_del_sitio.sql.';

-- Permisos explícitos (ver AGENTS.md, "Supabase"): los equipos y el rol de
-- servicio; nunca anon.
revoke all on function public.catalogo_del_sitio(timestamptz, timestamptz, integer) from public, anon;
grant execute on function public.catalogo_del_sitio(timestamptz, timestamptz, integer)
  to authenticated, service_role;
