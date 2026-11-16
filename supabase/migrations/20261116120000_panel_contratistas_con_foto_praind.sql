-- La lista de contratistas del panel muestra si cada uno tiene la foto de su
-- gafete PRAIND cargada (columna "Foto PRAIND", igual que en el escritorio).
--
-- Se agrega `tiene_foto_praind` AL FINAL de la vista: `create or replace view`
-- sólo admite columnas nuevas al final, y así no cambia nada de lo que ya
-- leen el panel ni las vistas que dependen de esta (lienzo, ia_consultas).
-- Resto de la definición sin cambios (ver `vistas_estado_y_adentro`).
-- `create or replace` conserva los permisos (SELECT para authenticated) y
-- `security_invoker` se vuelve a dar explícito: la RLS de contratistas sigue
-- decidiendo quién ve qué.

create or replace view public.panel_contratistas_estado
with (security_invoker = true)
as
select
  c.id,
  c.identificacion,
  c.nombre,
  c.empresa_id,
  coalesce(e.nombre, c.empresa_nombre) as empresa_nombre,
  coalesce(e.activa, true) as empresa_activa,
  c.tipo_ingreso,
  coalesce(c.es_personal_ruta, false) as es_personal_ruta,
  c.fecha_vencimiento_praind,
  c.activo,
  r.requiere_praind,
  c.fecha_vencimiento_praind - h.hoy as dias_para_vencer,
  case
    when not r.requiere_praind then 'NO_REQUIERE'::text
    when c.fecha_vencimiento_praind is null then 'SIN_REGISTRO'::text
    when c.fecha_vencimiento_praind < h.hoy then 'VENCIDA'::text
    when c.fecha_vencimiento_praind <= (h.hoy + 30) then 'POR_VENCER'::text
    else 'VIGENTE'::text
  end as estado_praind,
  case
    when not coalesce(e.activa, true) then 'EMPRESA_INACTIVA'::text
    when not c.activo then 'SIN_ACCESO'::text
    when not r.requiere_praind then 'PERMITIDO'::text
    when c.fecha_vencimiento_praind is null then 'PRAIND_NO_REGISTRADO'::text
    when c.fecha_vencimiento_praind < h.hoy then 'PRAIND_VENCIDO'::text
    when c.fecha_vencimiento_praind <= (h.hoy + 30) then 'PERMITIDO_CON_ADVERTENCIA'::text
    else 'PERMITIDO'::text
  end as estado_acceso,
  a.sitio_id as adentro_sitio_id,
  s.nombre as adentro_sitio_nombre,
  a.hora_entrada as adentro_desde,
  c.praind_foto is not null as tiene_foto_praind
from public.contratistas c
cross join (select (now() at time zone 'America/Costa_Rica')::date as hoy) h
left join public.empresas e on e.id = c.empresa_id
cross join lateral (
  select coalesce(c.es_personal_ruta, false)
         or (c.tipo_ingreso = any (array['PRAIND'::text, 'IN_HOUSE'::text])) as requiere_praind
) r
left join lateral (
  select i.sitio_id, i.hora_entrada
  from public.ingresos i
  where i.contratista_id = c.id and i.hora_salida is null
  order by i.hora_entrada desc
  limit 1
) a on true
left join public.sitios s on s.id = a.sitio_id;

grant select on public.panel_contratistas_estado to authenticated, service_role;
