-- Lienzo del asistente: la hora y el día de la semana de cada ingreso, y cuánto
-- tiempo estuvo adentro. Con eso el panel arma "horas pico" (día de la semana ×
-- hora) sin pasar por una consulta libre, y los análisis pueden usar la
-- permanencia.
--
-- `lienzo_ingresos` devuelve tres columnas más:
-- - `hora`: hora de entrada en Costa Rica (0 a 23).
-- - `dia_semana`: "Lunes" … "Domingo", del día de entrada en Costa Rica.
-- - `horas_adentro`: horas entre la entrada y la salida, con un decimal; vacío
--   si la persona sigue adentro.
--
-- Agregar columnas a lo que devuelve una función obliga a borrarla y crearla de
-- nuevo (`create or replace` no cambia el tipo de retorno). Mismos parámetros,
-- misma comprobación de administrador y mismos permisos explícitos que en
-- `20261022120000_lienzo_catalogo`; nada para `anon`.

drop function if exists public.lienzo_ingresos(date, date, text[], text[], text, text[], text, text);

create function public.lienzo_ingresos(
  p_desde date,
  p_hasta date,
  p_empresas text[] default null,
  p_empresas_excluir text[] default null,
  p_contiene_empresa text default null,
  p_tipos_ingreso text[] default null,
  p_sitio text default null,
  p_contiene_nombre text default null
)
returns table (
  dia date,
  nombre text,
  empresa text,
  tipo_ingreso text,
  sitio text,
  hora integer,
  dia_semana text,
  horas_adentro double precision
)
language plpgsql
stable
security invoker
set search_path = ''
as $$
begin
  if not exists (select 1 from public.administradores_panel a where a.correo = (select auth.email())) then
    raise exception 'Sólo los administradores del panel pueden usar el lienzo'
      using errcode = '42501';
  end if;
  if p_desde is null or p_hasta is null or p_hasta < p_desde or p_hasta - p_desde > 92 then
    raise exception 'Rango de fechas inválido (máximo 92 días)' using errcode = '22023';
  end if;

  return query
  select (i.hora_entrada at time zone 'America/Costa_Rica')::date,
         i.contratista_nombre::text,
         i.empresa_nombre::text,
         i.tipo_ingreso::text,
         s.nombre::text,
         extract(hour from (i.hora_entrada at time zone 'America/Costa_Rica'))::integer,
         (array['Lunes', 'Martes', 'Miércoles', 'Jueves', 'Viernes', 'Sábado', 'Domingo'])[
           extract(isodow from (i.hora_entrada at time zone 'America/Costa_Rica'))::integer
         ],
         case
           when i.hora_salida is null then null
           else round((extract(epoch from (i.hora_salida - i.hora_entrada)) / 3600)::numeric, 1)::double precision
         end
    from public.ingresos i
    left join public.sitios s on s.id = i.sitio_id
   where i.hora_entrada >= (p_desde::timestamp at time zone 'America/Costa_Rica')
     and i.hora_entrada < ((p_hasta + 1)::timestamp at time zone 'America/Costa_Rica')
     and (p_empresas is null or i.empresa_nombre = any(p_empresas))
     and (p_empresas_excluir is null or i.empresa_nombre is null
          or i.empresa_nombre <> all(p_empresas_excluir))
     and (p_contiene_empresa is null
          or i.empresa_nombre ilike '%' || p_contiene_empresa || '%')
     and (p_tipos_ingreso is null or i.tipo_ingreso = any(p_tipos_ingreso))
     and (p_sitio is null or s.nombre = p_sitio)
     and (p_contiene_nombre is null or i.contratista_nombre ilike '%' || p_contiene_nombre || '%')
   order by i.hora_entrada desc
   limit 20000;
end;
$$;

revoke all on function public.lienzo_ingresos(date, date, text[], text[], text, text[], text, text) from public, anon;
grant execute on function public.lienzo_ingresos(date, date, text[], text[], text, text[], text, text) to authenticated, service_role;
