-- Dos huecos encontrados en la revisión de los cambios del 2026-10-03
-- (ver docs/auditorias/revision-carreras-2026-10-04.md):
--
-- 1. Aviso de "persona adentro dos veces" también dentro de la MISMA unidad.
--
--    Los índices únicos de cédula activa (`ingresos_contratista_activo_idx`,
--    `ingresos_proveedor_cedula_activa_idx`, `ingresos_correo_cedula_activa_idx`)
--    son globales: si el otro equipo de la misma unidad ya registró a la
--    persona, el `INSERT` de este equipo (hecho sin conexión) recibe 409 y la
--    cola lo marca fallido, pero el ingreso sigue abierto en este equipo. El
--    único aviso a quien opera lo dan `*_activos_en_otras_unidades`, y esas
--    funciones filtraban `sitio_id <> unidad del equipo`: el choque con el
--    otro equipo de la misma unidad no avisaba nada (sólo subía el contador
--    de fallos de la cola) y ese ingreso nunca llegaba a la nube.
--
--    Ahora también reportan la cédula si está adentro en la misma unidad
--    pero registrada por OTRO equipo (`dispositivo_entrada_id` distinto del
--    que llama). Como el índice único no admite dos filas abiertas con la
--    misma cédula, si la fila abierta en la nube es de otro equipo, la de
--    este equipo no está en la nube (fue rechazada o todavía no se envió):
--    es un duplicado real. El nombre de la unidad viene con
--    " (otro equipo de esta unidad)" para que el aviso que ya muestran las
--    apps ("... activo acá Y en <unidad>") se lea bien sin cambiar las apps.
--    Mismas firmas: `create or replace` y las apps instaladas se benefician
--    sin actualizarse. El nombre "en_otras_unidades" queda por compatibilidad.
--
-- 2. El gafete de VISITA se comparte entre dos tablas sin barrera común.
--
--    El ingreso "por correo" (`ingresos_correo`) y las visitas
--    (`movimientos_visita`) reparten el mismo catálogo físico de gafetes de
--    visita. `ingresos_correo_gafete_activo_sitio_idx` sólo cubre su propia
--    tabla, así que dos equipos sin conexión podían entregar el mismo gafete
--    uno a una visita y otro a un ingreso por correo, y la nube aceptaba los
--    dos. Un índice no puede abarcar dos tablas: lo cubre un trigger
--    `before insert` en cada una que, bajo un candado por (unidad, gafete),
--    rechaza la fila si el gafete está abierto en la OTRA tabla.
--
--    El candado (`pg_advisory_xact_lock`) serializa dos `INSERT` simultáneos
--    en tablas distintas con el mismo gafete: el segundo espera a que el
--    primero confirme y, como la consulta del trigger toma una instantánea
--    nueva (READ COMMITTED, función volátil), ya ve la fila del primero.
--
--    El error usa `23505` (PostgREST responde 409) y, para `ingresos_correo`,
--    nombra `ingresos_correo_gafete_activo_sitio_idx` en el mensaje: es lo
--    que la cola de salida del núcleo reconoce (`es_conflicto_gafete_activo`)
--    para marcar la fila fallida de inmediato y avisar del choque de gafete,
--    igual que cuando choca con otro ingreso por correo.

-- (1) Funciones de aviso: otra unidad, u otro equipo de esta unidad.

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
       and (i.sitio_id <> v_equipo.sitio_id or i.dispositivo_entrada_id <> v_equipo.id);
end;
$$;

revoke all on function public.contratistas_activos_en_otras_unidades(text[]) from public, anon;
grant execute on function public.contratistas_activos_en_otras_unidades(text[]) to authenticated;

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
       and (i.sitio_id <> v_equipo.sitio_id or i.dispositivo_entrada_id <> v_equipo.id);
end;
$$;

revoke all on function public.proveedores_activos_en_otras_unidades(text[]) from public, anon;
grant execute on function public.proveedores_activos_en_otras_unidades(text[]) to authenticated;

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
       and (i.sitio_id <> v_equipo.sitio_id or i.dispositivo_entrada_id <> v_equipo.id);
end;
$$;

revoke all on function public.correos_activos_en_otras_unidades(text[]) from public, anon;
grant execute on function public.correos_activos_en_otras_unidades(text[]) to authenticated;

-- (2) Gafete de visita compartido entre `ingresos_correo` y `movimientos_visita`.
--
-- `security definer`: la consulta a la otra tabla no debe depender de la RLS
-- de quien inserta (las dos filas son de la misma unidad, pero así la
-- garantía no se rompe si mañana cambia una política).

create function private.gafete_de_visita_libre_en_la_otra_tabla()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
begin
  if new.gafete_numero is null or new.hora_salida is not null then
    return new;
  end if;

  -- Mismo candado desde las dos tablas: una clave por (unidad, gafete).
  perform pg_catalog.pg_advisory_xact_lock(
    pg_catalog.hashtextextended('gafete_visita:' || new.sitio_id::text || ':' || new.gafete_numero::text, 0)
  );

  if tg_table_name = 'ingresos_correo' then
    if exists (
      select 1 from public.movimientos_visita m
       where m.sitio_id = new.sitio_id
         and m.gafete_numero = new.gafete_numero
         and m.hora_salida is null
    ) then
      raise exception
        'El gafete de visita % ya está en uso por una visita de la unidad (ingresos_correo_gafete_activo_sitio_idx)',
        new.gafete_numero
        using errcode = '23505';
    end if;
  else
    if exists (
      select 1 from public.ingresos_correo c
       where c.sitio_id = new.sitio_id
         and c.gafete_numero = new.gafete_numero
         and c.hora_salida is null
    ) then
      raise exception
        'El gafete de visita % ya está en uso por un ingreso por correo de la unidad',
        new.gafete_numero
        using errcode = '23505';
    end if;
  end if;

  return new;
end;
$$;

revoke all on function private.gafete_de_visita_libre_en_la_otra_tabla() from public;

create trigger ingresos_correo_gafete_de_visita_libre
  before insert on public.ingresos_correo
  for each row execute function private.gafete_de_visita_libre_en_la_otra_tabla();

create trigger movimientos_visita_gafete_de_visita_libre
  before insert on public.movimientos_visita
  for each row execute function private.gafete_de_visita_libre_en_la_otra_tabla();
