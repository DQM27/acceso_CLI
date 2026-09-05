-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
begin;
select set_config('diagnostico.realtime_id', gen_random_uuid()::text, true);
insert into realtime.messages (id, topic, extension)
values (current_setting('diagnostico.realtime_id')::uuid, 'sitio:prueba-a', 'broadcast');

set local role authenticated;
select set_config('request.jwt.claim', '', true),
       set_config('request.jwt.claims', '{"role":"authenticated","sitio_id":"prueba-a"}', true),
       set_config('realtime.topic', 'sitio:prueba-a', true);
do $$
begin
  if (select count(*) from realtime.messages
      where id = current_setting('diagnostico.realtime_id')::uuid) <> 1 then
    raise exception 'El dispositivo no puede autorizar su canal';
  end if;
end $$;

select set_config('request.jwt.claims', '{"role":"authenticated","sitio_id":"prueba-b"}', true);
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
select set_config('request.jwt.claims', '{"role":"anon","sitio_id":"prueba-a"}', true);
do $$
begin
  if exists (select 1 from realtime.messages
             where id = current_setting('diagnostico.realtime_id')::uuid) then
    raise exception 'Un cliente anónimo puede entrar al canal';
  end if;
end $$;
select '4 comprobaciones de autorización correctas' as resultado;
rollback;
