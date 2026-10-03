-- Ingreso único entre unidades para proveedores y para el gafete provisional
-- KOF: misma receta que 20261003170000_ingreso_unico_entre_unidades
-- (contratistas).
--
-- Proveedores: la verificación "¿ya está adentro en otra unidad?" consultaba
-- `/rest/v1/ingresos_proveedor`, y la política "leer ingresos_proveedor del
-- propio sitio o admin" sólo deja ver a cada equipo lo de SU unidad: desde
-- otra unidad siempre respondía "libre". El aviso posterior a sincronizar
-- tenía la misma ceguera.
--
-- Gafete provisional KOF: "un encargado no puede tener dos gafetes
-- provisionales a la vez" sólo se verificaba en el equipo que entregaba; ni
-- otro equipo de la misma unidad ni otra unidad lo veían.
--
-- 1. Índices únicos: la garantía final, aunque dos equipos registren en el
--    mismo milisegundo o uno lo haga sin conexión (el segundo `INSERT` recibe
--    409 y la cola de salida lo marca fallido de inmediato).
-- 2. Funciones `security definer` sólo para equipos vigentes: responden si
--    la persona está adentro (o el encargado tiene un préstamo) y en qué
--    unidad, sin abrir la lectura de las filas de otras unidades.
--
-- Antes de aplicar se verificó que no hubiera duplicados abiertos, en
-- staging y en producción (0 casos en ambas tablas).

-- (1) Índices únicos.
create unique index ingresos_proveedor_cedula_activa_idx
  on public.ingresos_proveedor (cedula)
  where hora_salida is null;

-- KOF: el encargado se identifica por su código de empleado (el catálogo ya
-- lo tiene único), no por `encargado_id`: si el mismo encargado quedara dos
-- veces en el catálogo con ids distintos, el código sigue siendo el mismo.
create unique index prestamos_gafete_provisional_encargado_activo_idx
  on public.prestamos_gafete_provisional (encargado_codigo_empleado)
  where hora_devolucion is null;

-- (2a) Ingreso de proveedor abierto con esta cédula, en cualquier unidad (a
-- lo sumo uno, por el índice). Para la verificación antes de registrar.
create or replace function public.ingreso_proveedor_activo(p_cedula text)
returns table (sitio_id uuid, sitio_nombre text)
language plpgsql
stable
security definer
set search_path = ''
as $$
begin
  if (private.dispositivo_que_llama()).id is null and not (select private.es_admin_global()) then
    raise exception 'Sólo un equipo registrado puede consultar ingresos activos' using errcode = '42501';
  end if;

  return query
    select i.sitio_id, s.nombre
      from public.ingresos_proveedor i
      join public.sitios s on s.id = i.sitio_id
     where i.cedula = pg_catalog.btrim(p_cedula)
       and i.hora_salida is null
     limit 1;
end;
$$;

revoke all on function public.ingreso_proveedor_activo(text) from public, anon;
grant execute on function public.ingreso_proveedor_activo(text) to authenticated;

-- (2b) De una lista de cédulas, los proveedores que están adentro en OTRA
-- unidad que la del equipo que llama. Para el aviso posterior a sincronizar.
create or replace function public.proveedores_activos_en_otras_unidades(p_cedulas text[])
returns table (cedula text, sitio_nombre text)
language plpgsql
stable
security definer
set search_path = ''
as $$
declare
  v_equipo public.dispositivos;
begin
  v_equipo := private.dispositivo_que_llama();
  if v_equipo.id is null then
    raise exception 'Sólo un equipo registrado puede consultar ingresos activos' using errcode = '42501';
  end if;
  if pg_catalog.cardinality(p_cedulas) > 1000 then
    raise exception 'Demasiadas cédulas' using errcode = '22023';
  end if;

  return query
    select i.cedula, s.nombre
      from public.ingresos_proveedor i
      join public.sitios s on s.id = i.sitio_id
     where i.cedula = any (p_cedulas)
       and i.sitio_id <> v_equipo.sitio_id
       and i.hora_salida is null;
end;
$$;

revoke all on function public.proveedores_activos_en_otras_unidades(text[]) from public, anon;
grant execute on function public.proveedores_activos_en_otras_unidades(text[]) to authenticated;

-- (2c) Préstamo de gafete provisional KOF sin devolver del encargado con este
-- código de empleado, en cualquier unidad (a lo sumo uno, por el índice).
-- Para la verificación antes de entregar.
create or replace function public.prestamo_provisional_activo_de_encargado(p_codigo_empleado text)
returns table (sitio_id uuid, sitio_nombre text)
language plpgsql
stable
security definer
set search_path = ''
as $$
begin
  if (private.dispositivo_que_llama()).id is null and not (select private.es_admin_global()) then
    raise exception 'Sólo un equipo registrado puede consultar préstamos activos' using errcode = '42501';
  end if;

  return query
    select p.sitio_id, s.nombre
      from public.prestamos_gafete_provisional p
      join public.sitios s on s.id = p.sitio_id
     where p.encargado_codigo_empleado = pg_catalog.btrim(p_codigo_empleado)
       and p.hora_devolucion is null
     limit 1;
end;
$$;

revoke all on function public.prestamo_provisional_activo_de_encargado(text) from public, anon;
grant execute on function public.prestamo_provisional_activo_de_encargado(text) to authenticated;
