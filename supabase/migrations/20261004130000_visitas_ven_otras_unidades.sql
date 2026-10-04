-- Visitas: "un visitante no puede estar adentro dos veces" vuelve a funcionar
-- entre unidades (revisión del 2026-10-04, punto 4; ver
-- docs/auditorias/revision-carreras-2026-10-04.md).
--
-- Mismo bug que 20261003170000_ingreso_unico_entre_unidades corrigió para
-- contratistas y 20261003190000 para proveedores: el núcleo preguntaba por
-- `/rest/v1/movimientos_visita?...&sitio_id=neq.<unidad propia>`, pero la
-- política "leer movimientos_visita del propio sitio o admin" sólo deja ver
-- a cada equipo las filas de SU unidad. La consulta, por construcción,
-- siempre respondía vacío: ni la verificación al hacer el check-in
-- (`visitante_activo_en_otro_sitio`) ni el aviso posterior a sincronizar
-- (`visitantes_con_conflicto_activo`) veían nunca nada.
--
-- 1. `movimientos_visita_cedula_activa_idx`: la garantía final. Dos visitas
--    abiertas con la misma cédula son imposibles, aunque dos equipos
--    registren a la vez o uno lo haga sin conexión: el segundo `INSERT`
--    recibe 409 y la cola de salida del núcleo lo marca fallido de inmediato
--    (`indice_persona_activa_de`), igual que con contratistas.
-- 2. `visita_activa_de_visitante` y `visitantes_activos_en_otras_unidades`:
--    lo mínimo que un equipo necesita saber (si el visitante está adentro y
--    dónde) sin abrir la lectura de las filas de otras unidades. La segunda
--    sigue el criterio de 20261004120000: también reporta lo abierto por el
--    OTRO equipo de la misma unidad.

-- (1) Antes del índice no puede haber duplicados abiertos. En vez de fallar
-- con el error genérico de `create unique index`, se explica qué hacer.
do $$
declare
  v_cedula text;
begin
  select visitante_cedula into v_cedula
    from public.movimientos_visita
   where hora_salida is null
   group by visitante_cedula
  having pg_catalog.count(*) > 1
   limit 1;
  if v_cedula is not null then
    raise exception 'Hay visitas abiertas duplicadas (por ejemplo, la cédula %): cerrar las sobrantes antes de aplicar esta migración', v_cedula;
  end if;
end;
$$;

create unique index movimientos_visita_cedula_activa_idx
  on public.movimientos_visita (visitante_cedula)
  where hora_salida is null;

-- (2a) Visita abierta de una cédula en cualquier unidad (a lo sumo una, por
-- el índice). Para la verificación al hacer el check-in.
create function public.visita_activa_de_visitante(p_cedula text)
returns table (sitio_id uuid, sitio_nombre text)
language plpgsql
stable
security definer
set search_path = ''
as $$
begin
  if (private.dispositivo_que_llama()).id is null and not (select private.es_admin_global()) then
    raise exception 'Sólo un equipo registrado puede consultar visitas activas' using errcode = '42501';
  end if;

  return query
    select m.sitio_id, s.nombre
      from public.movimientos_visita m
      join public.sitios s on s.id = m.sitio_id
     where m.visitante_cedula = pg_catalog.btrim(p_cedula)
       and m.hora_salida is null
     limit 1;
end;
$$;

revoke all on function public.visita_activa_de_visitante(text) from public, anon;
grant execute on function public.visita_activa_de_visitante(text) to authenticated;

-- (2b) De una lista de cédulas, las que están adentro en otra unidad o
-- abiertas por otro equipo de esta. Para el aviso posterior a sincronizar.
create function public.visitantes_activos_en_otras_unidades(p_cedulas text[])
returns table (visitante_cedula text, sitio_nombre text)
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
    raise exception 'Sólo un equipo registrado puede consultar visitas activas' using errcode = '42501';
  end if;
  if pg_catalog.cardinality(p_cedulas) > 1000 then
    raise exception 'Demasiadas cédulas' using errcode = '22023';
  end if;

  return query
    select m.visitante_cedula,
           case when m.sitio_id = v_equipo.sitio_id
                then s.nombre || ' (otro equipo de esta unidad)'
                else s.nombre
           end
      from public.movimientos_visita m
      join public.sitios s on s.id = m.sitio_id
     where m.visitante_cedula = any (p_cedulas)
       and m.hora_salida is null
       and (m.sitio_id <> v_equipo.sitio_id or m.dispositivo_entrada_id <> v_equipo.id);
end;
$$;

revoke all on function public.visitantes_activos_en_otras_unidades(text[]) from public, anon;
grant execute on function public.visitantes_activos_en_otras_unidades(text[]) to authenticated;
