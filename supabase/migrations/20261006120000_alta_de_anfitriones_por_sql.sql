-- Alta de anfitriones por SQL para la web de visitas con correo y
-- contraseña (sin Google). Ver
-- docs/auditorias/investigacion-login-correo-web-visitas-2026-10-06.md.
--
-- Quien administra solo escribe en `public.anfitriones`:
--
--   insert into public.anfitriones (correo, nombre)
--   values ('ana.mora@dominio.com', 'Ana Mora');            -- alta
--   update public.anfitriones set activo = false
--   where correo = 'ana.mora@dominio.com';                  -- baja
--
-- La cuenta de Supabase Auth (sin la cual no se puede mandar el código de
-- primer ingreso ni guardar una contraseña) la crea la Edge Function
-- `cuentas-anfitriones` con la API de administración: escribir directo en
-- `auth.users` no está soportado por Supabase y se rompe entre versiones de
-- GoTrue. Así el registro público de Auth puede quedar APAGADO.
--
-- 1. Un trigger por sentencia sobre `anfitriones` le pide a la función que
--    reconcilie. La función relee las tablas completas (mismo criterio que
--    `sync-access-policy`): un aviso perdido se corrige con el siguiente, y
--    `select private.sincronizar_cuentas_anfitriones();` la fuerza a mano.
-- 2. Nunca bloquea la escritura: `net.http_post` es asíncrono, y si faltan
--    los secretos de Vault solo deja un WARNING (la autorización real sigue
--    siendo esta tabla; sin cuenta de Auth la persona simplemente no puede
--    pedir su código todavía).
-- 3. La URL de la función vive en Vault, no en este archivo: staging y
--    producción usan la misma migración sin editarla (lección de
--    docs/recuperacion-sitio-staging.md con `sync_access_policy`).
-- 4. La función se autentica con un secreto propio que solo existe en
--    Vault; la verifica con `public.secreto_cuentas_anfitriones_valido`,
--    ejecutable únicamente por `service_role`. No hay secretos de Edge
--    Function que configurar aparte.
--
-- Paso manual por proyecto (nunca versionar los valores):
--
--   select vault.create_secret(
--     'https://<ref>.supabase.co/functions/v1/cuentas-anfitriones',
--     'cuentas_anfitriones_url');
--   select vault.create_secret(
--     encode(extensions.gen_random_bytes(32), 'hex'),
--     'cuentas_anfitriones_secreto');

-- (0) `auth.email()` siempre viene en minúsculas: un correo con mayúsculas
-- o espacios nunca coincidiría y la persona quedaría sin acceso sin aviso.
alter table public.anfitriones
  add constraint anfitriones_correo_normalizado
  check (correo = lower(btrim(correo)) and correo like '_%@_%._%');

comment on table public.anfitriones is
  'Quién puede agendar visitas en visitas.megabrisas.com. Alta: insert (correo en minúsculas); baja: activo = false. La cuenta de Supabase Auth la crea/bloquea la Edge Function cuentas-anfitriones (trigger anfitriones_sincronizar_cuentas). Esta tabla decide la autorización real.';

-- (1) Pedido de reconciliación a la Edge Function.
create function private.sincronizar_cuentas_anfitriones()
returns void
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_url text;
  v_secreto text;
begin
  select decrypted_secret into v_url
    from vault.decrypted_secrets where name = 'cuentas_anfitriones_url';
  select decrypted_secret into v_secreto
    from vault.decrypted_secrets where name = 'cuentas_anfitriones_secreto';

  if v_url is null or v_secreto is null then
    raise warning 'Faltan los secretos de Vault cuentas_anfitriones_url / cuentas_anfitriones_secreto: las cuentas de Auth de anfitriones no se sincronizaron (ver migración alta_de_anfitriones_por_sql)';
    return;
  end if;

  perform net.http_post(
    url := v_url,
    headers := jsonb_build_object(
      'Content-Type', 'application/json',
      'x-webhook-secret', v_secreto
    ),
    body := '{}'::jsonb,
    timeout_milliseconds := 15000
  );
end;
$$;

revoke all on function private.sincronizar_cuentas_anfitriones() from public, anon, authenticated;

create function private.sincronizar_cuentas_anfitriones_al_cambiar()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
begin
  perform private.sincronizar_cuentas_anfitriones();
  return null;
end;
$$;

revoke all on function private.sincronizar_cuentas_anfitriones_al_cambiar() from public, anon, authenticated;

create trigger anfitriones_sincronizar_cuentas
  after insert or delete or update of correo, activo on public.anfitriones
  for each statement
  execute function private.sincronizar_cuentas_anfitriones_al_cambiar();

-- (2) Verificación del secreto desde la Edge Function (con service role).
-- Exige un mínimo de longitud para que un secreto mal cargado (vacío,
-- "test") no deje la puerta abierta.
create function public.secreto_cuentas_anfitriones_valido(p_secreto text)
returns boolean
language sql
stable
security definer
set search_path = ''
as $$
  select coalesce(length(p_secreto) >= 32, false) and exists (
    select 1 from vault.decrypted_secrets
    where name = 'cuentas_anfitriones_secreto'
      and length(decrypted_secret) >= 32
      and decrypted_secret = p_secreto
  );
$$;

revoke all on function public.secreto_cuentas_anfitriones_valido(text) from public, anon, authenticated;
grant execute on function public.secreto_cuentas_anfitriones_valido(text) to service_role;
