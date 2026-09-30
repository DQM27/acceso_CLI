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
-- Ahora el texto de búsqueda (cédula, nombre, empresa y placa, sin tildes ni
-- mayúsculas) se calcula una sola vez al escribir la fila (columna generada)
-- y queda indexado con trigramas (`pg_trgm`), que es lo que permite resolver
-- un "contiene" (`like '%perez%'`) con índice en vez de recorrer la tabla.
-- Los equipos no mandan ni leen esta columna: la calcula Postgres.

create extension if not exists pg_trgm with schema extensions;

alter table public.ingresos
  add column texto_busqueda text generated always as (
    public.plegar_texto(
      coalesce(contratista_cedula, '') || ' ' ||
      coalesce(contratista_nombre, '') || ' ' ||
      coalesce(empresa_nombre, '') || ' ' ||
      coalesce(placa, '')
    )
  ) stored;

create index if not exists ingresos_texto_busqueda_trgm_idx
  on public.ingresos using gin (texto_busqueda extensions.gin_trgm_ops);

-- Misma vista, misma forma: sólo `texto_busqueda` pasa a leer la columna
-- guardada (e incluye la placa) en vez de calcularse por fila.
create or replace view public.panel_movimientos
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
  i.texto_busqueda,
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
  public.plegar_texto(coalesce(i.contratista_nombre, '')) as nombre_p,
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
