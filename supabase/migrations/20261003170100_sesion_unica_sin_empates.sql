-- Sesión única por unidad, sin empates: siempre queda una sola unidad.
--
-- Hasta acá (20261002150000_sesion_unica_hora_del_servidor), cada hora de
-- ingreso llevaba un margen de 5 s + deriva, y si dos ingresos en unidades
-- distintas caían dentro de sus márgenes ("duda") NO se cerraba ninguno.
-- En la práctica eso dejaba las dos sesiones abiertas para siempre: cada
-- sincronización volvía a comparar las mismas horas y nunca decidía. Visto
-- en staging el 2026-10-03: ingresos con 7 y 9 s de diferencia en Brisas y
-- CDI Cartago quedaron abiertos los dos hasta que el usuario salió a mano.
--
-- Ahora (decisión del dueño, 2026-10-03):
-- 1. Margen real: 1 s de latencia de la red (antes 5 s) + 100 ppm de deriva
--    del contador del equipo. El equipo mide el transcurrido en ms con un
--    contador que no se puede mover; la única incertidumbre de verdad es
--    lo que tarda el pedido en llegar.
-- 2. Si aun así se solapan, gana la ÚLTIMA en llegar a la nube:
--    `registrada_en` es `clock_timestamp()` (µs) del primer registro de cada
--    sesión, tomado bajo el candado por usuario que ya serializa los
--    ingresos. Nunca hay dos iguales; si los hubiera, desempata el id del
--    equipo. Las filas viejas sin margen (`margen_ms` NULL) también se
--    deciden así en vez de quedar en duda.
--
-- Con esto `sesion_en_duda` deja de emitirse (el valor sigue permitido en
-- `eventos_seguridad_dispositivos` para no romper el historial).

alter table public.sesiones_usuario
  add column registrada_en timestamptz;

-- Filas abiertas de antes: su mejor aproximación de llegada es la hora de
-- ingreso que ya tenían.
update public.sesiones_usuario set registrada_en = iniciada_en where registrada_en is null;

alter table public.sesiones_usuario
  alter column registrada_en set not null,
  alter column registrada_en set default pg_catalog.clock_timestamp();

comment on column public.sesiones_usuario.registrada_en is
  'Llegada a la nube del primer registro de esta sesión (clock_timestamp). Desempata ingresos solapados: gana la última.';

-- ¿La sesión A es anterior a la B (y por lo tanto pierde contra ella)?
-- Primero por hora de ingreso, si los márgenes no se tocan; si se tocan (o
-- falta un margen), por orden de llegada a la nube; y como último recurso,
-- por id de equipo. Siempre da una respuesta: para dos sesiones distintas,
-- exactamente una de `sesion_anterior(a, b)` / `sesion_anterior(b, a)` es
-- verdadera.
create or replace function private.sesion_anterior(
  a_inicio timestamptz,
  a_margen_ms integer,
  a_registrada timestamptz,
  a_dispositivo uuid,
  b_inicio timestamptz,
  b_margen_ms integer,
  b_registrada timestamptz,
  b_dispositivo uuid
)
returns boolean
language sql
immutable
set search_path = ''
as $$
  select case
    when private.sesion_claramente_anterior(a_inicio, a_margen_ms, b_inicio, b_margen_ms) then true
    when private.sesion_claramente_anterior(b_inicio, b_margen_ms, a_inicio, a_margen_ms) then false
    else (a_registrada, a_dispositivo) < (b_registrada, b_dispositivo)
  end
$$;

revoke all on function private.sesion_anterior(timestamptz, integer, timestamptz, uuid, timestamptz, integer, timestamptz, uuid) from public;

create or replace function public.sesion_usuario_en_unidad(
  p_cedula text,
  p_sesion_id uuid,
  p_transcurrido_ms bigint
)
returns text
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_equipo public.dispositivos;
  v_usuario public.usuarios;
  v_propia public.sesiones_usuario;
  v_otra record;
  v_nueva boolean;
  v_inicio timestamptz;
  v_margen integer;
  v_registrada timestamptz;
begin
  v_equipo := private.dispositivo_que_llama();
  if v_equipo.id is null then
    raise exception 'Sólo un equipo registrado puede abrir sesión' using errcode = '42501';
  end if;

  -- Más de 31 días seguidos de sesión no es un uso real: dato roto.
  if p_sesion_id is null or p_transcurrido_ms is null
     or p_transcurrido_ms < 0 or p_transcurrido_ms > 31::bigint * 24 * 3600 * 1000 then
    raise exception 'Duración de sesión inválida' using errcode = '22023';
  end if;

  -- Asignación con subconsulta a propósito: el SQL Editor de Supabase
  -- malinterpreta la otra forma de cargar una fila en una variable (la toma
  -- por la creación de una tabla, pide confirmar RLS y parte el script por
  -- cada punto y coma, también dentro de esta función). Mismo resultado.
  v_usuario := (select u from public.usuarios u where u.cedula = btrim(p_cedula) and u.activo);
  if v_usuario.id is null then
    return 'sin_usuario';
  end if;

  -- Dos ingresos del mismo usuario al mismo tiempo se atienden de a uno: el
  -- orden en que pasan este candado es el orden de llegada.
  perform pg_catalog.pg_advisory_xact_lock(pg_catalog.hashtextextended(v_usuario.id::text, 0));

  v_propia := (select s from public.sesiones_usuario s
                where s.usuario_id = v_usuario.id and s.dispositivo_id = v_equipo.id);

  -- Otra unidad ya cerró esta sesión (su fila se borró y la bitácora la
  -- tiene cerrada): la decisión no se revisa.
  if v_propia.usuario_id is null and exists (
    select 1 from public.bitacora_sesiones b
     where b.dispositivo_id = v_equipo.id and b.sesion_id = p_sesion_id and b.cerrada_en is not null
  ) then
    return 'desplazada';
  end if;

  v_nueva := v_propia.usuario_id is null or v_propia.sesion_id is distinct from p_sesion_id;
  if v_nueva then
    v_inicio := pg_catalog.now() - pg_catalog.make_interval(secs => p_transcurrido_ms / 1000.0);
    -- 1 s de latencia de la red + 100 ppm de deriva del contador.
    v_margen := 1000 + (p_transcurrido_ms / 10000)::integer;
    -- Después del candado: es la llegada de verdad, no la del pedido.
    v_registrada := pg_catalog.clock_timestamp();
    -- Nuevo ingreso en este equipo sin haber cerrado el anterior.
    if v_propia.usuario_id is not null then
      perform private.cerrar_en_bitacora(v_usuario.id, v_equipo.id, 'sin_cierre', v_propia.actualizada_en);
    end if;
  else
    -- Misma sesión: hora, margen y llegada quedan los de su primer registro.
    v_inicio := v_propia.iniciada_en;
    v_margen := v_propia.margen_ms;
    v_registrada := v_propia.registrada_en;
  end if;

  -- ¿Hay en otra unidad una sesión posterior a esta? Entonces esta perdió.
  if exists (
    select 1 from public.sesiones_usuario s
     where s.usuario_id = v_usuario.id
       and s.sitio_id <> v_equipo.sitio_id
       and private.sesion_anterior(v_inicio, v_margen, v_registrada, v_equipo.id,
                                   s.iniciada_en, s.margen_ms, s.registrada_en, s.dispositivo_id)
  ) then
    delete from public.sesiones_usuario
     where usuario_id = v_usuario.id and dispositivo_id = v_equipo.id;
    -- Queda en la bitácora aunque nunca se hubiera registrado (ingreso sin
    -- conexión): abierta y cerrada, con el motivo.
    perform private.anotar_sesion_en_bitacora(v_usuario, v_equipo, p_sesion_id, v_inicio);
    perform private.cerrar_en_bitacora(v_usuario.id, v_equipo.id, 'desplazada', pg_catalog.now());
    return 'desplazada';
  end if;

  -- Esta sesión gana: se cierran todas las de otras unidades (ya no queda
  -- ninguna en duda), con aviso en vivo a cada una y un evento de seguridad.
  for v_otra in
    select s.sitio_id, sit.nombre as sitio_nombre, pg_catalog.count(*) as equipos
      from public.sesiones_usuario s
      join public.sitios sit on sit.id = s.sitio_id
     where s.usuario_id = v_usuario.id
       and s.sitio_id <> v_equipo.sitio_id
     group by s.sitio_id, sit.nombre
  loop
    update public.bitacora_sesiones b
       set cerrada_en = greatest(pg_catalog.now(), b.iniciada_en),
           motivo_cierre = 'otra_unidad'
      from public.sesiones_usuario s
     where s.usuario_id = v_usuario.id
       and s.sitio_id = v_otra.sitio_id
       and b.usuario_id = s.usuario_id
       and b.dispositivo_id = s.dispositivo_id
       and b.cerrada_en is null;

    delete from public.sesiones_usuario s
     where s.usuario_id = v_usuario.id
       and s.sitio_id = v_otra.sitio_id;

    perform realtime.send(
      pg_catalog.jsonb_build_object(
        'cedula', v_usuario.cedula,
        'sitio_nuevo_id', v_equipo.sitio_id,
        'ocurrido_en', pg_catalog.now()
      ),
      'sesion_cerrada',
      'sitio:' || v_otra.sitio_id::text,
      true
    );

    insert into public.eventos_seguridad_dispositivos (dispositivo_id, tipo, detalle)
    values (
      v_equipo.id,
      'sesion_en_otra_unidad',
      pg_catalog.jsonb_build_object(
        'cedula', v_usuario.cedula,
        'nombre', v_usuario.nombre,
        'sitio_anterior_id', v_otra.sitio_id,
        'sitio_anterior', v_otra.sitio_nombre,
        'sitio_nuevo_id', v_equipo.sitio_id,
        'equipos_cerrados', v_otra.equipos
      )
    );
  end loop;

  if v_nueva then
    perform private.anotar_sesion_en_bitacora(v_usuario, v_equipo, p_sesion_id, v_inicio);
  end if;

  insert into public.sesiones_usuario (usuario_id, dispositivo_id, sitio_id, iniciada_en, sesion_id, margen_ms, registrada_en)
  values (v_usuario.id, v_equipo.id, v_equipo.sitio_id, v_inicio, p_sesion_id, v_margen, v_registrada)
  on conflict (usuario_id, dispositivo_id) do update
    set sitio_id = excluded.sitio_id,
        iniciada_en = excluded.iniciada_en,
        sesion_id = excluded.sesion_id,
        margen_ms = excluded.margen_ms,
        registrada_en = excluded.registrada_en,
        actualizada_en = pg_catalog.now();

  return 'vigente';
end;
$$;

revoke all on function public.sesion_usuario_en_unidad(text, uuid, bigint) from public, anon;
grant execute on function public.sesion_usuario_en_unidad(text, uuid, bigint) to authenticated;
