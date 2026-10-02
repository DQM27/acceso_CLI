-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
--
-- Cubre las migraciones revocacion_efectiva_dispositivos y
-- vinculacion_dispositivos_por_codigo. Un dispositivo sólo tiene dos
-- estados: vinculado o retirado.
--   1-3. La política restrictiva "solo dispositivos vigentes": un token sólo
--        sirve con la huella de la clave registrada del dispositivo; las
--        sesiones humanas no se ven afectadas.
--   4-8. `canjear_codigo_vinculacion` es de un solo uso, respeta la
--        vigencia, sólo vincula un dispositivo nuevo (sin clave) y ata la
--        clave pública.
--   9.   Retirado: pierde todo acceso con el MISMO token.
--   10.  Clientes (anon/authenticated) no pueden llamar al canje ni leer
--        códigos o eventos.
begin;

insert into public.sitios (id, nombre) values (gen_random_uuid(), 'Diagnóstico Vigencia');
select set_config('diagnostico.sitio', (select id::text from public.sitios where nombre = 'Diagnóstico Vigencia'), true);

-- A: ya vinculado. B: recién dado de alta, todavía sin clave.
insert into public.dispositivos (id, sitio_id, tipo, etiqueta, clave_publica_jwk, clave_huella) values
  (gen_random_uuid(), current_setting('diagnostico.sitio')::uuid, 'pc', 'Diagnóstico Vigente A',
   '{"kty":"EC","crv":"P-256","x":"inicial","y":"inicial"}', 'huella-inicial'),
  (gen_random_uuid(), current_setting('diagnostico.sitio')::uuid, 'pc', 'Diagnóstico Nuevo B', null, null);
select set_config('diagnostico.dispositivo',
         (select id::text from public.dispositivos where etiqueta = 'Diagnóstico Vigente A'), true),
       set_config('diagnostico.nuevo',
         (select id::text from public.dispositivos where etiqueta = 'Diagnóstico Nuevo B'), true);

insert into public.gafetes (id, sitio_id, dispositivo_origen_id, numero, estado)
values (gen_random_uuid(), current_setting('diagnostico.sitio')::uuid,
        current_setting('diagnostico.dispositivo')::uuid, 987654, 'DISPONIBLE');

-- Claims del token de A, SIN `huella`: cada paso le agrega la que corresponda.
create temporary table jwt_base on commit drop as
select json_build_object(
  'role', 'authenticated',
  'sub', current_setting('diagnostico.dispositivo'),
  'sitio_id', current_setting('diagnostico.sitio'),
  'tipo', 'pc'
)::jsonb as claims;
grant select on jwt_base to authenticated;

-- 1. Vigente: lee los gafetes de su sitio.
set local role authenticated;
select set_config('request.jwt.claims',
  ((select claims from jwt_base) || '{"huella":"huella-inicial"}')::text, true);
do $$
begin
  if not exists (select 1 from public.gafetes where numero = 987654) then
    raise exception '1: un dispositivo vigente no puede leer los gafetes de su sitio';
  end if;
end $$;

-- 2. Un token con otra huella, o sin huella, no sirve.
select set_config('request.jwt.claims',
  ((select claims from jwt_base) || '{"huella":"huella-de-otra-clave"}')::text, true);
do $$
begin
  if exists (select 1 from public.gafetes where numero = 987654) then
    raise exception '2a: un token con otra huella puede leer';
  end if;
end $$;
select set_config('request.jwt.claims', (select claims from jwt_base)::text, true);
do $$
begin
  if exists (select 1 from public.gafetes where numero = 987654) then
    raise exception '2b: un token sin huella puede leer';
  end if;
end $$;

-- 3. Una sesión humana (sin sitio_id) no se ve afectada por la política.
--    Se evalúa fuera del rol `authenticated`: ese rol no tiene USAGE sobre
--    el esquema `private` (a propósito); las políticas sí pueden invocarla.
reset role;
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', 'diagnostico-humano@example.com')::text, true);
do $$
begin
  if not (select private.dispositivo_vigente()) then
    raise exception '3: dispositivo_vigente() rechaza una sesión sin sitio_id';
  end if;
end $$;

-- 4-8. Canje de código (como service_role, igual que la Edge Function).
reset role;
insert into public.codigos_vinculacion (dispositivo_id, codigo_hash, creado_por, expira_en)
values (current_setting('diagnostico.nuevo')::uuid, 'diag-codigo-ok', 'diag@example.com', now() + interval '15 minutes'),
       (current_setting('diagnostico.nuevo')::uuid, 'diag-codigo-vencido', 'diag@example.com', now() - interval '1 minute'),
       (current_setting('diagnostico.dispositivo')::uuid, 'diag-codigo-de-vinculado', 'diag@example.com', now() + interval '15 minutes');

set local role service_role;
do $$
declare
  v_filas int;
begin
  select count(*) into v_filas from public.canjear_codigo_vinculacion(
    'diag-codigo-vencido', '{"kty":"EC"}', 'huella-x', '{}', '127.0.0.1');
  if v_filas <> 0 then
    raise exception '4: un código vencido se pudo canjear';
  end if;

  select count(*) into v_filas from public.canjear_codigo_vinculacion(
    'diag-codigo-de-vinculado', '{"kty":"EC"}', 'huella-y', '{}', '127.0.0.1');
  if v_filas <> 0 then
    raise exception '5: un código vinculó a un dispositivo que ya tenía clave';
  end if;
  if exists (select 1 from public.codigos_vinculacion
             where codigo_hash = 'diag-codigo-de-vinculado' and usado_en is not null) then
    raise exception '5b: el código rechazado quedó consumido';
  end if;

  select count(*) into v_filas from public.canjear_codigo_vinculacion(
    'diag-codigo-ok', '{"kty":"EC","crv":"P-256","x":"a","y":"b"}', 'huella-canje',
    '{"nombre_dispositivo":"PC Diagnóstico"}', '127.0.0.1');
  if v_filas <> 1 then
    raise exception '6: un código vigente no se pudo canjear';
  end if;

  select count(*) into v_filas from public.canjear_codigo_vinculacion(
    'diag-codigo-ok', '{"kty":"EC"}', 'huella-otra', '{}', '127.0.0.1');
  if v_filas <> 0 then
    raise exception '7: el mismo código se pudo canjear dos veces';
  end if;

  if not exists (
    select 1 from public.dispositivos
     where id = current_setting('diagnostico.nuevo')::uuid
       and clave_huella = 'huella-canje'
       and nombre_dispositivo = 'PC Diagnóstico'
       and vinculado_en is not null
  ) then
    raise exception '8: el canje no ató la clave o no guardó la metadata';
  end if;
end $$;

-- 9. Retirado: pierde todo acceso con el MISMO token.
reset role;
update public.dispositivos set revoked_at = now() where id = current_setting('diagnostico.dispositivo')::uuid;
set local role authenticated;
select set_config('request.jwt.claims',
  ((select claims from jwt_base) || '{"huella":"huella-inicial"}')::text, true);
do $$
begin
  if exists (select 1 from public.gafetes where numero = 987654) then
    raise exception '9: un dispositivo retirado sigue leyendo con su token viejo';
  end if;
end $$;

-- 10. Ningún cliente llama al canje ni lee códigos/eventos.
reset role;
update public.dispositivos set revoked_at = null where id = current_setting('diagnostico.dispositivo')::uuid;
set local role authenticated;
select set_config('request.jwt.claims',
  ((select claims from jwt_base) || '{"huella":"huella-inicial"}')::text, true);
do $$
begin
  if exists (select 1 from public.codigos_vinculacion) then
    raise exception '10a: un dispositivo puede leer codigos_vinculacion';
  end if;
  if exists (select 1 from public.eventos_seguridad_dispositivos) then
    raise exception '10b: un dispositivo puede leer eventos_seguridad_dispositivos';
  end if;
  begin
    perform public.canjear_codigo_vinculacion('x', '{}', 'x', '{}', 'x');
    raise exception '10c: authenticated puede ejecutar canjear_codigo_vinculacion';
  exception when insufficient_privilege then
    null;
  end;
end $$;

select 'comprobaciones de vigencia y vinculación correctas' as resultado;
rollback;
