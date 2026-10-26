-- Historial del panel para proveedores, visitas por correo y gafetes KOF.
--
-- Hasta acá el Historial del panel sólo mostraba contratistas
-- (`panel_movimientos`, sobre `ingresos`), y el de las otras tres clases
-- sólo se veía en la PC de cada unidad. Escritorio ya tiene las cuatro
-- clases (`desktop/src/pantallas/Historial.tsx`); el panel las suma con el
-- mismo selector, conservando lo suyo: todas las unidades a la vez y la
-- paginación en el servidor.
--
-- Una vista por clase con la misma forma que `panel_movimientos`: la fila
-- ya plana (unidad y dispositivo de entrada incluidos) y columnas sin tildes
-- ni mayúsculas (`*_p`) para que el servidor filtre y ordene. Los nombres
-- de las columnas de tiempo y de usuario son los mismos en las tres
-- (`hora_entrada`/`hora_salida`, `usuario_entrada_nombre`/
-- `usuario_salida_nombre`; en KOF, la entrega y la devolución del gafete),
-- así el panel las pide con el mismo código.
--
-- Sin función de búsqueda con índices de trigramas como
-- `panel_buscar_movimientos`: estas tablas son mucho más chicas que
-- `ingresos` (en producción, al escribir esto: 146 ingresos de proveedor,
-- 10 por correo y 40 préstamos KOF, contra 1.935 de contratistas). El
-- buscador del panel filtra la vista por las columnas `*_p`, acotado por
-- fecha y unidad. Si alguna crece como `ingresos`, se le da su función.
--
-- `security_invoker`: cada vista lee con los permisos de quien consulta,
-- así rige la RLS de la tabla (el administrador del panel ve todas las
-- unidades; un equipo, sólo la suya).

create view public.panel_movimientos_proveedor
with (security_invoker = true) as
select
  p.id,
  p.sitio_id,
  s.nombre as sitio_nombre,
  p.cedula,
  p.nombre,
  p.empresa_nombre,
  p.placa,
  p.gafete_numero,
  p.hora_entrada,
  p.hora_salida,
  p.usuario_entrada_nombre,
  p.usuario_salida_nombre,
  d.tipo as dispositivo_entrada_tipo,
  public.plegar_texto(coalesce(s.nombre, '')) as unidad_p,
  public.plegar_texto(coalesce(p.cedula, '')) as cedula_p,
  public.plegar_texto(coalesce(p.nombre, '')) as nombre_p,
  public.plegar_texto(coalesce(p.empresa_nombre, '')) as empresa_p,
  public.plegar_texto(coalesce(p.placa, '')) as placa_p,
  public.plegar_texto(coalesce(p.usuario_entrada_nombre, '')) as usuario_entrada_p,
  public.plegar_texto(coalesce(p.usuario_salida_nombre, '')) as usuario_salida_p
from public.ingresos_proveedor p
left join public.sitios s on s.id = p.sitio_id
left join public.dispositivos d on d.id = p.dispositivo_entrada_id;

create view public.panel_movimientos_correo
with (security_invoker = true) as
select
  c.id,
  c.sitio_id,
  s.nombre as sitio_nombre,
  c.cedula,
  c.nombre,
  c.motivo,
  c.placa,
  c.gafete_numero,
  c.hora_entrada,
  c.hora_salida,
  c.usuario_entrada_nombre,
  c.usuario_salida_nombre,
  d.tipo as dispositivo_entrada_tipo,
  public.plegar_texto(coalesce(s.nombre, '')) as unidad_p,
  public.plegar_texto(coalesce(c.cedula, '')) as cedula_p,
  public.plegar_texto(coalesce(c.nombre, '')) as nombre_p,
  public.plegar_texto(coalesce(c.motivo, '')) as motivo_p,
  public.plegar_texto(coalesce(c.placa, '')) as placa_p,
  public.plegar_texto(coalesce(c.usuario_entrada_nombre, '')) as usuario_entrada_p,
  public.plegar_texto(coalesce(c.usuario_salida_nombre, '')) as usuario_salida_p
from public.ingresos_correo c
left join public.sitios s on s.id = c.sitio_id
left join public.dispositivos d on d.id = c.dispositivo_entrada_id;

-- KOF: el encargado recibe un gafete provisional (entrega) y lo devuelve
-- (devolución). Se publican con los nombres de entrada/salida de las otras
-- vistas; el panel los rotula "Entrega", "Devolución", "Entregó", "Recibió".
create view public.panel_movimientos_kof
with (security_invoker = true) as
select
  k.id,
  k.sitio_id,
  s.nombre as sitio_nombre,
  k.encargado_codigo_empleado as codigo_empleado,
  k.encargado_nombre as nombre,
  k.gafete_numero,
  k.hora_entrega as hora_entrada,
  k.hora_devolucion as hora_salida,
  k.usuario_entrega_nombre as usuario_entrada_nombre,
  k.usuario_devolucion_nombre as usuario_salida_nombre,
  d.tipo as dispositivo_entrada_tipo,
  public.plegar_texto(coalesce(s.nombre, '')) as unidad_p,
  public.plegar_texto(coalesce(k.encargado_codigo_empleado, '')) as codigo_p,
  public.plegar_texto(coalesce(k.encargado_nombre, '')) as nombre_p,
  public.plegar_texto(coalesce(k.usuario_entrega_nombre, '')) as usuario_entrada_p,
  public.plegar_texto(coalesce(k.usuario_devolucion_nombre, '')) as usuario_salida_p
from public.prestamos_gafete_provisional k
left join public.sitios s on s.id = k.sitio_id
left join public.dispositivos d on d.id = k.dispositivo_entrega_id;

-- Proveedor y correo ya tienen `(hora_entrada desc)` para ordenar el
-- historial de todas las unidades; KOF sólo lo tenía por unidad.
create index if not exists prestamos_gafete_provisional_hora_entrega_idx
  on public.prestamos_gafete_provisional (hora_entrega desc);

-- Permisos explícitos (desde `20261020130000_permisos_api_explicitos`):
-- lectura para el panel (`authenticated`, con la RLS de cada tabla) y para
-- las Edge Functions de administración (`service_role`). Nunca `anon`.
revoke all on public.panel_movimientos_proveedor from public, anon, authenticated, service_role;
revoke all on public.panel_movimientos_correo from public, anon, authenticated, service_role;
revoke all on public.panel_movimientos_kof from public, anon, authenticated, service_role;
grant select on public.panel_movimientos_proveedor to authenticated, service_role;
grant select on public.panel_movimientos_correo to authenticated, service_role;
grant select on public.panel_movimientos_kof to authenticated, service_role;
