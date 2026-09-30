-- Sesión única por UNIDAD (decisión del usuario 2026-09-12, confirmada
-- 2026-09-30). Ver docs/features-futuras/plan-sesion-unica-dispositivos.md.
--
-- Reglas:
-- * Un usuario SÍ puede tener sesión en varios equipos de la MISMA unidad a
--   la vez (PC y celular del mismo puesto).
-- * NO puede tenerla en dos unidades distintas: gana el último ingreso y se
--   cierra la sesión de la unidad anterior. Sólo se cierra la sesión; el
--   equipo sigue registrado.
-- * Sin conexión se puede entrar (la operación no se detiene); al volver la
--   conexión se reconcilia con la hora del ingreso: si en otra unidad hubo
--   un ingreso más reciente, la sesión de este equipo es la que se cierra.
-- * Cada vez que una unidad le quita la sesión a otra queda un evento de
--   seguridad, visible en el panel ("Intentos y alertas").
--
-- Mecanismo: los equipos llaman `sesion_usuario_en_unidad` al iniciar
-- sesión y en cada sincronización (~2 min). La misma llamada registra la
-- sesión y responde si sigue vigente, así el camino con y sin conexión es
-- uno solo. Al desplazar a otra unidad se le avisa en vivo por su canal
-- (`sesion_cerrada`); el equipo que recibe el aviso vuelve a llamar la
-- función antes de cerrar (el aviso no es la fuente de verdad).

create table public.sesiones_usuario (
  usuario_id uuid not null references public.usuarios (id) on delete cascade,
  dispositivo_id uuid not null references public.dispositivos (id) on delete cascade,
  sitio_id uuid not null references public.sitios (id) on delete cascade,
  -- Hora del ingreso según el equipo (puede ser de un ingreso sin conexión).
  iniciada_en timestamptz not null,
  actualizada_en timestamptz not null default now(),
  primary key (usuario_id, dispositivo_id)
);

create index sesiones_usuario_dispositivo_idx on public.sesiones_usuario (dispositivo_id);
create index sesiones_usuario_sitio_idx on public.sesiones_usuario (sitio_id);

comment on table public.sesiones_usuario is
  'Sesión abierta de cada usuario por equipo. Sólo la escriben sesion_usuario_en_unidad y cerrar_sesion_usuario_en_unidad.';

-- Sin políticas: nadie la lee ni la escribe directo. Las funciones de abajo
-- son `security definer` y validan al equipo que llama.
alter table public.sesiones_usuario enable row level security;
revoke all on public.sesiones_usuario from anon, authenticated;

-- Nuevo tipo de evento de seguridad.
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
    'sesion_en_otra_unidad'
  ));

-- Equipo que llama: sólo un dispositivo vinculado y vigente (JWT de equipo
-- con `sub` = dispositivo y `sitio_id`). `null` si no lo es.
create or replace function private.dispositivo_que_llama()
returns public.dispositivos
language sql
stable
security definer
set search_path = ''
as $$
  select d.*
    from public.dispositivos d
   where d.id = nullif((select auth.jwt()) ->> 'sub', '')::uuid
     and d.sitio_id = nullif((select auth.jwt()) ->> 'sitio_id', '')::uuid
     and d.revoked_at is null
     and d.clave_huella = ((select auth.jwt()) ->> 'huella')
$$;

revoke all on function private.dispositivo_que_llama() from public;

-- Registra la sesión de `p_cedula` en el equipo que llama y responde:
--   'vigente'     -> la sesión sigue; si había otra unidad, se le cerró.
--   'desplazada'  -> hubo un ingreso más reciente en otra unidad: este
--                    equipo debe cerrar la sesión.
--   'sin_usuario' -> la cédula no es un usuario activo de la nube (por
--                    ejemplo, el ROOT local del arranque): no aplica.
create or replace function public.sesion_usuario_en_unidad(
  p_cedula text,
  p_iniciada_en timestamptz
)
returns text
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_equipo public.dispositivos;
  v_usuario public.usuarios;
  v_iniciada timestamptz := least(coalesce(p_iniciada_en, now()), now());
  v_otra record;
begin
  v_equipo := private.dispositivo_que_llama();
  if v_equipo.id is null then
    raise exception 'Sólo un equipo registrado puede abrir sesión' using errcode = '42501';
  end if;

  select * into v_usuario
    from public.usuarios
   where cedula = btrim(p_cedula) and activo;
  if not found then
    return 'sin_usuario';
  end if;

  -- Dos ingresos del mismo usuario al mismo tiempo se atienden de a uno.
  perform pg_catalog.pg_advisory_xact_lock(pg_catalog.hashtextextended(v_usuario.id::text, 0));

  -- ¿Hay un ingreso más reciente en otra unidad? Entonces este es el viejo.
  if exists (
    select 1 from public.sesiones_usuario s
     where s.usuario_id = v_usuario.id
       and s.sitio_id <> v_equipo.sitio_id
       and s.iniciada_en > v_iniciada
  ) then
    delete from public.sesiones_usuario
     where usuario_id = v_usuario.id and dispositivo_id = v_equipo.id;
    return 'desplazada';
  end if;

  -- Este ingreso gana: se cierran las sesiones de otras unidades, con aviso
  -- en vivo a cada una y un evento de seguridad.
  for v_otra in
    select s.sitio_id, sit.nombre as sitio_nombre, pg_catalog.count(*) as equipos
      from public.sesiones_usuario s
      join public.sitios sit on sit.id = s.sitio_id
     where s.usuario_id = v_usuario.id and s.sitio_id <> v_equipo.sitio_id
     group by s.sitio_id, sit.nombre
  loop
    delete from public.sesiones_usuario
     where usuario_id = v_usuario.id and sitio_id = v_otra.sitio_id;

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

  insert into public.sesiones_usuario (usuario_id, dispositivo_id, sitio_id, iniciada_en)
  values (v_usuario.id, v_equipo.id, v_equipo.sitio_id, v_iniciada)
  on conflict (usuario_id, dispositivo_id) do update
    set sitio_id = excluded.sitio_id,
        iniciada_en = greatest(public.sesiones_usuario.iniciada_en, excluded.iniciada_en),
        actualizada_en = pg_catalog.now();

  return 'vigente';
end;
$$;

-- Cierre voluntario (el usuario sale): quita la sesión de este equipo.
-- Best-effort desde las apps: si falla, la fila vieja no molesta (un
-- ingreso posterior en otra unidad la reemplaza igual).
create or replace function public.cerrar_sesion_usuario_en_unidad(p_cedula text)
returns void
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_equipo public.dispositivos;
begin
  v_equipo := private.dispositivo_que_llama();
  if v_equipo.id is null then
    raise exception 'Sólo un equipo registrado puede cerrar sesión' using errcode = '42501';
  end if;

  delete from public.sesiones_usuario s
   using public.usuarios u
   where u.id = s.usuario_id
     and u.cedula = btrim(p_cedula)
     and s.dispositivo_id = v_equipo.id;
end;
$$;

revoke all on function public.sesion_usuario_en_unidad(text, timestamptz) from public, anon;
revoke all on function public.cerrar_sesion_usuario_en_unidad(text) from public, anon;
grant execute on function public.sesion_usuario_en_unidad(text, timestamptz) to authenticated;
grant execute on function public.cerrar_sesion_usuario_en_unidad(text) to authenticated;
