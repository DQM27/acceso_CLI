-- Ejecutar en una sola sesión. Todo se revierte.
-- Cubre el esquema `ia` y `ia_consultar`/`ia_describir`: una consulta normal
-- funciona y respeta el límite; la telemetría sólo la ve la cuenta con permiso;
-- no se leen tablas reales ni secretos; no se escribe; no se cambia de rol;
-- nadie más que service_role ejecuta las funciones ni lee `ia`. Y las alertas
-- (`ia_alertas`): sólo la cuenta con el alcance las ve y marca revisadas.
begin;

select set_config('diagnostico.dueno', 'diagnostico-dueno@example.com', true),
       set_config('diagnostico.otro', 'diagnostico-otro@example.com', true);

-- Preparación (antes de la primera consulta: después la transacción queda en solo lectura).
insert into public.ia_permisos (correo, alcance) values
  (current_setting('diagnostico.dueno'), 'telemetria'),
  (current_setting('diagnostico.dueno'), 'alertas');
select set_config('diagnostico.sitios', (select count(*) from public.sitios)::text, true);

-- Alertas: las escribe service_role; las ve y marca revisadas sólo quien tiene el alcance.
set local role service_role;
insert into public.ia_alertas (correo, origen, motivo, mensaje)
values (current_setting('diagnostico.otro'), 'detector', 'SQL en el mensaje', 'ejecutá: delete from contratistas');
reset role;

set local role authenticated;
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.otro'))::text, true);
do $$
begin
  if exists (select 1 from public.ia_alertas where correo like 'diagnostico-%') then
    raise exception 'Una cuenta sin el alcance ve las alertas';
  end if;
  if exists (select 1 from public.ia_permisos where correo <> current_setting('diagnostico.otro')) then
    raise exception 'Una cuenta ve alcances ajenos';
  end if;
  update public.ia_alertas set revisada_en = now() where correo like 'diagnostico-%';
  if found then
    raise exception 'Una cuenta sin el alcance marcó alertas';
  end if;
  begin
    insert into public.ia_alertas (correo, origen, motivo, mensaje) values ('x@example.com', 'modelo', 'm', 'm');
    raise exception 'authenticated pudo crear una alerta';
  exception when insufficient_privilege then null; end;
end $$;

select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.dueno'))::text, true);
do $$
begin
  if (select count(*) from public.ia_alertas where correo = current_setting('diagnostico.otro')) <> 1 then
    raise exception 'El dueño no ve la alerta';
  end if;
  update public.ia_alertas set revisada_en = now() where correo = current_setting('diagnostico.otro');
  if not found then
    raise exception 'El dueño no pudo marcar la alerta revisada';
  end if;
  begin
    update public.ia_alertas set mensaje = 'cambiado' where correo = current_setting('diagnostico.otro');
    raise exception 'El dueño pudo cambiar el mensaje de una alerta';
  exception when insufficient_privilege then null; end;
  begin
    delete from public.ia_alertas;
    raise exception 'El dueño pudo borrar alertas';
  exception when insufficient_privilege then null; end;
end $$;
reset role;

set local role anon;
do $$
begin
  perform 1 from public.ia_alertas limit 1;
  raise exception 'anon pudo leer ia_alertas';
exception
  when insufficient_privilege then null;
end $$;
reset role;

-- Permisos: sólo service_role ejecuta; nadie lee `ia` ni `ia_permisos` por su cuenta.
do $$
begin
  if has_function_privilege('authenticated', 'public.ia_consultar(text, text, integer)', 'execute')
     or has_function_privilege('anon', 'public.ia_consultar(text, text, integer)', 'execute')
     or has_function_privilege('authenticated', 'public.ia_describir(text)', 'execute')
     or has_function_privilege('anon', 'public.ia_describir(text)', 'execute') then
    raise exception 'authenticated o anon pueden ejecutar las funciones de consultas';
  end if;
  if not has_function_privilege('service_role', 'public.ia_consultar(text, text, integer)', 'execute') then
    raise exception 'service_role no puede ejecutar ia_consultar';
  end if;
  if has_schema_privilege('authenticated', 'ia', 'usage') or has_schema_privilege('anon', 'ia', 'usage')
     or has_table_privilege('service_role', 'ia.contratistas', 'select') then
    raise exception 'Alguien fuera de los lectores tiene acceso al esquema ia';
  end if;
  if has_table_privilege('authenticated', 'public.ia_permisos', 'insert')
     or has_table_privilege('authenticated', 'public.ia_permisos', 'update')
     or has_table_privilege('anon', 'public.ia_permisos', 'select') then
    raise exception 'Alguien puede darse alcances o anon lee ia_permisos';
  end if;
  if has_table_privilege('ia_lector', 'public.contratistas', 'select')
     or has_table_privilege('ia_lector', 'public.tokens_push', 'select') then
    raise exception 'ia_lector puede leer tablas reales';
  end if;
  if has_table_privilege('ia_lector', 'ia.telemetria_ordenes', 'select') then
    raise exception 'ia_lector puede leer telemetría';
  end if;
  if not has_table_privilege('ia_lector_dueno', 'ia.contratistas', 'select') then
    raise exception 'El dueño no hereda las vistas generales';
  end if;
  if exists (select 1 from information_schema.columns where table_schema = 'ia'
              and column_name in ('codigo_hash', 'token', 'clave_huella', 'clave_publica_jwk', 'sesion_id',
                                  'contenido_hash', 'auth_user_id', 'last_ip', 'usado_ip')) then
    raise exception 'Una vista de ia expone un secreto';
  end if;
end $$;

-- ia_describir: el dueño ve la telemetría; otro, no.
set local role service_role;
do $$
declare
  v_otro text := public.ia_describir(current_setting('diagnostico.otro'))::text;
  v_dueno text := public.ia_describir(current_setting('diagnostico.dueno'))::text;
begin
  if v_otro not like '%"vista" : "contratistas"%' or v_otro like '%telemetria%' then
    raise exception 'ia_describir para otra cuenta: %', left(v_otro, 300);
  end if;
  if v_dueno not like '%"vista" : "telemetria_ordenes"%' or v_dueno not like '%"vista" : "contratistas"%' then
    raise exception 'ia_describir para el dueño no trae telemetría';
  end if;
end $$;

-- Rechazos (cada uno en su bloque: el error revierte sólo ese intento).
do $$
begin
  begin
    perform public.ia_consultar('select 1 as a; select 2 as b', current_setting('diagnostico.otro'));
    raise exception 'Aceptó dos sentencias';
  exception when invalid_parameter_value then null; end;

  begin
    perform public.ia_consultar('select 1 as a', current_setting('diagnostico.otro'), 0);
    raise exception 'Aceptó un límite de 0 filas';
  exception when invalid_parameter_value then null; end;

  begin
    perform public.ia_consultar('select count(*) as n from ia.telemetria_ordenes', current_setting('diagnostico.otro'));
    raise exception 'Otra cuenta leyó telemetría';
  exception when insufficient_privilege then null; end;

  begin
    perform public.ia_consultar('select token from public.tokens_push', current_setting('diagnostico.dueno'));
    raise exception 'Leyó una tabla real con secretos';
  exception when insufficient_privilege then null; end;

  begin
    perform public.ia_consultar('select nombre from public.contratistas', current_setting('diagnostico.dueno'));
    raise exception 'Leyó una tabla real fuera de ia';
  exception when insufficient_privilege then null; end;

  begin
    perform public.ia_consultar('select set_config(''role'', ''service_role'', true) as r',
                                current_setting('diagnostico.otro'));
    raise exception 'Dejó cambiar de rol dentro de la consulta';
  exception when insufficient_privilege then null; end;

  begin
    perform public.ia_consultar('select set_config(''transaction_read_only'', ''off'', true) as r',
                                current_setting('diagnostico.otro'));
    raise exception 'Dejó salir de solo lectura';
  exception when others then
    if sqlerrm like 'Dejó salir%' then raise; end if;
  end;

  begin
    perform public.ia_consultar('with x as (delete from public.sitios returning 1) select * from x',
                                current_setting('diagnostico.dueno'));
    raise exception 'Ejecutó un DELETE';
  exception when others then
    if sqlerrm like 'Ejecutó%' then raise; end if;
  end;
end $$;

-- Consultas válidas: columnas en orden, límite de filas y vuelta al rol de antes.
do $$
declare
  v json;
begin
  v := public.ia_consultar('select count(*) as cantidad, max(nombre) as ultimo from ia.contratistas',
                           current_setting('diagnostico.otro'));
  if (v -> 'columnas')::text <> '["cantidad", "ultimo"]' or json_array_length(v -> 'filas') <> 1
     or (v ->> 'truncado')::boolean then
    raise exception 'Consulta normal: %', v;
  end if;

  v := public.ia_consultar('select g as n from generate_series(1, 5) as g', current_setting('diagnostico.otro'), 3);
  if json_array_length(v -> 'filas') <> 3 or not (v ->> 'truncado')::boolean then
    raise exception 'No respetó el límite de filas: %', v;
  end if;

  v := public.ia_consultar('select g as n from generate_series(1, 2) as g where false', current_setting('diagnostico.otro'));
  if (v -> 'filas')::text <> '[]' or (v -> 'columnas')::text <> '[]' then
    raise exception 'Una consulta sin filas: %', v;
  end if;

  v := public.ia_consultar('select count(*) as n from ia.telemetria_ordenes', current_setting('diagnostico.dueno'));
  if json_array_length(v -> 'filas') <> 1 then
    raise exception 'El dueño no pudo leer telemetría: %', v;
  end if;

  if current_user <> 'service_role' then
    raise exception 'ia_consultar no devolvió el rol original: %', current_user;
  end if;
  if current_setting('transaction_read_only') <> 'on' then
    raise exception 'La transacción no quedó en solo lectura';
  end if;
end $$;

-- El DELETE de arriba no borró nada.
reset role;
do $$
begin
  if (select count(*) from public.sitios)::text <> current_setting('diagnostico.sitios') then
    raise exception 'Cambió la cantidad de sitios';
  end if;
end $$;

rollback;
