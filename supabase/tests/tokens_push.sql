-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
-- Cubre `registrar_token_push` y la tabla `tokens_push`: un equipo vigente
-- registra y actualiza su token, el mismo token pasa de un equipo a otro sin
-- duplicarse, un equipo retirado / una sesión sin equipo / anon no pueden
-- registrar, un token inválido se rechaza, y ningún equipo lee la tabla
-- directo (ni la propia fila: la lee sólo la Edge Function con service_role).
begin;

insert into public.sitios (id, nombre) values
  ('aaaaaaaa-0000-0000-0000-00000000a401', 'Unidad push A'),
  ('aaaaaaaa-0000-0000-0000-00000000a402', 'Unidad push B');

insert into public.dispositivos (id, sitio_id, tipo, etiqueta, clave_publica_jwk, clave_huella, revoked_at) values
  ('cccccccc-0000-0000-0000-00000000c401', 'aaaaaaaa-0000-0000-0000-00000000a401', 'mobile', 'Celular A1', '{"kty":"EC"}', 'huella-push-a1', null),
  ('cccccccc-0000-0000-0000-00000000c402', 'aaaaaaaa-0000-0000-0000-00000000a401', 'mobile', 'Celular A2', '{"kty":"EC"}', 'huella-push-a2', null),
  ('cccccccc-0000-0000-0000-00000000c403', 'aaaaaaaa-0000-0000-0000-00000000a402', 'mobile', 'Celular B retirado', '{"kty":"EC"}', 'huella-push-b', now());

-- Hace de equipo: JWT de dispositivo (sub, sitio_id, huella).
create or replace function pg_temp.como_equipo(p_id uuid, p_sitio uuid, p_huella text) returns void
language sql as $$
  select set_config('request.jwt.claims', json_build_object(
    'role', 'authenticated', 'sub', p_id, 'sitio_id', p_sitio, 'huella', p_huella)::text, true)
$$;

set local role authenticated;

do $$
declare
  token_1 text := 'token-fcm-de-prueba-numero-uno-0000000000';
  token_2 text := 'token-fcm-de-prueba-numero-dos-0000000000';
begin
  -- 1. Un equipo vigente registra su token (con espacios: se recortan).
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c401', 'aaaaaaaa-0000-0000-0000-00000000a401', 'huella-push-a1');
  perform public.registrar_token_push('  ' || token_1 || '  ');

  -- 2. Ni el propio equipo lee la tabla directo.
  if exists (select 1 from public.tokens_push) then
    raise exception '2: un equipo pudo leer tokens_push';
  end if;

  -- 3. FCM rotó el token: el mismo equipo lo actualiza (sigue siendo una fila).
  perform public.registrar_token_push(token_2);

  -- 4. Otro equipo registra el token que tenía el primero: se le quita al
  --    primero (un token no queda en dos equipos).
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c402', 'aaaaaaaa-0000-0000-0000-00000000a401', 'huella-push-a2');
  perform public.registrar_token_push(token_2);

  -- 5. Token inválido.
  begin
    perform public.registrar_token_push('corto');
    raise exception '5: se aceptó un token inválido';
  exception
    when invalid_parameter_value then null;
  end;
  begin
    perform public.registrar_token_push(null);
    raise exception '5: se aceptó un token nulo';
  exception
    when invalid_parameter_value then null;
  end;

  -- 6. Un equipo retirado no puede registrar.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c403', 'aaaaaaaa-0000-0000-0000-00000000a402', 'huella-push-b');
  begin
    perform public.registrar_token_push(token_1);
    raise exception '6: un equipo retirado pudo registrar un token';
  exception
    when insufficient_privilege then null;
  end;

  -- 7. Huella que no es la de la clave vigente (token de equipo viejo).
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c401', 'aaaaaaaa-0000-0000-0000-00000000a401', 'huella-vieja');
  begin
    perform public.registrar_token_push(token_1);
    raise exception '7: un JWT con huella vieja pudo registrar un token';
  exception
    when insufficient_privilege then null;
  end;

  -- 8. Un equipo no puede escribir la tabla directo.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c401', 'aaaaaaaa-0000-0000-0000-00000000a401', 'huella-push-a1');
  begin
    insert into public.tokens_push (dispositivo_id, sitio_id, token)
    values ('cccccccc-0000-0000-0000-00000000c401', 'aaaaaaaa-0000-0000-0000-00000000a401', token_1);
    raise exception '8: un equipo pudo insertar en tokens_push directo';
  exception
    when insufficient_privilege then null;
  end;
end $$;

-- Una sesión humana (sin sitio_id) no es un equipo: no registra.
select set_config('request.jwt.claims', json_build_object(
  'role', 'authenticated', 'sub', gen_random_uuid(), 'email', 'alguien@example.com')::text, true);
do $$
begin
  perform public.registrar_token_push('token-fcm-de-prueba-sin-equipo-000000000');
  raise exception 'Una sesión sin equipo pudo registrar un token';
exception
  when insufficient_privilege then null;
end $$;

-- Estado final, visto sin RLS: una sola fila, del equipo A2, con el token 2.
reset role;
do $$
declare
  n int;
begin
  select count(*) into n from public.tokens_push
   where dispositivo_id in ('cccccccc-0000-0000-0000-00000000c401', 'cccccccc-0000-0000-0000-00000000c402', 'cccccccc-0000-0000-0000-00000000c403');
  if n <> 1 then raise exception 'final: se esperaba 1 fila, hay %', n; end if;
  if not exists (
    select 1 from public.tokens_push
     where dispositivo_id = 'cccccccc-0000-0000-0000-00000000c402'
       and sitio_id = 'aaaaaaaa-0000-0000-0000-00000000a401'
       and token = 'token-fcm-de-prueba-numero-dos-0000000000'
  ) then
    raise exception 'final: el token 2 debería quedar sólo en el equipo A2';
  end if;
end $$;

-- anon no puede ejecutar la función ni leer la tabla.
set local role anon;
do $$
begin
  perform public.registrar_token_push('token-fcm-de-prueba-anonimo-00000000000');
  raise exception 'anon pudo ejecutar registrar_token_push';
exception
  when insufficient_privilege then null;
end $$;
do $$
begin
  perform 1 from public.tokens_push limit 1;
  raise exception 'anon pudo leer tokens_push';
exception
  when insufficient_privilege then null;
end $$;

rollback;
