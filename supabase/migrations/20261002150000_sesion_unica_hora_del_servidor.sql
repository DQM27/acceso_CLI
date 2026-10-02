-- Sesión única por unidad: la hora del ingreso la pone la NUBE, con margen
-- de duda. Reemplaza la comparación de relojes de los equipos.
--
-- Antes, cada equipo mandaba la hora del ingreso según su propio reloj: un
-- equipo con el reloj atrasado (o movido a mano estando sin conexión)
-- registraba un ingreso "viejo" y la nube lo daba por desplazado aunque el
-- usuario acabara de entrar. Ahora el equipo manda cuánto tiempo pasó desde
-- el ingreso, medido con su contador de arranque (no se puede mover y
-- sigue contando con el equipo suspendido), y la nube calcula la hora con
-- su propio reloj: `now() - transcurrido`. Es el mismo principio que
-- Kronos (Lyft) y TrustedTime (Google). Funciona igual con y sin conexión:
-- un ingreso sin red hace 3 horas llega con "transcurrido = 3 h".
--
-- Cada hora lleva su margen de error (`margen_ms`): latencia de la red más
-- la deriva del contador. Sólo se decide quién ganó cuando los dos
-- ingresos están separados por más que sus márgenes. Si no (ingresos casi
-- simultáneos en dos unidades, o una fila sin margen conocido), NO se cierra
-- ninguna sesión y queda un evento `sesion_en_duda` para el panel: ante la
-- duda, ningún equipo queda sin poder operar.
--
-- Cada sesión tiene un identificador (`sesion_id`, lo genera el equipo al
-- iniciar sesión): ya no se reconoce "la misma sesión" comparando horas.
--
-- Compatibilidad: la firma anterior se elimina. Un equipo sin actualizar
-- recibe un error al llamarla y, como todo chequeo remoto, falla "abierto"
-- (no expulsa a nadie). Las filas abiertas de antes quedan sin margen (no
-- confiables) hasta la próxima sincronización de su equipo actualizado.

alter table public.sesiones_usuario
  add column sesion_id uuid,
  add column margen_ms integer check (margen_ms >= 0);

comment on column public.sesiones_usuario.iniciada_en is
  'Hora del ingreso según el reloj del servidor (now() menos lo que el equipo midió desde el ingreso).';
comment on column public.sesiones_usuario.margen_ms is
  'Error posible de iniciada_en, en ms. NULL = no confiable: nunca decide por sí sola un cierre.';

alter table public.bitacora_sesiones
  add column sesion_id uuid;

create unique index bitacora_sesiones_sesion_idx
  on public.bitacora_sesiones (dispositivo_id, sesion_id)
  where sesion_id is not null;

alter table public.eventos_seguridad_dispositivos
  drop constraint eventos_seguridad_dispositivos_tipo_check;
alter table public.eventos_seguridad_dispositivos
  add constraint eventos_seguridad_dispositivos_tipo_check check (tipo in (
    'codigo_inexistente',
    'codigo_usado',
    'codigo_vencido',
    'codigo_anulado',
    'firma_invalida',
    'hardware_distinto',
    'sesion_en_otra_unidad',
    'sesion_en_duda'
  ));

-- `true` sólo si el ingreso `a` fue SEGURO antes que `b`: los dos márgenes
-- se conocen y los intervalos [inicio - margen, inicio + margen] no se
-- tocan. Cualquier duda da `false`.
create or replace function private.sesion_claramente_anterior(
  a_inicio timestamptz,
  a_margen_ms integer,
  b_inicio timestamptz,
  b_margen_ms integer
)
returns boolean
language sql
immutable
set search_path = ''
as $$
  select a_margen_ms is not null
     and b_margen_ms is not null
     and a_inicio + pg_catalog.make_interval(secs => a_margen_ms / 1000.0)
       < b_inicio - pg_catalog.make_interval(secs => b_margen_ms / 1000.0)
$$;

revoke all on function private.sesion_claramente_anterior(timestamptz, integer, timestamptz, integer) from public;

-- Anota un inicio de sesión una sola vez por `sesion_id`.
create or replace function private.anotar_sesion_en_bitacora(
  p_usuario public.usuarios,
  p_equipo public.dispositivos,
  p_sesion_id uuid,
  p_iniciada_en timestamptz
)
returns void
language sql
security definer
set search_path = ''
as $$
  insert into public.bitacora_sesiones (
    usuario_id, cedula, nombre, dispositivo_id, dispositivo_etiqueta,
    sitio_id, sitio_nombre, iniciada_en, sesion_id
  )
  select p_usuario.id, p_usuario.cedula, p_usuario.nombre, p_equipo.id, p_equipo.etiqueta,
         p_equipo.sitio_id, sit.nombre, p_iniciada_en, p_sesion_id
    from public.sitios sit
   where sit.id = p_equipo.sitio_id
  on conflict do nothing
$$;

revoke all on function private.anotar_sesion_en_bitacora(public.usuarios, public.dispositivos, uuid, timestamptz) from public;

drop function public.sesion_usuario_en_unidad(text, timestamptz);

-- Registra la sesión `p_sesion_id` de `p_cedula` en el equipo que llama y
-- responde:
--   'vigente'     -> la sesión sigue; las de otras unidades que fueron
--                    seguro anteriores se cerraron.
--   'desplazada'  -> hubo un ingreso seguro posterior en otra unidad: este
--                    equipo debe cerrar la sesión.
--   'sin_usuario' -> la cédula no es un usuario activo de la nube (por
--                    ejemplo, el ROOT local del arranque): no aplica.
-- `p_transcurrido_ms`: milisegundos desde el ingreso según el contador de
-- arranque del equipo.
create function public.sesion_usuario_en_unidad(
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

  select * into v_usuario
    from public.usuarios
   where cedula = btrim(p_cedula) and activo;
  if not found then
    return 'sin_usuario';
  end if;

  -- Dos ingresos del mismo usuario al mismo tiempo se atienden de a uno.
  perform pg_catalog.pg_advisory_xact_lock(pg_catalog.hashtextextended(v_usuario.id::text, 0));

  select * into v_propia
    from public.sesiones_usuario
   where usuario_id = v_usuario.id and dispositivo_id = v_equipo.id;

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
    -- 5 s de latencia de la red + 100 ppm de deriva del contador.
    v_margen := 5000 + (p_transcurrido_ms / 10000)::integer;
    -- Nuevo ingreso en este equipo sin haber cerrado el anterior.
    if v_propia.usuario_id is not null then
      perform private.cerrar_en_bitacora(v_usuario.id, v_equipo.id, 'sin_cierre', v_propia.actualizada_en);
    end if;
  else
    -- Misma sesión: la hora y el margen quedan los de su primer registro.
    v_inicio := v_propia.iniciada_en;
    v_margen := v_propia.margen_ms;
  end if;

  -- ¿Hay un ingreso seguro posterior en otra unidad? Entonces este perdió.
  if exists (
    select 1 from public.sesiones_usuario s
     where s.usuario_id = v_usuario.id
       and s.sitio_id <> v_equipo.sitio_id
       and private.sesion_claramente_anterior(v_inicio, v_margen, s.iniciada_en, s.margen_ms)
  ) then
    delete from public.sesiones_usuario
     where usuario_id = v_usuario.id and dispositivo_id = v_equipo.id;
    -- Queda en la bitácora aunque nunca se hubiera registrado (ingreso sin
    -- conexión): abierta y cerrada, con el motivo.
    perform private.anotar_sesion_en_bitacora(v_usuario, v_equipo, p_sesion_id, v_inicio);
    perform private.cerrar_en_bitacora(v_usuario.id, v_equipo.id, 'desplazada', pg_catalog.now());
    return 'desplazada';
  end if;

  -- Este ingreso gana sobre las sesiones de otras unidades que fueron seguro
  -- anteriores: se cierran, con aviso en vivo a cada una y un evento.
  for v_otra in
    select s.sitio_id, sit.nombre as sitio_nombre, pg_catalog.count(*) as equipos
      from public.sesiones_usuario s
      join public.sitios sit on sit.id = s.sitio_id
     where s.usuario_id = v_usuario.id
       and s.sitio_id <> v_equipo.sitio_id
       and private.sesion_claramente_anterior(s.iniciada_en, s.margen_ms, v_inicio, v_margen)
     group by s.sitio_id, sit.nombre
  loop
    update public.bitacora_sesiones b
       set cerrada_en = greatest(pg_catalog.now(), b.iniciada_en),
           motivo_cierre = 'otra_unidad'
      from public.sesiones_usuario s
     where s.usuario_id = v_usuario.id
       and s.sitio_id = v_otra.sitio_id
       and private.sesion_claramente_anterior(s.iniciada_en, s.margen_ms, v_inicio, v_margen)
       and b.usuario_id = s.usuario_id
       and b.dispositivo_id = s.dispositivo_id
       and b.cerrada_en is null;

    delete from public.sesiones_usuario s
     where s.usuario_id = v_usuario.id
       and s.sitio_id = v_otra.sitio_id
       and private.sesion_claramente_anterior(s.iniciada_en, s.margen_ms, v_inicio, v_margen);

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

  -- Lo que queda en otras unidades es duda: las dos sesiones siguen y el
  -- panel lo ve. Una sola vez, al registrar la sesión nueva.
  if v_nueva then
    insert into public.eventos_seguridad_dispositivos (dispositivo_id, tipo, detalle)
    select v_equipo.id,
           'sesion_en_duda',
           pg_catalog.jsonb_build_object(
             'cedula', v_usuario.cedula,
             'nombre', v_usuario.nombre,
             'sitio_otro_id', s.sitio_id,
             'sitio_otro', sit.nombre,
             'sitio_id', v_equipo.sitio_id,
             'equipos', pg_catalog.count(*)
           )
      from public.sesiones_usuario s
      join public.sitios sit on sit.id = s.sitio_id
     where s.usuario_id = v_usuario.id
       and s.sitio_id <> v_equipo.sitio_id
     group by s.sitio_id, sit.nombre;

    perform private.anotar_sesion_en_bitacora(v_usuario, v_equipo, p_sesion_id, v_inicio);
  end if;

  insert into public.sesiones_usuario (usuario_id, dispositivo_id, sitio_id, iniciada_en, sesion_id, margen_ms)
  values (v_usuario.id, v_equipo.id, v_equipo.sitio_id, v_inicio, p_sesion_id, v_margen)
  on conflict (usuario_id, dispositivo_id) do update
    set sitio_id = excluded.sitio_id,
        iniciada_en = excluded.iniciada_en,
        sesion_id = excluded.sesion_id,
        margen_ms = excluded.margen_ms,
        actualizada_en = pg_catalog.now();

  return 'vigente';
end;
$$;

revoke all on function public.sesion_usuario_en_unidad(text, uuid, bigint) from public, anon;
grant execute on function public.sesion_usuario_en_unidad(text, uuid, bigint) to authenticated;
