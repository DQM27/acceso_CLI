-- Dos vistas para el panel (y más adelante escritorio), sin datos nuevos:
-- todo sale de lo que ya se guarda.
--
-- Las dos son `security_invoker`: cada quien ve sólo lo que sus políticas de
-- RLS le dejan ver en las tablas de abajo. Un admin_global del panel ve todas
-- las unidades; un equipo, sólo la suya (contratistas y empresas son
-- globales, así que ve el catálogo completo, pero "adentro" sólo de su unidad).

-- Contratistas con su estado de acceso, calculado con las MISMAS reglas que
-- `verificar_acceso` (src/domain/acceso.rs, versión 2), en el mismo orden:
--   0. empresa inactiva            -> EMPRESA_INACTIVA
--   1. sin acceso                  -> SIN_ACCESO
--   2. no requiere PRAIND          -> PERMITIDO
--   -  requiere y no tiene fecha   -> PRAIND_NO_REGISTRADO
--   3. PRAIND vencido (fecha < hoy)-> PRAIND_VENCIDO
--   4. vence en <= 30 días         -> PERMITIDO_CON_ADVERTENCIA
--   5. vigente                     -> PERMITIDO
-- Requiere PRAIND: personal de ruta, PRAIND o IN_HOUSE (`requiere_praind_de`).
-- "Hoy" es el día calendario de Costa Rica, igual que `panel_crear_contratista`.
-- Si cambian las reglas en src/domain/acceso.rs, hay que cambiar esta vista.
--
-- `adentro_*`: el ingreso abierto (sin salida) más reciente del contratista,
-- si tiene uno. Usa `ingresos_contratista_id_idx`.
create view public.panel_contratistas_estado
with (security_invoker = true) as
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
    when not r.requiere_praind then 'NO_REQUIERE'
    when c.fecha_vencimiento_praind is null then 'SIN_REGISTRO'
    when c.fecha_vencimiento_praind < h.hoy then 'VENCIDA'
    when c.fecha_vencimiento_praind <= h.hoy + 30 then 'POR_VENCER'
    else 'VIGENTE'
  end as estado_praind,
  case
    when not coalesce(e.activa, true) then 'EMPRESA_INACTIVA'
    when not c.activo then 'SIN_ACCESO'
    when not r.requiere_praind then 'PERMITIDO'
    when c.fecha_vencimiento_praind is null then 'PRAIND_NO_REGISTRADO'
    when c.fecha_vencimiento_praind < h.hoy then 'PRAIND_VENCIDO'
    when c.fecha_vencimiento_praind <= h.hoy + 30 then 'PERMITIDO_CON_ADVERTENCIA'
    else 'PERMITIDO'
  end as estado_acceso,
  a.sitio_id as adentro_sitio_id,
  s.nombre as adentro_sitio_nombre,
  a.hora_entrada as adentro_desde
from public.contratistas c
cross join (select (now() at time zone 'America/Costa_Rica')::date as hoy) h
left join public.empresas e on e.id = c.empresa_id
cross join lateral (
  select coalesce(c.es_personal_ruta, false)
         or c.tipo_ingreso in ('PRAIND', 'IN_HOUSE') as requiere_praind
) r
left join lateral (
  select i.sitio_id, i.hora_entrada
    from public.ingresos i
   where i.contratista_id = c.id
     and i.hora_salida is null
   order by i.hora_entrada desc
   limit 1
) a on true
left join public.sitios s on s.id = a.sitio_id;

revoke all on public.panel_contratistas_estado from anon, authenticated;
grant select on public.panel_contratistas_estado to authenticated;

-- Quién está adentro ahora, por unidad: contratistas, proveedores y
-- préstamos de gafete provisional KOF sin salida/devolución. Cada rama usa
-- el índice parcial de "gafete en uso"/"activos" de su tabla (sólo filas
-- abiertas), así que no recorre el historial. El tiempo adentro lo calcula
-- quien muestra la fila (`hora_entrada` es el instante).
create view public.panel_adentro_ahora
with (security_invoker = true) as
select
  'CONTRATISTA'::text as tipo,
  i.id,
  i.sitio_id,
  s.nombre as sitio_nombre,
  i.contratista_cedula as identificacion,
  i.contratista_nombre as nombre,
  i.empresa_nombre,
  i.gafete_numero,
  nullif(btrim(i.placa), '') as placa,
  i.hora_entrada,
  i.usuario_entrada_nombre
from public.ingresos i
left join public.sitios s on s.id = i.sitio_id
where i.hora_salida is null
union all
select
  'PROVEEDOR'::text,
  p.id,
  p.sitio_id,
  s.nombre,
  p.cedula,
  p.nombre,
  p.empresa_nombre,
  p.gafete_numero,
  nullif(btrim(p.placa), ''),
  p.hora_entrada,
  p.usuario_entrada_nombre
from public.ingresos_proveedor p
left join public.sitios s on s.id = p.sitio_id
where p.hora_salida is null
union all
select
  'PROVISIONAL_KOF'::text,
  k.id,
  k.sitio_id,
  s.nombre,
  k.encargado_codigo_empleado,
  k.encargado_nombre,
  'KOF',
  k.gafete_numero,
  null,
  k.hora_entrega,
  k.usuario_entrega_nombre
from public.prestamos_gafete_provisional k
left join public.sitios s on s.id = k.sitio_id
where k.hora_devolucion is null;

revoke all on public.panel_adentro_ahora from anon, authenticated;
grant select on public.panel_adentro_ahora to authenticated;
