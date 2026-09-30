-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
--
-- Cubre las migraciones revocacion_efectiva_dispositivos y
-- vinculacion_dispositivos_por_codigo:
--   1-4. La política restrictiva "solo dispositivos vigentes" corta al
--        instante a un dispositivo suspendido, re-vinculado (token con
--        `huella` vieja), sin vincular o con un token sin `huella`, sin
--        tocar sesiones humanas.
--   5-8. `canjear_codigo_vinculacion` es de un solo uso, respeta la
--        vigencia y ata la clave pública.
--   9.   Clientes (anon/authenticated) no pueden llamar al canje ni leer
--        códigos o eventos.
begin;

insert into public.sitios (id, nombre) values (gen_random_uuid(), 'Diagnóstico Vigencia');
select set_config('diagnostico.sitio', (select id::text from public.sitios where nombre = 'Diagnóstico Vigencia'), true);

insert into public.dispositivos (id, sitio_id, tipo, etiqueta, clave_publica_jwk, clave_huella)
values (gen_random_uuid(), current_setting('diagnostico.sitio')::uuid, 'pc', 'Diagnóstico Vigente',
        '{"kty":"EC","crv":"P-256","x":"inicial","y":"inicial"}', 'huella-inicial');
select set_config('diagnostico.dispositivo',
  (select id::text from public.dispositivos where etiqueta = 'Diagnóstico Vigente'), true);

insert into public.gafetes (id, sitio_id, dispositivo_origen_id, numero, estado)
values (gen_random_uuid(), current_setting('diagnostico.sitio')::uuid,
        current_setting('diagnostico.dispositivo')::uuid, 987654, 'DISPONIBLE');

-- Claims del token de ese dispositivo, SIN `huella`: cada paso le agrega
-- la huella que corresponda.
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

-- 2. Suspendido: pierde todo acceso con el MISMO token.
reset role;
update public.dispositivos set suspended_at = now() where id = current_setting('diagnostico.dispositivo')::uuid;
set local role authenticated;
select set_config('request.jwt.claims',
  ((select claims from jwt_base) || '{"huella":"huella-inicial"}')::text, true);
do $$
begin
  if exists (select 1 from public.gafetes where numero = 987654) then
    raise exception '2: un dispositivo suspendido sigue leyendo con su token viejo';
  end if;
end $$;

-- 3. Re-vinculado: reactivado pero con clave nueva, el token de la clave
--    anterior queda inválido; uno con la huella nueva sí sirve, y uno sin
--    `huella` nunca.
reset role;
update public.dispositivos
   set suspended_at = null,
       clave_publica_jwk = '{"kty":"EC","crv":"P-256","x":"diag","y":"diag"}',
       clave_huella = 'huella-diagnostico'
 where id = current_setting('diagnostico.dispositivo')::uuid;
set local role authenticated;
select set_config('request.jwt.claims',
  ((select claims from jwt_base) || '{"huella":"huella-inicial"}')::text, true);
do $$
begin
  if exists (select 1 from public.gafetes where numero = 987654) then
    raise exception '3a: el token de la clave anterior sigue sirviendo tras re-vincular';
  end if;
end $$;
select set_config('request.jwt.claims',
  ((select claims from jwt_base) || '{"huella":"huella-diagnostico"}')::text, true);
do $$
begin
  if not exists (select 1 from public.gafetes where numero = 987654) then
    raise exception '3b: el token con la huella vigente no puede leer';
  end if;
end $$;
select set_config('request.jwt.claims', (select claims from jwt_base)::text, true);
do $$
begin
  if exists (select 1 from public.gafetes where numero = 987654) then
    raise exception '3c: un token sin huella puede leer';
  end if;
end $$;

-- 4. Una sesión humana (sin sitio_id) no se ve afectada por la política.
--    Se evalúa fuera del rol `authenticated`: ese rol no tiene USAGE sobre
--    el esquema `private` (a propósito); las políticas sí pueden invocarla.
reset role;
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', 'diagnostico-humano@example.com')::text, true);
do $$
begin
  if not (select private.dispositivo_vigente()) then
    raise exception '4: dispositivo_vigente() rechaza una sesión sin sitio_id';
  end if;
end $$;

-- 4b. Sin vincular (clave pendiente de un código nuevo): ningún token sirve.
reset role;
update public.dispositivos
   set clave_publica_jwk = null, clave_huella = null
 where id = current_setting('diagnostico.dispositivo')::uuid;
set local role authenticated;
select set_config('request.jwt.claims',
  ((select claims from jwt_base) || '{"huella":"huella-diagnostico"}')::text, true);
do $$
begin
  if exists (select 1 from public.gafetes where numero = 987654) then
    raise exception '4b: un dispositivo sin vincular sigue leyendo con un token viejo';
  end if;
end $$;

-- 5-8. Canje de código (como service_role, igual que la Edge Function).
reset role;
insert into public.codigos_vinculacion (dispositivo_id, codigo_hash, creado_por, expira_en)
values (current_setting('diagnostico.dispositivo')::uuid, 'diag-codigo-ok', 'diag@example.com', now() + interval '15 minutes'),
       (current_setting('diagnostico.dispositivo')::uuid, 'diag-codigo-vencido', 'diag@example.com', now() - interval '1 minute');

set local role service_role;
do $$
declare
  v_filas int;
begin
  select count(*) into v_filas from public.canjear_codigo_vinculacion(
    'diag-codigo-vencido', '{"kty":"EC"}', 'huella-x', '{}', '127.0.0.1');
  if v_filas <> 0 then
    raise exception '5: un código vencido se pudo canjear';
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
     where id = current_setting('diagnostico.dispositivo')::uuid
       and clave_huella = 'huella-canje'
       and nombre_dispositivo = 'PC Diagnóstico'
       and vinculado_en is not null
  ) then
    raise exception '8: el canje no ató la clave o no guardó la metadata';
  end if;
end $$;

-- 9. Ningún cliente llama al canje ni lee códigos/eventos.
reset role;
set local role authenticated;
select set_config('request.jwt.claims',
  ((select claims from jwt_base) || '{"huella":"huella-canje"}')::text, true);
do $$
begin
  if exists (select 1 from public.codigos_vinculacion) then
    raise exception '9a: un dispositivo puede leer codigos_vinculacion';
  end if;
  if exists (select 1 from public.eventos_seguridad_dispositivos) then
    raise exception '9b: un dispositivo puede leer eventos_seguridad_dispositivos';
  end if;
  begin
    perform public.canjear_codigo_vinculacion('x', '{}', 'x', '{}', 'x');
    raise exception '9c: authenticated puede ejecutar canjear_codigo_vinculacion';
  exception when insufficient_privilege then
    null;
  end;
end $$;

select 'comprobaciones de vigencia y vinculación correctas' as resultado;
rollback;
