-- La columna "Medio" del Historial muestra la placa cuando el ingreso fue en
-- vehículo (igual que la grilla de escritorio). La columna nueva va al final de
-- la vista: `create or replace view` sólo admite agregar columnas al final.

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
  i.placa
from public.ingresos i
left join public.sitios s on s.id = i.sitio_id
left join public.dispositivos d on d.id = i.dispositivo_entrada_id;
