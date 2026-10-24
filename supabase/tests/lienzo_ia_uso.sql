-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
-- Cubre `lienzo_ia_uso`: la Edge Function (service_role) inserta; cada
-- administrador del panel lee sólo sus filas; nadie más lee ni escribe; los
-- límites de la tabla.
begin;

select set_config('diagnostico.correo_admin', 'diagnostico-admin@example.com', true),
       set_config('diagnostico.correo_otro', 'diagnostico-otro-admin@example.com', true),
       set_config('diagnostico.correo_normal', 'diagnostico-normal@example.com', true);

insert into public.administradores_panel (correo) values
  (current_setting('diagnostico.correo_admin')),
  (current_setting('diagnostico.correo_otro'));

-- La Edge Function escribe con service_role.
set local role service_role;
insert into public.lienzo_ia_uso (correo, pregunta, resultado, lienzo, respuesta, modelo,
                                  tokens_entrada, tokens_salida, costo_usd)
values
  (current_setting('diagnostico.correo_admin'), 'PRAIND vencidos por empresa', 'editado',
   '{"fuente": "contratistas", "filtros": [], "widgets": []}', 'Listo.', 'claude-haiku-5-5', 1200, 300, 0.00027),
  (current_setting('diagnostico.correo_otro'), 'hola', 'respondido', null, '¿Qué quiere ver?', 'claude-haiku-5-5', 900, 20, 0.0001);
reset role;

-- Límites de la tabla.
do $$
begin
  begin
    insert into public.lienzo_ia_uso (correo, pregunta, resultado) values ('x@example.com', '', 'editado');
    raise exception 'Aceptó una pregunta vacía';
  exception when check_violation then null; end;
  begin
    insert into public.lienzo_ia_uso (correo, pregunta, resultado) values ('x@example.com', repeat('a', 2001), 'editado');
    raise exception 'Aceptó una pregunta de más de 2000 caracteres';
  exception when check_violation then null; end;
  begin
    insert into public.lienzo_ia_uso (correo, pregunta, resultado) values ('x@example.com', 'p', 'inventado');
    raise exception 'Aceptó un resultado fuera de la lista';
  exception when check_violation then null; end;
  begin
    insert into public.lienzo_ia_uso (correo, pregunta, resultado, reintentos) values ('x@example.com', 'p', 'editado', 2);
    raise exception 'Aceptó más de un reintento';
  exception when check_violation then null; end;
  begin
    insert into public.lienzo_ia_uso (correo, pregunta, resultado, costo_usd) values ('x@example.com', 'p', 'editado', -1);
    raise exception 'Aceptó un costo negativo';
  exception when check_violation then null; end;
end $$;

-- Un administrador lee sólo sus consultas, y no puede escribir.
set local role authenticated;
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_admin'))::text, true);
do $$
declare
  v_preguntas text;
begin
  select string_agg(pregunta, ', ') into v_preguntas from public.lienzo_ia_uso
   where correo like 'diagnostico-%';
  if v_preguntas is distinct from 'PRAIND vencidos por empresa' then
    raise exception 'Un admin ve consultas que no son suyas: %', v_preguntas;
  end if;

  begin
    insert into public.lienzo_ia_uso (correo, pregunta, resultado)
    values (current_setting('diagnostico.correo_admin'), 'p', 'editado');
    raise exception 'Un admin pudo escribir en el registro';
  exception when insufficient_privilege then null; end;

  begin
    update public.lienzo_ia_uso set respuesta = 'cambiada' where correo = current_setting('diagnostico.correo_admin');
    raise exception 'Un admin pudo modificar el registro';
  exception when insufficient_privilege then null; end;

  begin
    delete from public.lienzo_ia_uso;
    raise exception 'Un admin pudo borrar el registro';
  exception when insufficient_privilege then null; end;
end $$;

-- Quien no es administrador no ve nada, aunque el correo coincida con una fila.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_normal'))::text, true);
reset role;
set local role service_role;
insert into public.lienzo_ia_uso (correo, pregunta, resultado)
values (current_setting('diagnostico.correo_normal'), 'ya no es admin', 'editado');
reset role;
set local role authenticated;
do $$
begin
  if exists (select 1 from public.lienzo_ia_uso where correo like 'diagnostico-%') then
    raise exception 'Quien no es admin ve el registro del asistente';
  end if;
end $$;

-- anon no tiene ningún permiso; service_role no puede modificar ni borrar (sólo agrega).
reset role;
do $$
begin
  if has_table_privilege('anon', 'public.lienzo_ia_uso', 'select')
     or has_table_privilege('anon', 'public.lienzo_ia_uso', 'insert') then
    raise exception 'anon tiene permisos sobre lienzo_ia_uso';
  end if;
  if has_table_privilege('service_role', 'public.lienzo_ia_uso', 'update')
     or has_table_privilege('service_role', 'public.lienzo_ia_uso', 'delete') then
    raise exception 'service_role puede modificar o borrar el registro';
  end if;
end $$;
set local role anon;
do $$
begin
  perform 1 from public.lienzo_ia_uso limit 1;
  raise exception 'anon pudo leer lienzo_ia_uso';
exception
  when insufficient_privilege then null;
end $$;

rollback;
