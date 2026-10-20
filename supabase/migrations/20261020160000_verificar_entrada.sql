-- La verificación final de una entrada en un solo llamado.
--
-- Al confirmar un ingreso, el equipo hacía hasta tres consultas en paralelo:
-- ¿la persona está adentro por esta vía en alguna unidad?, ¿está adentro por
-- otra vía?, y ¿el gafete lo tiene el otro equipo del sitio? Esta función
-- responde las tres juntas.
--
-- No repite ninguna regla: llama a las funciones que ya las tienen
-- (`ingreso_activo_de_contratista`, `ingreso_proveedor_activo`,
-- `ingreso_correo_activo`, `prestamo_provisional_activo_de_encargado` y
-- `persona_adentro_por_otra_via`), y el gafete se consulta igual que el GET
-- de antes: abierto en el sitio del token, por otro equipo, con ese número
-- (para el correo, también en las visitas: comparten el gafete de visita).
-- `security invoker`: el gafete pasa por la RLS de quien llama; las funciones
-- de arriba siguen siendo `security definer`, como ya eran.
--
-- `p_via`: CONTRATISTA, PROVEEDOR, POR_CORREO o KOF (para KOF, `p_cedula` es
-- el código de empleado del encargado y no hay "otra vía").
-- Devuelve `{ "misma_via": {sitio_id, sitio_nombre} | null,
--             "otra_via": {via, sitio_id, sitio_nombre} | null,
--             "gafete_ocupado": bool }`.

create or replace function public.verificar_entrada(
  p_via text,
  p_cedula text,
  p_gafete bigint default null
)
returns jsonb
language sql
stable
security invoker
set search_path = ''
as $$
  with parametros as (
    select ((select auth.jwt()) ->> 'sitio_id')::uuid as sitio_id,
           ((select auth.jwt()) ->> 'sub')::uuid as dispositivo_id
  )
  select pg_catalog.jsonb_build_object(
    'misma_via', (
      select pg_catalog.to_jsonb(f) from (
        select a.sitio_id, a.sitio_nombre from public.ingreso_activo_de_contratista(p_cedula) a
         where p_via = 'CONTRATISTA'
        union all
        select a.sitio_id, a.sitio_nombre from public.ingreso_proveedor_activo(p_cedula) a
         where p_via = 'PROVEEDOR'
        union all
        select a.sitio_id, a.sitio_nombre from public.ingreso_correo_activo(p_cedula) a
         where p_via = 'POR_CORREO'
        union all
        select a.sitio_id, a.sitio_nombre from public.prestamo_provisional_activo_de_encargado(p_cedula) a
         where p_via = 'KOF'
        limit 1
      ) f),
    'otra_via', (
      select pg_catalog.to_jsonb(f) from (
        select o.via, o.sitio_id, o.sitio_nombre
          from public.persona_adentro_por_otra_via(p_cedula, p_via) o
         where p_via in ('CONTRATISTA', 'PROVEEDOR', 'POR_CORREO')
         limit 1
      ) f),
    'gafete_ocupado', p_gafete is not null and (
      case p_via
        when 'CONTRATISTA' then exists (
          select 1 from public.ingresos i, parametros p
           where i.sitio_id = p.sitio_id and i.dispositivo_entrada_id <> p.dispositivo_id
             and i.hora_salida is null and i.gafete_numero = p_gafete)
        when 'PROVEEDOR' then exists (
          select 1 from public.ingresos_proveedor i, parametros p
           where i.sitio_id = p.sitio_id and i.dispositivo_entrada_id <> p.dispositivo_id
             and i.hora_salida is null and i.gafete_numero = p_gafete)
        -- El gafete de visita lo comparten el correo y las visitas.
        when 'POR_CORREO' then exists (
          select 1 from public.ingresos_correo i, parametros p
           where i.sitio_id = p.sitio_id and i.dispositivo_entrada_id <> p.dispositivo_id
             and i.hora_salida is null and i.gafete_numero = p_gafete)
          or exists (
          select 1 from public.movimientos_visita m, parametros p
           where m.sitio_id = p.sitio_id and m.dispositivo_entrada_id <> p.dispositivo_id
             and m.hora_salida is null and m.gafete_numero = p_gafete)
        when 'KOF' then exists (
          select 1 from public.prestamos_gafete_provisional g, parametros p
           where g.sitio_id = p.sitio_id and g.dispositivo_entrega_id <> p.dispositivo_id
             and g.hora_devolucion is null and g.gafete_numero = p_gafete)
        else false
      end)
  );
$$;

comment on function public.verificar_entrada(text, text, bigint) is
  'Verificación final de una entrada en un solo llamado: misma vía, otra vía y gafete en uso por el otro equipo. Reutiliza las funciones de cada regla. Ver 20261020160000_verificar_entrada.sql.';

-- Permisos explícitos (ver AGENTS.md, "Supabase").
revoke all on function public.verificar_entrada(text, text, bigint) from public, anon;
grant execute on function public.verificar_entrada(text, text, bigint) to authenticated, service_role;
