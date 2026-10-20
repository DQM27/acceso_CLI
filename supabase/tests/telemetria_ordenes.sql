-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
-- Cubre `telemetria_ordenes` y su historial: el administrador del panel crea y
-- cambia órdenes; cada equipo lee SÓLO la suya (no la de otro equipo ni la de
-- otro sitio); un equipo no escribe, no borra y no lee el historial; un correo
-- que no es administrador no ve nada; anon no toca; la versión, el autor y la
-- hora los pone el servidor (el cliente no puede falsearlos); los CHECK
-- rechazan un grupo desconocido (incluido `base`) y una nota larga; cada
-- cambio queda en el historial y sale por Realtime al canal del sitio; y un
-- equipo retirado deja de leer.
begin;

select set_config('diagnostico.correo_admin', 'diagnostico-admin-telemetria@example.com', true),
       set_config('diagnostico.correo_normal', 'diagnostico-normal-telemetria@example.com', true);

insert into public.administradores_panel (correo)
values (current_setting('diagnostico.correo_admin'));

insert into public.sitios (id, nombre) values
  ('aaaaaaaa-0000-0000-0000-00000000a601', 'Unidad telemetria A'),
  ('aaaaaaaa-0000-0000-0000-00000000a602', 'Unidad telemetria B');

insert into public.dispositivos (id, sitio_id, tipo, etiqueta, clave_publica_jwk, clave_huella, revoked_at) values
  ('cccccccc-0000-0000-0000-00000000c601', 'aaaaaaaa-0000-0000-0000-00000000a601', 'mobile', 'Celular telemetria A1', '{"kty":"EC"}', 'huella-tel-a1', null),
  ('cccccccc-0000-0000-0000-00000000c602', 'aaaaaaaa-0000-0000-0000-00000000a601', 'mobile', 'Celular telemetria A2', '{"kty":"EC"}', 'huella-tel-a2', null),
  ('cccccccc-0000-0000-0000-00000000c603', 'aaaaaaaa-0000-0000-0000-00000000a602', 'mobile', 'Celular telemetria B1', '{"kty":"EC"}', 'huella-tel-b1', null);

-- Lo que hace el administrador del panel (sesión humana con su correo).
set local role authenticated;
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_admin'))::text,
  true);

-- 1. El administrador crea las órdenes de A1 y A2, y de B1. Intenta falsear la
--    versión, el autor y la hora: el servidor los reemplaza.
insert into public.telemetria_ordenes (dispositivo_id, grupos, vence_en, nota, version, actualizado_por, actualizado_en)
values
  ('cccccccc-0000-0000-0000-00000000c601', '{rendimiento,fluidez}', now() + interval '2 days', 'lentitud reportada', 999999, 'otro@example.com', '2000-01-01'),
  ('cccccccc-0000-0000-0000-00000000c602', '{rendimiento}', null, '', 0, '', now()),
  ('cccccccc-0000-0000-0000-00000000c603', '{recursos}', null, 'otro sitio', 0, '', now());

do $$
declare
  v record;
begin
  select * into v from public.telemetria_ordenes where dispositivo_id = 'cccccccc-0000-0000-0000-00000000c601';
  if v.version = 999999 or v.version < 1 then
    raise exception '1: el cliente pudo falsear la versión (%)', v.version;
  end if;
  if v.actualizado_por <> current_setting('diagnostico.correo_admin') then
    raise exception '1: el autor no es el correo de la sesión (%)', v.actualizado_por;
  end if;
  if v.actualizado_en < now() - interval '1 minute' then
    raise exception '1: el cliente pudo falsear la hora';
  end if;
end $$;

-- 2. Cada cambio sube la versión (nunca baja ni se repite).
do $$
declare
  v1 bigint;
  v2 bigint;
begin
  select version into v1 from public.telemetria_ordenes where dispositivo_id = 'cccccccc-0000-0000-0000-00000000c601';
  update public.telemetria_ordenes set grupos = '{rendimiento,fluidez,recursos}'
   where dispositivo_id = 'cccccccc-0000-0000-0000-00000000c601';
  select version into v2 from public.telemetria_ordenes where dispositivo_id = 'cccccccc-0000-0000-0000-00000000c601';
  if v2 <= v1 then
    raise exception '2: la versión no subió (% -> %)', v1, v2;
  end if;
end $$;

-- 3. Los CHECK: grupo desconocido, `base` (no se guarda) y nota larga.
do $$
begin
  begin
    update public.telemetria_ordenes set grupos = '{rendimiento,inventado}'
     where dispositivo_id = 'cccccccc-0000-0000-0000-00000000c602';
    raise exception '3: se aceptó un grupo desconocido';
  exception when check_violation then null;
  end;
  begin
    update public.telemetria_ordenes set grupos = '{base}'
     where dispositivo_id = 'cccccccc-0000-0000-0000-00000000c602';
    raise exception '3: se aceptó `base`, que no se guarda';
  exception when check_violation then null;
  end;
  begin
    update public.telemetria_ordenes set nota = pg_catalog.repeat('x', 201)
     where dispositivo_id = 'cccccccc-0000-0000-0000-00000000c602';
    raise exception '3: se aceptó una nota de más de 200 caracteres';
  exception when check_violation then null;
  end;
end $$;

-- 4. El administrador lee todas las órdenes y el historial; no puede borrar.
do $$
begin
  if (select count(*) from public.telemetria_ordenes
       where dispositivo_id in ('cccccccc-0000-0000-0000-00000000c601', 'cccccccc-0000-0000-0000-00000000c602', 'cccccccc-0000-0000-0000-00000000c603')) <> 3 then
    raise exception '4: el administrador no ve las 3 órdenes';
  end if;
  -- A1 tiene 2 cambios (alta y actualización), A2 uno, B1 uno.
  if (select count(*) from public.telemetria_ordenes_historial
       where dispositivo_id = 'cccccccc-0000-0000-0000-00000000c601') <> 2 then
    raise exception '4: el historial de A1 no tiene los 2 cambios';
  end if;
  if (select cambiado_por from public.telemetria_ordenes_historial
       where dispositivo_id = 'cccccccc-0000-0000-0000-00000000c601' order by id limit 1)
     <> current_setting('diagnostico.correo_admin') then
    raise exception '4: el historial no guardó el autor';
  end if;
end $$;
do $$
begin
  delete from public.telemetria_ordenes where dispositivo_id = 'cccccccc-0000-0000-0000-00000000c602';
  raise exception '4: el administrador pudo borrar una orden';
exception when insufficient_privilege then null;
end $$;
do $$
begin
  update public.telemetria_ordenes_historial set nota = 'falsificada';
  raise exception '4: el administrador pudo modificar el historial';
exception when insufficient_privilege then null;
end $$;

-- 5. Cada cambio sale por Realtime al canal del sitio, con la versión y sin la
--    nota. (`realtime.messages` guarda los avisos enviados con `realtime.send`.)
reset role;
do $$
declare
  n integer;
begin
  select count(*) into n from realtime.messages
   where topic = 'sitio:aaaaaaaa-0000-0000-0000-00000000a601'
     and event = 'telemetria_config'
     and payload ->> 'dispositivo_id' = 'cccccccc-0000-0000-0000-00000000c601';
  if n <> 2 then
    raise exception '5: se esperaban 2 avisos de A1 en el canal del sitio A, hay %', n;
  end if;
  if exists (select 1 from realtime.messages
              where event = 'telemetria_config' and payload ? 'nota') then
    raise exception '5: el aviso lleva la nota';
  end if;
end $$;
set local role authenticated;

-- 6. Un equipo lee SÓLO su orden.
select set_config('request.jwt.claims', json_build_object(
  'role', 'authenticated', 'sub', 'cccccccc-0000-0000-0000-00000000c601',
  'sitio_id', 'aaaaaaaa-0000-0000-0000-00000000a601', 'huella', 'huella-tel-a1')::text, true);
do $$
begin
  if (select count(*) from public.telemetria_ordenes) <> 1 then
    raise exception '6: A1 debe ver exactamente 1 orden, ve %', (select count(*) from public.telemetria_ordenes);
  end if;
  if not exists (select 1 from public.telemetria_ordenes where dispositivo_id = 'cccccccc-0000-0000-0000-00000000c601') then
    raise exception '6: A1 no ve su propia orden';
  end if;
  if exists (select 1 from public.telemetria_ordenes where dispositivo_id <> 'cccccccc-0000-0000-0000-00000000c601') then
    raise exception '6: A1 ve la orden de otro equipo';
  end if;
end $$;

-- 7. Un equipo no escribe, no borra y no lee el historial.
do $$
begin
  update public.telemetria_ordenes set grupos = '{depuracion}' where dispositivo_id = 'cccccccc-0000-0000-0000-00000000c601';
  if found then
    raise exception '7: un equipo pudo cambiar su orden';
  end if;
exception when insufficient_privilege then null;
end $$;
do $$
begin
  insert into public.telemetria_ordenes (dispositivo_id, grupos)
  values ('cccccccc-0000-0000-0000-00000000c601', '{depuracion}')
  on conflict (dispositivo_id) do nothing;
  raise exception '7: un equipo pudo escribir una orden';
exception when insufficient_privilege then null;
end $$;
do $$
begin
  delete from public.telemetria_ordenes;
  raise exception '7: un equipo pudo borrar órdenes';
exception when insufficient_privilege then null;
end $$;
do $$
begin
  if exists (select 1 from public.telemetria_ordenes_historial) then
    raise exception '7: un equipo pudo leer el historial';
  end if;
end $$;

-- 8. Un equipo sin orden no ve nada (A3 no tiene fila): se usa el mismo JWT con
--    el id de un equipo que no existe en la tabla.
select set_config('request.jwt.claims', json_build_object(
  'role', 'authenticated', 'sub', 'cccccccc-0000-0000-0000-00000000c699',
  'sitio_id', 'aaaaaaaa-0000-0000-0000-00000000a601', 'huella', 'huella-no-existe')::text, true);
do $$
begin
  if exists (select 1 from public.telemetria_ordenes) then
    raise exception '8: un equipo sin fila vio órdenes de otros';
  end if;
end $$;

-- 9. Un correo que no es administrador no ve nada ni escribe.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_normal'))::text,
  true);
do $$
begin
  if exists (select 1 from public.telemetria_ordenes) or exists (select 1 from public.telemetria_ordenes_historial) then
    raise exception '9: un correo no administrador pudo leer telemetría';
  end if;
end $$;
do $$
begin
  insert into public.telemetria_ordenes (dispositivo_id, grupos)
  values ('cccccccc-0000-0000-0000-00000000c602', '{depuracion}');
  raise exception '9: un correo no administrador pudo crear una orden';
exception
  -- La política WITH CHECK también se reporta como insufficient_privilege.
  when insufficient_privilege then null;
end $$;
do $$
begin
  update public.telemetria_ordenes set grupos = '{depuracion}';
  if found then
    raise exception '9: un correo no administrador pudo cambiar una orden';
  end if;
end $$;

-- 10. Un equipo retirado deja de leer su orden (política "solo dispositivos vigentes").
reset role;
update public.dispositivos set revoked_at = now() where id = 'cccccccc-0000-0000-0000-00000000c601';
set local role authenticated;
select set_config('request.jwt.claims', json_build_object(
  'role', 'authenticated', 'sub', 'cccccccc-0000-0000-0000-00000000c601',
  'sitio_id', 'aaaaaaaa-0000-0000-0000-00000000a601', 'huella', 'huella-tel-a1')::text, true);
do $$
begin
  if exists (select 1 from public.telemetria_ordenes) then
    raise exception '10: un equipo retirado pudo leer su orden';
  end if;
end $$;

-- 11. anon no toca nada.
set local role anon;
do $$
begin
  perform 1 from public.telemetria_ordenes limit 1;
  raise exception '11: anon pudo leer telemetria_ordenes';
exception when insufficient_privilege then null;
end $$;
do $$
begin
  perform 1 from public.telemetria_ordenes_historial limit 1;
  raise exception '11: anon pudo leer el historial';
exception when insufficient_privilege then null;
end $$;

rollback;
