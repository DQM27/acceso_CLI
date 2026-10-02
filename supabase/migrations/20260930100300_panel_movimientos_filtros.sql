-- Columnas de `panel_movimientos` para que el servidor filtre y ordene las
-- columnas del Historial igual que la grilla de escritorio:
--
--   * `tipo_texto` / `medio_texto`: el texto que se ve en pantalla ("IN HOUSE",
--     la placa si el ingreso fue en vehículo). Se filtra y ordena por lo que la
--     persona lee, no por el valor interno ("IN_HOUSE").
--   * `hora_entrada_txt` / `hora_salida_txt`: la hora en Costa Rica ("19:47";
--     "Activo" si no hay salida), como la columna "Hora ...".
--   * `*_p`: la misma columna sin tildes ni mayúsculas (`plegar_texto`) para que
--     el filtro de texto encuentre "Sanchez" en "Sánchez". Postgres sólo calcula
--     una columna de la vista cuando la consulta la usa, así que no cuestan nada
--     mientras nadie las pida.
--
-- Las columnas nuevas van al final: `create or replace view` sólo admite eso.

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
  public.plegar_texto(
    coalesce(i.contratista_cedula, '') || ' ' ||
    coalesce(i.contratista_nombre, '') || ' ' ||
    coalesce(i.empresa_nombre, '')
  ) as texto_busqueda,
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
