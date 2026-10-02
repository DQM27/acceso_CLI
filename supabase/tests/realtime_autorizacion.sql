-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
--
-- Canal en vivo `sitio:<id>` (`realtime.messages`):
--   1. un dispositivo vigente entra al canal de su sitio;
--   2. no entra al de otro sitio;
--   3. una sesión sin sitio no entra;
--   4. un cliente anónimo no entra;
--   5. un dispositivo retirado no entra ni al de su propio sitio
--      (`20260930020000_realtime_solo_dispositivos_vigentes`).
begin;

insert into public.sitios (id, nombre) values
  (gen_random_uuid(), 'Diagnóstico RT A'),
  (gen_random_uuid(), 'Diagnóstico RT B');
select set_config('diagnostico.sitio_a', (select id::text from public.sitios where nombre = 'Diagnóstico RT A'), true),
       set_config('diagnostico.sitio_b', (select id::text from public.sitios where nombre = 'Diagnóstico RT B'), true);

insert into public.dispositivos (id, sitio_id, tipo, etiqueta, clave_publica_jwk, clave_huella) values
  (gen_random_uuid(), current_setting('diagnostico.sitio_a')::uuid, 'pc', 'Diagnóstico RT PC A', '{"kty":"EC"}', 'diag-huella-rt-a'),
  (gen_random_uuid(), current_setting('diagnostico.sitio_b')::uuid, 'pc', 'Diagnóstico RT PC B', '{"kty":"EC"}', 'diag-huella-rt-b');
select set_config('diagnostico.disp_a', (select id::text from public.dispositivos where etiqueta = 'Diagnóstico RT PC A'), true),
       set_config('diagnostico.disp_b', (select id::text from public.dispositivos where etiqueta = 'Diagnóstico RT PC B'), true);

select set_config('diagnostico.realtime_id', gen_random_uuid()::text, true);
insert into realtime.messages (id, topic, extension)
values (current_setting('diagnostico.realtime_id')::uuid,
        'sitio:' || current_setting('diagnostico.sitio_a'), 'broadcast');

create temporary table jwt_prueba on commit drop as
select json_build_object('role', 'authenticated', 'sub', current_setting('diagnostico.disp_a'),
         'huella', 'diag-huella-rt-a', 'sitio_id', current_setting('diagnostico.sitio_a'))::text as a,
       json_build_object('role', 'authenticated', 'sub', current_setting('diagnostico.disp_b'),
         'huella', 'diag-huella-rt-b', 'sitio_id', current_setting('diagnostico.sitio_b'))::text as b;
grant select on jwt_prueba to authenticated;

set local role authenticated;
select set_config('request.jwt.claim', '', true),
       set_config('request.jwt.claims', (select a from jwt_prueba), true),
       set_config('realtime.topic', 'sitio:' || current_setting('diagnostico.sitio_a'), true);
do $$
begin
  if (select count(*) from realtime.messages
      where id = current_setting('diagnostico.realtime_id')::uuid) <> 1 then
    raise exception 'El dispositivo no puede autorizar su canal';
  end if;
end $$;

select set_config('request.jwt.claims', (select b from jwt_prueba), true);
do $$
begin
  if exists (select 1 from realtime.messages
             where id = current_setting('diagnostico.realtime_id')::uuid) then
    raise exception 'Un dispositivo puede entrar al canal de otro sitio';
  end if;
end $$;

select set_config('request.jwt.claims', '{"role":"authenticated"}', true);
do $$
begin
  if exists (select 1 from realtime.messages
             where id = current_setting('diagnostico.realtime_id')::uuid) then
    raise exception 'Una sesión sin sitio puede entrar al canal';
  end if;
end $$;

set local role anon;
select set_config('request.jwt.claims',
  json_build_object('role', 'anon', 'sitio_id', current_setting('diagnostico.sitio_a'))::text, true);
do $$
begin
  if exists (select 1 from realtime.messages
             where id = current_setting('diagnostico.realtime_id')::uuid) then
    raise exception 'Un cliente anónimo puede entrar al canal';
  end if;
end $$;

reset role;
update public.dispositivos set revoked_at = now()
 where id = current_setting('diagnostico.disp_a')::uuid;
set local role authenticated;
select set_config('request.jwt.claims', (select a from jwt_prueba), true);
do $$
begin
  if exists (select 1 from realtime.messages
             where id = current_setting('diagnostico.realtime_id')::uuid) then
    raise exception 'Un dispositivo retirado puede entrar al canal de su sitio';
  end if;
end $$;

select '5 comprobaciones de autorización correctas' as resultado;
rollback;
