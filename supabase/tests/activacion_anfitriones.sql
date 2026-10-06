-- Cuentas de anfitriones con código de activación (migración
-- activacion_de_anfitriones_desde_el_panel). Ejecutar en una sola sesión;
-- todo se revierte.
begin;

-- Datos de prueba: un admin del panel y un anfitrión con su cuenta de Auth.
insert into public.administradores_panel (correo) values ('prueba-admin@example.invalid');
insert into auth.users (id, email, encrypted_password)
values ('00000000-0000-4000-8000-0000000a0001', 'prueba-anfitrion@example.invalid', 'aleatoria');
insert into public.anfitriones (correo, nombre, auth_user_id)
values ('prueba-anfitrion@example.invalid', 'PRUEBA', '00000000-0000-4000-8000-0000000a0001');

-- (1) El correo tiene que venir normalizado.
do $$
begin
  begin
    insert into public.anfitriones (correo, nombre) values ('Prueba.Mayus@Example.invalid', 'X');
    raise exception 'Se aceptó un correo con mayúsculas';
  exception when check_violation then null;
  end;
  begin
    insert into public.anfitriones (correo, nombre) values ('sin-arroba', 'X');
    raise exception 'Se aceptó un correo sin formato';
  exception when check_violation then null;
  end;
end $$;

-- (2) Emitir, fallar, acertar y consumir.
do $$
declare
  v_vence timestamptz;
  v_id uuid;
begin
  v_vence := public.emitir_codigo_anfitrion('prueba-anfitrion@example.invalid', 'ABCDE23456', 'prueba-admin@example.invalid', 'crear');
  if v_vence < now() + interval '71 hours' or v_vence > now() + interval '73 hours' then
    raise exception 'El código no vence a las 72 h: %', v_vence;
  end if;
  if exists (select 1 from private.anfitriones_activacion where codigo_hash = 'ABCDE23456') then
    raise exception 'El código quedó guardado en texto plano';
  end if;

  if public.verificar_codigo_anfitrion('prueba-anfitrion@example.invalid', 'ZZZZZ99999') is not null then
    raise exception 'Un código errado fue aceptado';
  end if;
  if (select intentos from private.anfitriones_activacion where correo = 'prueba-anfitrion@example.invalid') <> 1 then
    raise exception 'Un código errado no sumó un intento';
  end if;

  v_id := public.verificar_codigo_anfitrion('prueba-anfitrion@example.invalid', 'ABCDE23456');
  if v_id is distinct from '00000000-0000-4000-8000-0000000a0001'::uuid then
    raise exception 'El código correcto no devolvió la cuenta: %', v_id;
  end if;
  -- Verificar no consume: la contraseña puede fallar y la persona reintenta.
  if not exists (select 1 from private.anfitriones_activacion where correo = 'prueba-anfitrion@example.invalid') then
    raise exception 'Verificar consumió el código';
  end if;

  perform public.consumir_codigo_anfitrion('prueba-anfitrion@example.invalid');
  if public.verificar_codigo_anfitrion('prueba-anfitrion@example.invalid', 'ABCDE23456') is not null then
    raise exception 'Un código consumido volvió a servir';
  end if;
end $$;

-- (3) Cinco fallos agotan el código, aunque después llegue el correcto.
do $$
begin
  perform public.emitir_codigo_anfitrion('prueba-anfitrion@example.invalid', 'BCDEF34567', 'prueba-admin@example.invalid', 'restablecer');
  for i in 1..5 loop
    perform public.verificar_codigo_anfitrion('prueba-anfitrion@example.invalid', 'MALO' || i || 'XXXXXX');
  end loop;
  if public.verificar_codigo_anfitrion('prueba-anfitrion@example.invalid', 'BCDEF34567') is not null then
    raise exception 'El código sirvió después de agotar los intentos';
  end if;
end $$;

-- (4) Un código vencido no sirve.
do $$
begin
  perform public.emitir_codigo_anfitrion('prueba-anfitrion@example.invalid', 'CDEFG45678', 'prueba-admin@example.invalid', 'restablecer');
  update private.anfitriones_activacion set vence = now() - interval '1 second'
  where correo = 'prueba-anfitrion@example.invalid';
  if public.verificar_codigo_anfitrion('prueba-anfitrion@example.invalid', 'CDEFG45678') is not null then
    raise exception 'Un código vencido fue aceptado';
  end if;
end $$;

-- (5) Deshabilitar borra el código pendiente y bloquea verificar y emitir.
do $$
begin
  perform public.emitir_codigo_anfitrion('prueba-anfitrion@example.invalid', 'DEFGH56789', 'prueba-admin@example.invalid', 'restablecer');
  if public.cambiar_estado_anfitrion('prueba-anfitrion@example.invalid', false, 'prueba-admin@example.invalid')
     is distinct from '00000000-0000-4000-8000-0000000a0001'::uuid then
    raise exception 'cambiar_estado_anfitrion no devolvió la cuenta';
  end if;
  if exists (select 1 from private.anfitriones_activacion where correo = 'prueba-anfitrion@example.invalid') then
    raise exception 'Deshabilitar no borró el código pendiente';
  end if;
  begin
    perform public.emitir_codigo_anfitrion('prueba-anfitrion@example.invalid', 'EFGHJ67892', 'prueba-admin@example.invalid', 'restablecer');
    raise exception 'Se emitió un código para un anfitrión deshabilitado';
  exception when no_data_found then null;
  end;
  perform public.cambiar_estado_anfitrion('prueba-anfitrion@example.invalid', true, 'prueba-admin@example.invalid');
  if (select count(*) from private.bitacora_anfitriones where correo = 'prueba-anfitrion@example.invalid') < 6 then
    raise exception 'La bitácora no registró todas las acciones';
  end if;
end $$;

-- (6) Cerrar sesiones borra las de esa cuenta y sus refresh tokens.
insert into auth.sessions (id, user_id) values ('00000000-0000-4000-8000-0000000b0001', '00000000-0000-4000-8000-0000000a0001');
insert into auth.refresh_tokens (session_id, user_id) values ('00000000-0000-4000-8000-0000000b0001', '00000000-0000-4000-8000-0000000a0001');
select public.cerrar_sesiones_de_cuenta('00000000-0000-4000-8000-0000000a0001');
do $$
begin
  if exists (select 1 from auth.sessions where user_id = '00000000-0000-4000-8000-0000000a0001')
     or exists (select 1 from auth.refresh_tokens where session_id = '00000000-0000-4000-8000-0000000b0001') then
    raise exception 'Quedaron sesiones abiertas';
  end if;
end $$;

-- (7) Permisos: solo service_role toca códigos y cuentas; el listado exige
-- administrador del panel y nunca expone hashes.
do $$
declare
  v_funcion text;
begin
  foreach v_funcion in array array[
    'public.emitir_codigo_anfitrion(text, text, text, text)',
    'public.verificar_codigo_anfitrion(text, text)',
    'public.consumir_codigo_anfitrion(text)',
    'public.cambiar_estado_anfitrion(text, boolean, text)',
    'public.cerrar_sesiones_de_cuenta(uuid)',
    'public.cuenta_auth_por_correo(text)'
  ] loop
    if has_function_privilege('anon', v_funcion, 'execute') or has_function_privilege('authenticated', v_funcion, 'execute') then
      raise exception 'anon/authenticated pueden ejecutar %', v_funcion;
    end if;
    if not has_function_privilege('service_role', v_funcion, 'execute') then
      raise exception 'service_role no puede ejecutar %', v_funcion;
    end if;
  end loop;
  if has_table_privilege('authenticated', 'private.anfitriones_activacion', 'select') then
    raise exception 'authenticated puede leer los códigos';
  end if;
end $$;

set local role authenticated;
select set_config('request.jwt.claims', '{"role":"authenticated","email":"prueba-anfitrion@example.invalid"}', true);
do $$
begin
  perform public.panel_anfitriones();
  raise exception 'Un anfitrión pudo listar las cuentas del panel';
exception when insufficient_privilege then null;
end $$;
select set_config('request.jwt.claims', '{"role":"authenticated","email":"prueba-admin@example.invalid"}', true);
do $$
begin
  if (select estado from public.panel_anfitriones() where correo = 'prueba-anfitrion@example.invalid') <> 'activa' then
    raise exception 'El panel no ve el estado de la cuenta';
  end if;
end $$;
reset role;

-- (8) Estados que ve el panel.
do $$
begin
  perform public.emitir_codigo_anfitrion('prueba-anfitrion@example.invalid', 'FGHJK78923', 'prueba-admin@example.invalid', 'restablecer');
  perform set_config('request.jwt.claims', '{"role":"authenticated","email":"prueba-admin@example.invalid"}', true);
  if (select estado from public.panel_anfitriones() where correo = 'prueba-anfitrion@example.invalid') <> 'pendiente' then
    raise exception 'Un código vigente no figura como pendiente';
  end if;
  update private.anfitriones_activacion set vence = now() - interval '1 second'
  where correo = 'prueba-anfitrion@example.invalid';
  if (select estado from public.panel_anfitriones() where correo = 'prueba-anfitrion@example.invalid') <> 'codigo_vencido' then
    raise exception 'Un código vencido no figura como vencido';
  end if;
end $$;

rollback;
