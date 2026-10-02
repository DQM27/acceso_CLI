-- Vista de movimientos para el Historial del panel.
--
-- El panel paginaba y filtraba en el navegador sobre `ingresos` con joins
-- embebidos (`sitios`, `dispositivos`), y con eso no se puede ordenar ni buscar
-- por el nombre de la unidad ni por el tipo de dispositivo. Esta vista deja la
-- fila ya plana (unidad y dispositivo incluidos) y una columna de búsqueda sin
-- tildes ni mayúsculas, para que el servidor pagine, ordene y filtre.
--
-- `security_invoker`: se ejecuta con los permisos de quien consulta, así que
-- valen las políticas RLS de `ingresos`, `sitios` y `dispositivos` (el panel ve
-- todo por `es_admin_global()`; un equipo sólo su sitio; cualquiera sin permiso,
-- cero filas). La vista no abre nada nuevo.
--
-- Hoy sólo contratistas. Si el panel suma proveedores u otros movimientos, se
-- agregan aquí con `union all` y una columna `tipo`.

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
  ) as texto_busqueda
from public.ingresos i
left join public.sitios s on s.id = i.sitio_id
left join public.dispositivos d on d.id = i.dispositivo_entrada_id;

revoke all on public.panel_movimientos from anon, authenticated;
grant select on public.panel_movimientos to authenticated;
