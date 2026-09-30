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
-- Bitácora (pedido del usuario 2026-09-30): cada inicio de sesión queda en
-- `bitacora_sesiones` con usuario, unidad, equipo, fecha y hora de inicio y
-- de cierre, y el motivo del cierre. El panel la muestra en "Sesiones".
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

-- Sólo la escriben las funciones de abajo (`security definer`, validan al
-- equipo que llama). La lee únicamente el administrador del panel, a través
-- de `panel_bitacora_sesiones` (última actividad de las sesiones abiertas).
alter table public.sesiones_usuario enable row level security;
revoke all on public.sesiones_usuario from anon, authenticated;
grant select on public.sesiones_usuario to authenticated;
create policy "admin_global lee las sesiones abiertas" on public.sesiones_usuario
  for select to authenticated
  using ((select private.es_admin_global()));

-- Bitácora de sesiones. Guarda nombre de usuario, unidad y equipo tal como
-- eran en ese momento: si después se borra o renombra algo, el registro no
-- cambia. Una fila por inicio de sesión (no por sincronización).
--   motivo_cierre:
--     'salida'      -> el usuario cerró sesión;
--     'otra_unidad' -> se cerró porque entró en otra unidad;
--     'desplazada'  -> entró sin conexión y al reconectar ya había un
--                      ingreso más reciente en otra unidad;
--     'sin_cierre'  -> el equipo volvió a iniciar sesión sin haber cerrado la
--                      anterior (app cerrada de golpe, equipo apagado).
--   `cerrada_en` en 'sin_cierre' es la última actividad conocida de esa
--   sesión, no la hora exacta del cierre.
create table public.bitacora_sesiones (
  id bigint generated always as identity primary key,
  usuario_id uuid references public.usuarios (id) on delete set null,
  cedula text not null,
  nombre text not null,
  dispositivo_id uuid references public.dispositivos (id) on delete set null,
  dispositivo_etiqueta text,
  sitio_id uuid references public.sitios (id) on delete set null,
  sitio_nombre text,
  iniciada_en timestamptz not null,
  cerrada_en timestamptz,
  motivo_cierre text check (motivo_cierre in ('salida', 'otra_unidad', 'desplazada', 'sin_cierre')),
  constraint bitacora_sesiones_cierre_coherente check ((cerrada_en is null) = (motivo_cierre is null)),
  constraint bitacora_sesiones_unica unique (usuario_id, dispositivo_id, iniciada_en)
);

create index bitacora_sesiones_iniciada_idx on public.bitacora_sesiones (iniciada_en desc);
create index bitacora_sesiones_usuario_idx on public.bitacora_sesiones (usuario_id, iniciada_en desc);
create index bitacora_sesiones_sitio_idx on public.bitacora_sesiones (sitio_id, iniciada_en desc);
create index bitacora_sesiones_dispositivo_idx on public.bitacora_sesiones (dispositivo_id);
create index bitacora_sesiones_abiertas_idx on public.bitacora_sesiones (usuario_id) where cerrada_en is null;

-- La lee sólo el administrador del panel; la escriben sólo las funciones.
alter table public.bitacora_sesiones enable row level security;
revoke all on public.bitacora_sesiones from anon, authenticated;
grant select on public.bitacora_sesiones to authenticated;
create policy "admin_global lee la bitácora de sesiones" on public.bitacora_sesiones
  for select to authenticated
  using ((select private.es_admin_global()));

-- Cierra en la bitácora las sesiones abiertas de un usuario en un equipo.
create or replace function private.cerrar_en_bitacora(
  p_usuario_id uuid,
  p_dispositivo_id uuid,
  p_motivo text,
  p_cerrada_en timestamptz
)
returns void
language sql
security definer
set search_path = ''
as $$
  update public.bitacora_sesiones
     set cerrada_en = greatest(p_cerrada_en, iniciada_en),
         motivo_cierre = p_motivo
   where usuario_id = p_usuario_id
     and dispositivo_id = p_dispositivo_id
     and cerrada_en is null
$$;

revoke all on function private.cerrar_en_bitacora(uuid, uuid, text, timestamptz) from public;

-- Anota un inicio de sesión (idempotente: la misma sesión no se repite).
create or replace function private.anotar_en_bitacora(
  p_usuario public.usuarios,
  p_equipo public.dispositivos,
  p_iniciada_en timestamptz,
  p_cerrada_en timestamptz default null,
  p_motivo text default null
)
returns void
language sql
security definer
set search_path = ''
as $$
  insert into public.bitacora_sesiones (
    usuario_id, cedula, nombre, dispositivo_id, dispositivo_etiqueta,
    sitio_id, sitio_nombre, iniciada_en, cerrada_en, motivo_cierre
  )
  select p_usuario.id, p_usuario.cedula, p_usuario.nombre, p_equipo.id, p_equipo.etiqueta,
         p_equipo.sitio_id, sit.nombre, p_iniciada_en, p_cerrada_en, p_motivo
    from public.sitios sit
   where sit.id = p_equipo.sitio_id
  on conflict (usuario_id, dispositivo_id, iniciada_en) do nothing
$$;

revoke all on function private.anotar_en_bitacora(public.usuarios, public.dispositivos, timestamptz, timestamptz, text) from public;

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
  v_propia public.sesiones_usuario;
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

  select * into v_propia
    from public.sesiones_usuario
   where usuario_id = v_usuario.id and dispositivo_id = v_equipo.id;

  -- ¿Hay un ingreso más reciente en otra unidad? Entonces este es el viejo.
  if exists (
    select 1 from public.sesiones_usuario s
     where s.usuario_id = v_usuario.id
       and s.sitio_id <> v_equipo.sitio_id
       and s.iniciada_en > v_iniciada
  ) then
    delete from public.sesiones_usuario
     where usuario_id = v_usuario.id and dispositivo_id = v_equipo.id;
    -- Queda en la bitácora aunque nunca se hubiera registrado (ingreso
    -- sin conexión): abierta y cerrada, con el motivo.
    perform private.anotar_en_bitacora(v_usuario, v_equipo, v_iniciada);
    perform private.cerrar_en_bitacora(v_usuario.id, v_equipo.id, 'desplazada', pg_catalog.now());
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
    update public.bitacora_sesiones b
       set cerrada_en = greatest(pg_catalog.now(), b.iniciada_en),
           motivo_cierre = 'otra_unidad'
      from public.sesiones_usuario s
     where s.usuario_id = v_usuario.id
       and s.sitio_id = v_otra.sitio_id
       and b.usuario_id = s.usuario_id
       and b.dispositivo_id = s.dispositivo_id
       and b.cerrada_en is null;

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

  -- Nuevo ingreso en este equipo sin haber cerrado el anterior: el anterior
  -- se cierra en la bitácora con su última actividad conocida.
  if v_propia.usuario_id is not null and v_propia.iniciada_en < v_iniciada then
    perform private.cerrar_en_bitacora(v_usuario.id, v_equipo.id, 'sin_cierre', v_propia.actualizada_en);
  end if;
  if v_propia.usuario_id is null or v_propia.iniciada_en < v_iniciada then
    perform private.anotar_en_bitacora(v_usuario, v_equipo, v_iniciada);
  end if;

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

  update public.bitacora_sesiones b
     set cerrada_en = greatest(pg_catalog.now(), b.iniciada_en),
         motivo_cierre = 'salida'
    from public.usuarios u
   where u.cedula = btrim(p_cedula)
     and b.usuario_id = u.id
     and b.dispositivo_id = v_equipo.id
     and b.cerrada_en is null;

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

-- Vista del panel: la bitácora con la última actividad de las sesiones que
-- siguen abiertas (la actualiza cada sincronización del equipo) y la
-- duración. `security_invoker`: sólo la ve el admin_global.
create view public.panel_bitacora_sesiones
with (security_invoker = true) as
select
  b.id,
  b.usuario_id,
  b.cedula,
  b.nombre,
  b.dispositivo_id,
  b.dispositivo_etiqueta,
  d.tipo as dispositivo_tipo,
  b.sitio_id,
  b.sitio_nombre,
  b.iniciada_en,
  b.cerrada_en,
  b.motivo_cierre,
  coalesce(b.cerrada_en, s.actualizada_en, b.iniciada_en) as ultima_actividad,
  (b.cerrada_en is null) as abierta
from public.bitacora_sesiones b
left join public.dispositivos d on d.id = b.dispositivo_id
left join public.sesiones_usuario s
  on s.usuario_id = b.usuario_id
 and s.dispositivo_id = b.dispositivo_id
 and b.cerrada_en is null;

revoke all on public.panel_bitacora_sesiones from anon, authenticated;
grant select on public.panel_bitacora_sesiones to authenticated;
