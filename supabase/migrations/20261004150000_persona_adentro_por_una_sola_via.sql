-- Una persona no puede estar adentro por dos vías a la vez (revisión del
-- 2026-10-04, punto 5; ver docs/auditorias/revision-carreras-2026-10-04.md).
--
-- Contratistas (`ingresos`), proveedores (`ingresos_proveedor`) e ingresos
-- por correo (`ingresos_correo`) tienen cada uno su índice único de cédula
-- activa, pero esos índices no se ven entre sí: alguien podía estar adentro
-- como contratista y, a la vez, como proveedor o por correo (en la misma
-- unidad o en otra). Físicamente es la misma persona: no puede estar dos
-- veces adentro.
--
-- La cédula se compara normalizada (`public.normalizar_cedula`: sin
-- espacios, guiones ni puntos, en mayúsculas y sin el cero inicial del TSE),
-- porque cada tabla la recibe de un formulario distinto.
--
-- 1. Trigger `before insert` en las tres tablas: bajo un candado por cédula
--    (`pg_advisory_xact_lock`, el mismo desde las tres), rechaza la fila si
--    la persona tiene un ingreso abierto en cualquiera de las OTRAS dos. El
--    error es 23505 (PostgREST: 409) y nombra el índice de cédula activa de
--    la propia tabla: es lo que la cola de salida del núcleo ya reconoce
--    (`indice_persona_activa_de`) para dejar la fila fallida de inmediato.
--    Es la garantía final, también para las apps ya instaladas.
-- 2. `persona_adentro_por_otra_via(p_cedula, p_via)`: para la verificación
--    en vivo antes de registrar (apps nuevas). Responde si la persona está
--    adentro por otra vía, por cuál y dónde.
-- 3. `contratistas_`, `proveedores_` y `correos_activos_en_otras_unidades`
--    (aviso posterior a sincronizar) también reportan a quien está adentro
--    por otra vía, en cualquier unidad o equipo, con el nombre de la unidad
--    seguido de "(como contratista)", "(como proveedor)" o "(por correo)".
--    Mismas firmas: las apps instaladas muestran el aviso sin actualizarse.

-- Vía (texto) de cada tabla, para mensajes y para `p_via`.
create function private.via_de_tabla(p_tabla text)
returns text
language sql
immutable
set search_path = ''
as $$
  select case p_tabla
    when 'ingresos' then 'CONTRATISTA'
    when 'ingresos_proveedor' then 'PROVEEDOR'
    when 'ingresos_correo' then 'POR_CORREO'
  end
$$;

create function private.texto_via(p_via text)
returns text
language sql
immutable
set search_path = ''
as $$
  select case p_via
    when 'CONTRATISTA' then 'como contratista'
    when 'PROVEEDOR' then 'como proveedor'
    when 'POR_CORREO' then 'por correo'
  end
$$;

-- Ingresos abiertos de las tres vías, con la cédula normalizada. Sólo filas
-- abiertas: los índices parciales de cédula activa las recorren rápido.
create function private.personas_adentro()
returns table (via text, cedula_normalizada text, sitio_id uuid, dispositivo_entrada_id uuid)
language sql
stable
security definer
set search_path = ''
as $$
  select 'CONTRATISTA', public.normalizar_cedula(i.contratista_cedula), i.sitio_id, i.dispositivo_entrada_id
    from public.ingresos i where i.hora_salida is null
  union all
  select 'PROVEEDOR', public.normalizar_cedula(p.cedula), p.sitio_id, p.dispositivo_entrada_id
    from public.ingresos_proveedor p where p.hora_salida is null
  union all
  select 'POR_CORREO', public.normalizar_cedula(c.cedula), c.sitio_id, c.dispositivo_entrada_id
    from public.ingresos_correo c where c.hora_salida is null
$$;

revoke all on function private.via_de_tabla(text) from public;
revoke all on function private.texto_via(text) from public;
revoke all on function private.personas_adentro() from public;

-- (1) Garantía final.
create function private.persona_adentro_por_una_sola_via()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_via text := private.via_de_tabla(tg_table_name);
  v_cedula text;
  v_otra text;
begin
  if new.hora_salida is not null then
    return new;
  end if;
  if tg_table_name = 'ingresos' then
    v_cedula := public.normalizar_cedula(new.contratista_cedula);
  else
    v_cedula := public.normalizar_cedula(new.cedula);
  end if;
  if v_cedula is null then
    return new;
  end if;

  -- Mismo candado desde las tres tablas: dos ingresos simultáneos de la
  -- misma persona por vías distintas se atienden de a uno, y el segundo ve
  -- la fila del primero (READ COMMITTED, consulta con instantánea nueva).
  perform pg_catalog.pg_advisory_xact_lock(
    pg_catalog.hashtextextended('persona_adentro:' || v_cedula, 0)
  );

  select a.via into v_otra
    from private.personas_adentro() a
   where a.cedula_normalizada = v_cedula
     and a.via <> v_via
   limit 1;
  if v_otra is not null then
    raise exception 'Esta persona ya está adentro % (%)',
      private.texto_via(v_otra),
      case v_via
        when 'CONTRATISTA' then 'ingresos_contratista_activo_idx'
        when 'PROVEEDOR' then 'ingresos_proveedor_cedula_activa_idx'
        else 'ingresos_correo_cedula_activa_idx'
      end
      using errcode = '23505';
  end if;

  return new;
end;
$$;

revoke all on function private.persona_adentro_por_una_sola_via() from public;

create trigger ingresos_persona_adentro_por_una_sola_via
  before insert on public.ingresos
  for each row execute function private.persona_adentro_por_una_sola_via();

create trigger ingresos_proveedor_persona_adentro_por_una_sola_via
  before insert on public.ingresos_proveedor
  for each row execute function private.persona_adentro_por_una_sola_via();

create trigger ingresos_correo_persona_adentro_por_una_sola_via
  before insert on public.ingresos_correo
  for each row execute function private.persona_adentro_por_una_sola_via();

-- (2) Verificación en vivo: ¿está adentro por una vía distinta de `p_via`?
create function public.persona_adentro_por_otra_via(p_cedula text, p_via text)
returns table (via text, sitio_id uuid, sitio_nombre text)
language plpgsql
stable
security definer
set search_path = ''
as $$
begin
  if (private.dispositivo_que_llama()).id is null and not (select private.es_admin_global()) then
    raise exception 'Sólo un equipo registrado puede consultar ingresos activos' using errcode = '42501';
  end if;
  if p_via is null or p_via not in ('CONTRATISTA', 'PROVEEDOR', 'POR_CORREO') then
    raise exception 'Vía desconocida: %', p_via using errcode = '22023';
  end if;

  return query
    select a.via, a.sitio_id, s.nombre
      from private.personas_adentro() a
      join public.sitios s on s.id = a.sitio_id
     where a.cedula_normalizada = public.normalizar_cedula(p_cedula)
       and a.via <> p_via
     limit 1;
end;
$$;

revoke all on function public.persona_adentro_por_otra_via(text, text) from public, anon;
grant execute on function public.persona_adentro_por_otra_via(text, text) to authenticated;

-- (3) Avisos posteriores a sincronizar: lo de siempre (misma vía, otra
-- unidad u otro equipo de esta, ver 20261004120000) más lo abierto por otra
-- vía en cualquier unidad o equipo. `cedula` devuelve el texto tal como lo
-- mandó el equipo, para que pueda emparejarlo con su lista local.

create or replace function public.contratistas_activos_en_otras_unidades(p_cedulas text[])
returns table (contratista_cedula text, sitio_nombre text)
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
    select i.contratista_cedula,
           case when i.sitio_id = v_equipo.sitio_id
                then s.nombre || ' (otro equipo de esta unidad)'
                else s.nombre
           end
      from public.ingresos i
      join public.sitios s on s.id = i.sitio_id
     where i.contratista_cedula = any (p_cedulas)
       and i.hora_salida is null
       and (i.sitio_id <> v_equipo.sitio_id or i.dispositivo_entrada_id <> v_equipo.id)
    union all
    select c.pedida, s.nombre || ' (' || private.texto_via(a.via) || ')'
      from pg_catalog.unnest(p_cedulas) as c(pedida)
      join private.personas_adentro() a on a.cedula_normalizada = public.normalizar_cedula(c.pedida)
      join public.sitios s on s.id = a.sitio_id
     where a.via <> 'CONTRATISTA';
end;
$$;

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
    select i.cedula,
           case when i.sitio_id = v_equipo.sitio_id
                then s.nombre || ' (otro equipo de esta unidad)'
                else s.nombre
           end
      from public.ingresos_proveedor i
      join public.sitios s on s.id = i.sitio_id
     where i.cedula = any (p_cedulas)
       and i.hora_salida is null
       and (i.sitio_id <> v_equipo.sitio_id or i.dispositivo_entrada_id <> v_equipo.id)
    union all
    select c.pedida, s.nombre || ' (' || private.texto_via(a.via) || ')'
      from pg_catalog.unnest(p_cedulas) as c(pedida)
      join private.personas_adentro() a on a.cedula_normalizada = public.normalizar_cedula(c.pedida)
      join public.sitios s on s.id = a.sitio_id
     where a.via <> 'PROVEEDOR';
end;
$$;

create or replace function public.correos_activos_en_otras_unidades(p_cedulas text[])
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
    select i.cedula,
           case when i.sitio_id = v_equipo.sitio_id
                then s.nombre || ' (otro equipo de esta unidad)'
                else s.nombre
           end
      from public.ingresos_correo i
      join public.sitios s on s.id = i.sitio_id
     where i.cedula = any (p_cedulas)
       and i.hora_salida is null
       and (i.sitio_id <> v_equipo.sitio_id or i.dispositivo_entrada_id <> v_equipo.id)
    union all
    select c.pedida, s.nombre || ' (' || private.texto_via(a.via) || ')'
      from pg_catalog.unnest(p_cedulas) as c(pedida)
      join private.personas_adentro() a on a.cedula_normalizada = public.normalizar_cedula(c.pedida)
      join public.sitios s on s.id = a.sitio_id
     where a.via <> 'POR_CORREO';
end;
$$;
