-- Búsqueda del historial con índice, pensada para cuando se conecten las
-- demás unidades operativas (mucho más volumen en `ingresos`).
--
-- Medido en staging con 150.000 ingresos de 10 unidades (carga sintética en
-- una transacción revertida): la búsqueda por texto tardaba 3,6 s porque
-- `panel_movimientos` calculaba `plegar_texto(...)` (sin tildes ni
-- mayúsculas) en CADA fila del período en cada consulta; con 400.000 filas
-- superaba el minuto. El conteo exacto para paginar tardaba 1,7-5,3 s: eso se
-- resuelve en el panel dejando de pedirlo (ver `api/historial.ts`).
--
-- El buscador del historial busca sólo dos cosas, cada una con el índice que
-- le corresponde (empresa, placa y lo demás quedan para los filtros de
-- columna, que ya van acotados por fecha y unidad):
--
-- * Cédula: siempre se teclea desde el inicio, así que basta un B-tree por
--   prefijo (`like '1056%'`). La base usa intercalación `en_US` (ICU), con la
--   que un B-tree normal NO sirve para `like`: por eso `text_pattern_ops`.
--   Reemplaza a `ingresos_cedula_hora_entrada_idx`, que cubría la igualdad
--   exacta: este también la cubre, así que no hacen falta los dos.
-- * Nombre: se busca por partes y en cualquier orden ("perez", "carlos
--   perez"), lo que sólo resuelve un índice de trigramas (`pg_trgm`) sobre el
--   nombre ya plegado. El nombre plegado se calcula una sola vez al escribir
--   la fila (columna generada); los equipos no la mandan ni la leen.

create extension if not exists pg_trgm with schema extensions;

alter table public.ingresos
  add column contratista_nombre_plegado text generated always as (
    public.plegar_texto(coalesce(contratista_nombre, ''))
  ) stored;

create index if not exists ingresos_nombre_plegado_trgm_idx
  on public.ingresos using gin (contratista_nombre_plegado extensions.gin_trgm_ops);

drop index if exists public.ingresos_cedula_hora_entrada_idx;
create index if not exists ingresos_cedula_patron_idx
  on public.ingresos (contratista_cedula text_pattern_ops, hora_entrada desc);

-- La vista cambia de forma (sale `texto_busqueda`, que mezclaba cédula,
-- nombre, empresa y placa): hay que recrearla y volver a dar el permiso.
-- `nombre_p` pasa a leer la columna guardada, así el filtro de columna por
-- nombre también usa el índice de trigramas.
drop view if exists public.panel_movimientos;

create view public.panel_movimientos
with (security_invoker = true) as
select
  i.id,
  i.sitio_id,
  s.nombre as sitio_nombre,
  i.contratista_cedula,
  i.contratista_nombre,
  i.empresa_nombre,
  i.tipo_ingreso,
  i.medio_ingreso,
  i.gafete_numero,
  i.hora_entrada,
  i.hora_salida,
  i.usuario_entrada_nombre,
  i.usuario_salida_nombre,
  d.tipo as dispositivo_entrada_tipo,
  i.placa,
  coalesce(replace(i.tipo_ingreso, '_', ' '), '—') as tipo_texto,
  case
    when i.medio_ingreso = 'VEHICULO' and nullif(btrim(i.placa), '') is not null then btrim(i.placa)
    when i.medio_ingreso = 'VEHICULO' then 'VEHÍCULO'
    when i.medio_ingreso = 'CAMINANDO' then 'CAMINANDO'
    else '—'
  end as medio_texto,
  to_char(i.hora_entrada at time zone 'America/Costa_Rica', 'HH24:MI') as hora_entrada_txt,
  coalesce(to_char(i.hora_salida at time zone 'America/Costa_Rica', 'HH24:MI'), 'Activo') as hora_salida_txt,
  public.plegar_texto(coalesce(s.nombre, '')) as unidad_p,
  public.plegar_texto(coalesce(i.contratista_cedula, '')) as cedula_p,
  i.contratista_nombre_plegado as nombre_p,
  public.plegar_texto(coalesce(i.empresa_nombre, '')) as empresa_p,
  public.plegar_texto(coalesce(replace(i.tipo_ingreso, '_', ' '), '—')) as tipo_p,
  public.plegar_texto(
    case
      when i.medio_ingreso = 'VEHICULO' and nullif(btrim(i.placa), '') is not null then btrim(i.placa)
      when i.medio_ingreso = 'VEHICULO' then 'VEHÍCULO'
      when i.medio_ingreso = 'CAMINANDO' then 'CAMINANDO'
      else '—'
    end
  ) as medio_p,
  public.plegar_texto(coalesce(i.usuario_entrada_nombre, '')) as usuario_entrada_p,
  public.plegar_texto(coalesce(i.usuario_salida_nombre, '')) as usuario_salida_p
from public.ingresos i
left join public.sitios s on s.id = i.sitio_id
left join public.dispositivos d on d.id = i.dispositivo_entrada_id;

revoke all on public.panel_movimientos from anon, authenticated;
grant select on public.panel_movimientos to authenticated;

-- Buscador del historial: cédula o nombre, cada uno con su índice.
--
-- Por qué una función y no un filtro sobre la vista: bajo RLS, Postgres sólo
-- usa como condición de índice los operadores marcados `leakproof`, y
-- `like`/`ilike` no lo son (podrían filtrar información de filas ajenas por
-- sus errores o su tiempo). Consultando la vista, la búsqueda recorría todas
-- las filas del período aunque los índices existieran. Esta función corre
-- como dueño de las tablas (sin RLS), así que los índices sí se usan; por eso
-- valida ella misma el permiso: sólo un administrador global del panel
-- (el mismo criterio con el que la vista le muestra todas las unidades).
--
-- * Si el texto trae dígitos es una cédula (ningún nombre guardado tiene
--   dígitos): se normaliza igual que al guardar y se busca por prefijo con
--   `ingresos_cedula_patron_idx`. Las cédulas guardadas nunca empiezan con 0.
-- * Si no, es un nombre: cada palabra (sin tildes ni mayúsculas, en cualquier
--   orden) debe aparecer en el nombre, con `ingresos_nombre_plegado_trgm_idx`.
--
-- Fechas y unidades se aplican adentro para no traer coincidencias de fuera
-- del período; orden, paginación y filtros de columna los aplica PostgREST
-- sobre el resultado, igual que con la vista.
create or replace function public.panel_buscar_movimientos(
  p_busqueda text,
  p_desde timestamptz default null,
  p_hasta timestamptz default null,
  p_sitio_ids uuid[] default null
)
returns setof public.panel_movimientos
language plpgsql
stable
security definer
set search_path = ''
as $$
declare
  v_texto text := btrim(coalesce(p_busqueda, ''));
  v_cedula text := public.normalizar_cedula(v_texto);
  v_condiciones text[] := array[]::text[];
begin
  if not private.es_admin_global() then
    raise exception 'Sólo un administrador del panel puede buscar en el historial'
      using errcode = '42501';
  end if;

  if v_cedula is not null and v_cedula ~ '[0-9]' then
    v_cedula := regexp_replace(v_cedula, '^0+', '');
    if v_cedula = '' then
      return;
    end if;
    v_condiciones := array[format('m.contratista_cedula like %L', v_cedula || '%')];
  else
    select coalesce(array_agg(format(
             'm.nombre_p like %L',
             '%' || replace(replace(replace(palabra, '\', '\\'), '%', '\%'), '_', '\_') || '%'
           )), array[]::text[])
      into v_condiciones
      from regexp_split_to_table(public.plegar_texto(v_texto), '\s+') as palabra
     where palabra <> '';
    if cardinality(v_condiciones) = 0 then
      return;
    end if;
  end if;

  if p_desde is not null then
    v_condiciones := v_condiciones || format('m.hora_entrada >= %L::timestamptz', p_desde);
  end if;
  if p_hasta is not null then
    v_condiciones := v_condiciones || format('m.hora_entrada < %L::timestamptz', p_hasta);
  end if;
  if p_sitio_ids is not null then
    v_condiciones := v_condiciones || format('m.sitio_id = any(%L::uuid[])', p_sitio_ids);
  end if;

  return query execute
    'select m.* from public.panel_movimientos m where ' || array_to_string(v_condiciones, ' and ');
end
$$;

revoke all on function public.panel_buscar_movimientos(text, timestamptz, timestamptz, uuid[]) from public, anon;
grant execute on function public.panel_buscar_movimientos(text, timestamptz, timestamptz, uuid[]) to authenticated;
