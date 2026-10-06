-- Cuentas de anfitriones administradas desde el panel, con código de
-- activación (sin Google y sin correos). Ver
-- docs/auditorias/investigacion-login-correo-web-visitas-2026-10-06.md.
--
-- Flujo:
-- 1. El panel (Edge Function `admin-anfitriones`) da de alta correo y
--    nombre, crea la cuenta de Supabase Auth con una contraseña ALEATORIA que
--    nadie conoce y emite un código de activación que se muestra una sola vez.
-- 2. La persona entra a la web de visitas con correo + código y elige su
--    contraseña (Edge Function `anfitrion-activar`). Con el código NUNCA se
--    abre una sesión: no es una contraseña de Auth. Por eso su vencimiento
--    (72 h) y su límite de intentos (5) se cumplen de verdad, y no hace falta
--    tocar ninguna política RLS.
-- 3. "Restablecer" emite otro código y vuelve a poner una contraseña
--    aleatoria (la anterior deja de servir) y cierra sus sesiones.
--    "Deshabilitar" pone activo = false (las funciones de la base ya lo
--    exigen), y la Edge Function bloquea la cuenta en Auth.
--
-- El código se guarda solo como hash bcrypt en `private.` (fuera de la API;
-- ni el anfitrión ni el panel pueden leerlo). Todas las funciones que lo
-- tocan son SECURITY DEFINER y solo las ejecuta `service_role` (las Edge
-- Functions), salvo `panel_anfitriones`, que exige administrador del panel.

-- (1) `auth.email()` siempre llega en minúsculas: un correo con mayúsculas o
-- espacios nunca coincidiría y la persona quedaría sin acceso sin aviso.
alter table public.anfitriones
  add constraint anfitriones_correo_normalizado
  check (correo = lower(btrim(correo)) and correo like '_%@_%._%');

-- (2) Enlace con la cuenta de Auth (lo llena `admin-anfitriones`).
alter table public.anfitriones
  add column auth_user_id uuid unique references auth.users (id) on delete set null;

update public.anfitriones a
set auth_user_id = u.id
from auth.users u
where lower(u.email) = a.correo and a.auth_user_id is null;

comment on table public.anfitriones is
  'Quién puede agendar visitas en visitas.megabrisas.com. Alta, restablecer y baja desde el panel (Edge Function admin-anfitriones); la persona activa su cuenta con el código en la web de visitas. Esta tabla decide la autorización real.';
comment on column public.anfitriones.auth_user_id is
  'Cuenta de Supabase Auth del anfitrión. La crea admin-anfitriones con una contraseña aleatoria; la persona pone la suya al activar con el código.';

-- (3) Códigos de activación pendientes (uno por anfitrión).
create table private.anfitriones_activacion (
  correo text primary key references public.anfitriones (correo) on update cascade on delete cascade,
  codigo_hash text not null,
  vence timestamptz not null,
  intentos smallint not null default 0,
  emitido_por text not null,
  emitido_en timestamptz not null default now()
);
comment on table private.anfitriones_activacion is
  'Código de activación vigente de cada anfitrión (hash bcrypt, vence a las 72 h, máximo 5 intentos). Se borra al activar o al deshabilitar.';

-- (4) Bitácora: quién hizo qué y cuándo.
create table private.bitacora_anfitriones (
  id bigint generated always as identity primary key,
  correo text not null,
  accion text not null check (accion in ('crear', 'restablecer', 'deshabilitar', 'habilitar', 'activar')),
  hecho_por text not null,
  creado_en timestamptz not null default now()
);
create index bitacora_anfitriones_correo_idx on private.bitacora_anfitriones (correo, creado_en desc);

-- Fuera de la API y sin políticas: solo las funciones SECURITY DEFINER de
-- abajo las tocan. RLS activo igual, por si algún día se expone el esquema.
alter table private.anfitriones_activacion enable row level security;
alter table private.bitacora_anfitriones enable row level security;
revoke all on table private.anfitriones_activacion from public, anon, authenticated;
revoke all on table private.bitacora_anfitriones from public, anon, authenticated;

-- Constantes de la política, en un solo lugar.
create function private.activacion_vigencia() returns interval
language sql immutable set search_path = '' as $$ select interval '72 hours' $$;
create function private.activacion_intentos_maximos() returns smallint
language sql immutable set search_path = '' as $$ select 5::smallint $$;

-- (5) Emite (o reemplaza) el código de un anfitrión activo. El código lo
-- genera la Edge Function; acá solo se guarda su hash.
create function public.emitir_codigo_anfitrion(p_correo text, p_codigo text, p_emitido_por text, p_accion text)
returns timestamptz
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_vence timestamptz := now() + private.activacion_vigencia();
begin
  if p_accion not in ('crear', 'restablecer') then
    raise exception 'Acción inválida: %', p_accion using errcode = '22023';
  end if;
  if length(coalesce(p_codigo, '')) < 10 then
    raise exception 'Código demasiado corto' using errcode = '22023';
  end if;
  if not exists (select 1 from public.anfitriones where correo = p_correo and activo) then
    raise exception 'Anfitrión inexistente o deshabilitado' using errcode = 'P0002';
  end if;

  insert into private.anfitriones_activacion (correo, codigo_hash, vence, intentos, emitido_por, emitido_en)
  values (p_correo, extensions.crypt(p_codigo, extensions.gen_salt('bf', 10)), v_vence, 0, p_emitido_por, now())
  on conflict (correo) do update
    set codigo_hash = excluded.codigo_hash,
        vence = excluded.vence,
        intentos = 0,
        emitido_por = excluded.emitido_por,
        emitido_en = excluded.emitido_en;

  insert into private.bitacora_anfitriones (correo, accion, hecho_por) values (p_correo, p_accion, p_emitido_por);
  return v_vence;
end;
$$;

-- (6) Verifica un código. Devuelve la cuenta de Auth, o null por CUALQUIER
-- motivo (no existe, vencido, sin intentos, deshabilitado, código errado):
-- quien llama no debe poder distinguirlos. Un fallo suma un intento y queda
-- guardado (la función no lanza excepciones, así el contador no se revierte).
-- No consume el código: eso se hace después de guardar la contraseña, para
-- que una contraseña rechazada por débil no le gaste el código a la persona.
create function public.verificar_codigo_anfitrion(p_correo text, p_codigo text)
returns uuid
language plpgsql
volatile
security definer
set search_path = ''
as $$
declare
  v_fila private.anfitriones_activacion%rowtype;
  v_usuario uuid;
begin
  select * into v_fila from private.anfitriones_activacion where correo = p_correo for update;
  if not found or v_fila.vence <= now() or v_fila.intentos >= private.activacion_intentos_maximos() then
    return null;
  end if;

  select auth_user_id into v_usuario from public.anfitriones where correo = p_correo and activo;
  if v_usuario is null then
    return null;
  end if;

  if extensions.crypt(coalesce(p_codigo, ''), v_fila.codigo_hash) <> v_fila.codigo_hash then
    update private.anfitriones_activacion set intentos = intentos + 1 where correo = p_correo;
    return null;
  end if;
  return v_usuario;
end;
$$;

-- (7) Consume el código tras guardar la contraseña nueva.
create function public.consumir_codigo_anfitrion(p_correo text)
returns void
language plpgsql
security definer
set search_path = ''
as $$
begin
  delete from private.anfitriones_activacion where correo = p_correo;
  insert into private.bitacora_anfitriones (correo, accion, hecho_por) values (p_correo, 'activar', p_correo);
end;
$$;

-- (8) Habilita o deshabilita. Al deshabilitar se borra el código pendiente.
-- Devuelve la cuenta de Auth para que la Edge Function la bloquee o
-- desbloquee.
create function public.cambiar_estado_anfitrion(p_correo text, p_activo boolean, p_hecho_por text)
returns uuid
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_usuario uuid;
begin
  update public.anfitriones set activo = p_activo where correo = p_correo returning auth_user_id into v_usuario;
  if not found then
    raise exception 'Anfitrión inexistente' using errcode = 'P0002';
  end if;
  if not p_activo then
    delete from private.anfitriones_activacion where correo = p_correo;
  end if;
  insert into private.bitacora_anfitriones (correo, accion, hecho_por)
  values (p_correo, case when p_activo then 'habilitar' else 'deshabilitar' end, p_hecho_por);
  return v_usuario;
end;
$$;

-- (9) Cierra todas las sesiones de una cuenta (los refresh tokens caen en
-- cascada). El access token vigente dura hasta 1 h, pero las funciones de
-- escritura ya revisan `activo` en cada llamada.
create function public.cerrar_sesiones_de_cuenta(p_usuario uuid)
returns void
language sql
security definer
set search_path = ''
as $$
  delete from auth.sessions where user_id = p_usuario;
$$;

-- (10) Cuenta de Auth existente con ese correo (para enlazar anfitriones que
-- ya entraban con Google, o reintentos de un alta a medias).
create function public.cuenta_auth_por_correo(p_correo text)
returns uuid
language sql
stable
security definer
set search_path = ''
as $$
  select id from auth.users where lower(email) = lower(p_correo) limit 1;
$$;

-- (11) Listado para el panel: estado de cada cuenta, sin exponer hashes.
create function public.panel_anfitriones()
returns table (
  correo text,
  nombre text,
  activo boolean,
  estado text,
  codigo_vence timestamptz,
  creado_en timestamptz
)
language plpgsql
stable
security definer
set search_path = ''
as $$
begin
  if not private.es_admin_global() then
    raise exception 'Solo administradores del panel' using errcode = '42501';
  end if;
  return query
  select
    a.correo,
    a.nombre,
    a.activo,
    case
      when not a.activo then 'deshabilitada'
      when act.correo is not null and act.vence > now()
        and act.intentos < private.activacion_intentos_maximos() then 'pendiente'
      when act.correo is not null then 'codigo_vencido'
      when u.id is null then 'sin_cuenta'
      -- Entraba solo con Google: necesita un código para poner contraseña.
      when coalesce(u.encrypted_password, '') = '' then 'sin_contrasena'
      else 'activa'
    end,
    act.vence,
    a.creado_en
  from public.anfitriones a
  left join auth.users u on u.id = a.auth_user_id
  left join private.anfitriones_activacion act on act.correo = a.correo
  order by a.nombre;
end;
$$;

-- Permisos: nada para anon/authenticated salvo el listado del panel.
revoke all on function public.emitir_codigo_anfitrion(text, text, text, text) from public, anon, authenticated;
revoke all on function public.verificar_codigo_anfitrion(text, text) from public, anon, authenticated;
revoke all on function public.consumir_codigo_anfitrion(text) from public, anon, authenticated;
revoke all on function public.cambiar_estado_anfitrion(text, boolean, text) from public, anon, authenticated;
revoke all on function public.cerrar_sesiones_de_cuenta(uuid) from public, anon, authenticated;
revoke all on function public.cuenta_auth_por_correo(text) from public, anon, authenticated;
grant execute on function public.emitir_codigo_anfitrion(text, text, text, text) to service_role;
grant execute on function public.verificar_codigo_anfitrion(text, text) to service_role;
grant execute on function public.consumir_codigo_anfitrion(text) to service_role;
grant execute on function public.cambiar_estado_anfitrion(text, boolean, text) to service_role;
grant execute on function public.cerrar_sesiones_de_cuenta(uuid) to service_role;
grant execute on function public.cuenta_auth_por_correo(text) to service_role;

revoke all on function public.panel_anfitriones() from public, anon;
grant execute on function public.panel_anfitriones() to authenticated;

revoke all on function private.activacion_vigencia() from public, anon, authenticated;
revoke all on function private.activacion_intentos_maximos() from public, anon, authenticated;
