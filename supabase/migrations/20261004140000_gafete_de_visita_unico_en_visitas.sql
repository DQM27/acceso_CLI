-- Gafete de visita: un mismo gafete no puede quedar abierto en dos visitas
-- de la misma unidad (revisión del 2026-10-04; ver
-- docs/auditorias/revision-carreras-2026-10-04.md).
--
-- `movimientos_visita` nunca tuvo índice de "gafete en uso" en la nube (la
-- unicidad era sólo local, en el SQLite de cada equipo). Los demás
-- movimientos con gafete sí lo tienen (`ingresos_gafete_activo_sitio_idx`,
-- `ingresos_proveedor_gafete_activo_sitio_idx`,
-- `ingresos_correo_gafete_activo_sitio_idx`,
-- `prestamos_gafete_provisional_gafete_activo_sitio_idx`). Sin él, si los dos
-- equipos de una unidad entregaban el mismo gafete de visita sin conexión,
-- la nube aceptaba las dos visitas y nadie se enteraba.
--
-- 1. `movimientos_visita_gafete_activo_sitio_idx`: el segundo `INSERT` recibe
--    409 con el nombre del índice, y la cola de salida del núcleo lo
--    reconoce (`gafete_activo_de`, tipo `Visita`): la visita queda fallida de
--    inmediato y el escritorio avisa del choque de gafete.
-- 2. El trigger de 20261004120000 (gafete compartido con `ingresos_correo`)
--    nombra ahora este índice cuando rechaza una VISITA, para que la cola
--    trate ese rechazo igual que un choque entre dos visitas.

-- (1) Antes del índice no puede haber duplicados abiertos.
do $$
declare
  v_fila record;
begin
  select sitio_id, gafete_numero into v_fila
    from public.movimientos_visita
   where hora_salida is null and gafete_numero is not null
   group by sitio_id, gafete_numero
  having pg_catalog.count(*) > 1
   limit 1;
  if v_fila.sitio_id is not null then
    raise exception 'Hay visitas abiertas con el mismo gafete (gafete % en la unidad %): cerrar las sobrantes antes de aplicar esta migración',
      v_fila.gafete_numero, v_fila.sitio_id;
  end if;
end;
$$;

create unique index movimientos_visita_gafete_activo_sitio_idx
  on public.movimientos_visita (sitio_id, gafete_numero)
  where hora_salida is null and gafete_numero is not null;

-- (2) Mismo trigger, con el nombre del índice también en el rechazo de una
-- visita.
create or replace function private.gafete_de_visita_libre_en_la_otra_tabla()
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
        'El gafete de visita % ya está en uso por un ingreso por correo de la unidad (movimientos_visita_gafete_activo_sitio_idx)',
        new.gafete_numero
        using errcode = '23505';
    end if;
  end if;

  return new;
end;
$$;
