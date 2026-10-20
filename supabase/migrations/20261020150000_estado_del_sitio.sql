-- Lo que está adentro en el sitio, y las salidas que otro equipo dio sobre lo
-- de este, en un solo llamado.
--
-- Mismo motivo que `catalogo_del_sitio`: cada sincronización hacía hasta ocho
-- GET en ráfaga (contratistas, proveedores, correo y préstamos KOF), cada uno
-- barato en la base pero abriendo conexiones frías. Visitas queda afuera a
-- propósito: no está en producción y sigue con sus GET.
--
-- Por cada grupo pedido (`p_grupos`) devuelve, con las mismas columnas y
-- filtros que los GET de antes:
-- - `cierres_<grupo>` (`devoluciones_prestamos`): de los ids que este equipo
--   tiene abiertos (`p_<grupo>`), los que la nube ya tiene cerrados. Sin ids,
--   no va (el equipo no tiene nada abierto de esa clase).
-- - `<grupo>_abiertos`: todo lo que sigue adentro en el sitio, la lista chica
--   de "quién está adentro ahora" que reemplaza la caché local; entre ciclos
--   la mantiene al día Realtime.
-- - `conflictos_<grupo>`: de las cédulas que este equipo tiene adentro
--   (`p_cedulas_<grupo>`), las que además están adentro en otra unidad (o
--   abiertas por el otro equipo de esta). Llama a las funciones que ya tienen
--   esa regla (`contratistas_activos_en_otras_unidades`,
--   `proveedores_activos_en_otras_unidades`, `correos_activos_en_otras_unidades`).
--   El núcleo filtra la respuesta contra lo que sigue abierto al terminar la
--   sincronización, como antes.
-- Un grupo que no se pidió no aparece (un aviso en vivo pide sólo lo suyo).
--
-- `security invoker`: corre con la RLS de quien llama, igual que un GET; lo
-- abierto se filtra además por el `sitio_id` del token, como hacían los GET.

create or replace function public.estado_del_sitio(
  p_grupos text[],
  p_ingresos text[] default null,
  p_proveedor text[] default null,
  p_correo text[] default null,
  p_prestamos text[] default null,
  p_cedulas_ingresos text[] default null,
  p_cedulas_proveedor text[] default null,
  p_cedulas_correo text[] default null
)
returns jsonb
language sql
stable
security invoker
set search_path = ''
as $$
  with parametros as (
    select ((select auth.jwt()) ->> 'sitio_id')::uuid as sitio_id
  )
  select
    -- Contratistas.
    case when 'ingresos' = any(p_grupos) and pg_catalog.cardinality(p_ingresos) > 0 then
      pg_catalog.jsonb_build_object('cierres_ingresos', coalesce((select pg_catalog.jsonb_agg(f) from (
        select i.id, i.hora_salida, i.usuario_salida_nombre
          from public.ingresos i
         where i.id = any(p_ingresos::uuid[]) and i.hora_salida is not null
      ) f), '[]'::jsonb)) else '{}'::jsonb end
    || case when 'ingresos' = any(p_grupos) then
      pg_catalog.jsonb_build_object('ingresos_abiertos', coalesce((select pg_catalog.jsonb_agg(f) from (
        select i.id, i.contratista_nombre, i.hora_entrada, i.usuario_entrada_nombre,
               i.dispositivo_entrada_id, i.contratista_cedula, i.empresa_nombre, i.tipo_ingreso,
               i.medio_ingreso, i.gafete_numero, i.placa
          from public.ingresos i
         where i.sitio_id = (select sitio_id from parametros) and i.hora_salida is null
      ) f), '[]'::jsonb)) else '{}'::jsonb end
    -- Proveedores.
    || case when 'proveedor' = any(p_grupos) and pg_catalog.cardinality(p_proveedor) > 0 then
      pg_catalog.jsonb_build_object('cierres_proveedor', coalesce((select pg_catalog.jsonb_agg(f) from (
        select p.id, p.hora_salida, p.usuario_salida_nombre
          from public.ingresos_proveedor p
         where p.id = any(p_proveedor::uuid[]) and p.hora_salida is not null
      ) f), '[]'::jsonb)) else '{}'::jsonb end
    || case when 'proveedor' = any(p_grupos) then
      pg_catalog.jsonb_build_object('proveedor_abiertos', coalesce((select pg_catalog.jsonb_agg(f) from (
        select p.id, p.cedula, p.nombre, p.empresa_nombre, p.placa, p.gafete_numero,
               p.hora_entrada, p.usuario_entrada_nombre, p.dispositivo_entrada_id
          from public.ingresos_proveedor p
         where p.sitio_id = (select sitio_id from parametros) and p.hora_salida is null
      ) f), '[]'::jsonb)) else '{}'::jsonb end
    -- Visitas por correo.
    || case when 'correo' = any(p_grupos) and pg_catalog.cardinality(p_correo) > 0 then
      pg_catalog.jsonb_build_object('cierres_correo', coalesce((select pg_catalog.jsonb_agg(f) from (
        select c.id, c.hora_salida, c.usuario_salida_nombre
          from public.ingresos_correo c
         where c.id = any(p_correo::uuid[]) and c.hora_salida is not null
      ) f), '[]'::jsonb)) else '{}'::jsonb end
    || case when 'correo' = any(p_grupos) then
      pg_catalog.jsonb_build_object('correo_abiertos', coalesce((select pg_catalog.jsonb_agg(f) from (
        select c.id, c.cedula, c.nombre, c.motivo, c.placa, c.gafete_numero,
               c.hora_entrada, c.usuario_entrada_nombre, c.dispositivo_entrada_id
          from public.ingresos_correo c
         where c.sitio_id = (select sitio_id from parametros) and c.hora_salida is null
      ) f), '[]'::jsonb)) else '{}'::jsonb end
    -- Préstamos de gafete provisional (KOF).
    || case when 'prestamos' = any(p_grupos) and pg_catalog.cardinality(p_prestamos) > 0 then
      pg_catalog.jsonb_build_object('devoluciones_prestamos', coalesce((select pg_catalog.jsonb_agg(f) from (
        select g.id, g.hora_devolucion, g.usuario_devolucion_nombre
          from public.prestamos_gafete_provisional g
         where g.id = any(p_prestamos::uuid[]) and g.hora_devolucion is not null
      ) f), '[]'::jsonb)) else '{}'::jsonb end
    || case when 'prestamos' = any(p_grupos) then
      pg_catalog.jsonb_build_object('prestamos_abiertos', coalesce((select pg_catalog.jsonb_agg(f) from (
        select g.id, g.encargado_nombre, g.encargado_codigo_empleado, g.gafete_numero,
               g.hora_entrega, g.usuario_entrega_nombre
          from public.prestamos_gafete_provisional g
         where g.sitio_id = (select sitio_id from parametros) and g.hora_devolucion is null
      ) f), '[]'::jsonb)) else '{}'::jsonb end
    -- Conflictos: adentro también en otra unidad.
    || case when 'ingresos' = any(p_grupos) and pg_catalog.cardinality(p_cedulas_ingresos) > 0 then
      pg_catalog.jsonb_build_object('conflictos_ingresos', coalesce((select pg_catalog.jsonb_agg(f)
        from public.contratistas_activos_en_otras_unidades(p_cedulas_ingresos) f), '[]'::jsonb))
      else '{}'::jsonb end
    || case when 'proveedor' = any(p_grupos) and pg_catalog.cardinality(p_cedulas_proveedor) > 0 then
      pg_catalog.jsonb_build_object('conflictos_proveedor', coalesce((select pg_catalog.jsonb_agg(f)
        from public.proveedores_activos_en_otras_unidades(p_cedulas_proveedor) f), '[]'::jsonb))
      else '{}'::jsonb end
    || case when 'correo' = any(p_grupos) and pg_catalog.cardinality(p_cedulas_correo) > 0 then
      pg_catalog.jsonb_build_object('conflictos_correo', coalesce((select pg_catalog.jsonb_agg(f)
        from public.correos_activos_en_otras_unidades(p_cedulas_correo) f), '[]'::jsonb))
      else '{}'::jsonb end;
$$;

comment on function public.estado_del_sitio(text[], text[], text[], text[], text[], text[], text[], text[]) is
  'Lo abierto del sitio y las salidas dadas por otro equipo sobre lo de este, en un solo llamado. Corre con la RLS de quien llama. Ver 20261020150000_estado_del_sitio.sql.';

-- Permisos explícitos (ver AGENTS.md, "Supabase").
revoke all on function public.estado_del_sitio(text[], text[], text[], text[], text[], text[], text[], text[]) from public, anon;
grant execute on function public.estado_del_sitio(text[], text[], text[], text[], text[], text[], text[], text[])
  to authenticated, service_role;
