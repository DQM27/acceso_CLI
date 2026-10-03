-- Un contratista no puede tener dos ingresos abiertos a la vez, ni en la
-- misma unidad ni en unidades distintas.
--
-- La regla ya existía en el núcleo (`application::con_nube`, consulta en
-- vivo antes de registrar, y el aviso posterior a sincronizar), pero estaba
-- ciega: preguntaba por `/rest/v1/ingresos` y la política "leer ingresos del
-- propio sitio" sólo le deja ver a cada equipo los ingresos de SU unidad.
-- Desde otra unidad, la consulta siempre respondía "libre" (visto en
-- staging el 2026-10-03: el mismo contratista quedó adentro en dos unidades
-- con un minuto de diferencia). Y el índice único que el núcleo daba por
-- hecho para la carrera de milisegundos ("gana el primero en escribir")
-- nunca existió.
--
-- 1. `ingresos_contratista_activo_idx`: la garantía final. Dos ingresos
--    abiertos con la misma cédula son imposibles, aunque dos equipos
--    registren en el mismo milisegundo o uno haya registrado sin conexión:
--    el segundo `INSERT` recibe 409 y la cola de salida lo marca fallido
--    de inmediato (mismo trato que el gafete duplicado).
-- 2. `ingreso_activo_de_contratista` y
--    `contratistas_activos_en_otras_unidades`: lo mínimo que un equipo
--    necesita saber de otra unidad (si la persona está adentro y dónde),
--    sin abrir la lectura de los ingresos ajenos.

-- (1) Antes de crear el índice no puede haber duplicados abiertos: se
-- verificó en staging y producción (0 casos) antes de aplicar.
create unique index ingresos_contratista_activo_idx
  on public.ingresos (contratista_cedula)
  where hora_salida is null;

-- (2a) Ingreso abierto de una cédula en cualquier unidad (a lo sumo uno, por
-- el índice de arriba). Para la verificación antes de registrar.
create or replace function public.ingreso_activo_de_contratista(p_cedula text)
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
      from public.ingresos i
      join public.sitios s on s.id = i.sitio_id
     where i.contratista_cedula = pg_catalog.btrim(p_cedula)
       and i.hora_salida is null
     limit 1;
end;
$$;

revoke all on function public.ingreso_activo_de_contratista(text) from public, anon;
grant execute on function public.ingreso_activo_de_contratista(text) to authenticated;

-- (2b) De una lista de cédulas, las que están adentro en OTRA unidad que la
-- del equipo que llama. Para el aviso posterior a sincronizar (ingresos
-- registrados sin conexión que se colaron antes de que existiera el índice,
-- o que el índice rechazó).
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
    select i.contratista_cedula, s.nombre
      from public.ingresos i
      join public.sitios s on s.id = i.sitio_id
     where i.contratista_cedula = any (p_cedulas)
       and i.sitio_id <> v_equipo.sitio_id
       and i.hora_salida is null;
end;
$$;

revoke all on function public.contratistas_activos_en_otras_unidades(text[]) from public, anon;
grant execute on function public.contratistas_activos_en_otras_unidades(text[]) to authenticated;
