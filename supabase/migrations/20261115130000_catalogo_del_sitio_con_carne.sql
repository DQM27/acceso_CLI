-- El catálogo que reciben los equipos trae también `praind_foto` (carnés
-- PRAIND, ver 20261115120000_carnes_praind). Sin esto, la primera página del
-- catálogo (que llega por esta función) no traía la columna aunque el GET de
-- respaldo sí la pidiera. Las apps anteriores ignoran el campo nuevo.
-- Resto, tal como estaba (`20261031120000_empresas_proveedor_revision_y_fusion`).
create or replace function public.catalogo_del_sitio(
  p_desde timestamptz default null,
  p_desde_gafetes timestamptz default null,
  p_limite integer default 500
)
returns jsonb
language sql
stable
set search_path to ''
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
               c.tipo_ingreso, c.fecha_vencimiento_praind, c.es_personal_ruta, c.praind_foto, c.updated_at
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
        select ep.id, ep.nombre, ep.activa, ep.updated_at, ep.fusionada_en
          from public.empresas_proveedor ep
         where ep.sitio_id = (select sitio_id from parametros)
           and (p_desde is null or ep.updated_at > p_desde)
         order by ep.id
         limit (select limite from parametros)
      ) f), '[]'::jsonb)
  );
$$;
