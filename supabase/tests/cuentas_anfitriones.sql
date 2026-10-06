-- Alta de anfitriones por SQL (migración alta_de_anfitriones_por_sql).
-- Ejecutar en una sola sesión. Todo se revierte; los pedidos HTTP que el
-- trigger encola en pg_net también se descartan con el rollback.
begin;

-- (1) El correo tiene que venir normalizado: auth.email() siempre llega en
-- minúsculas y uno con mayúsculas dejaría al anfitrión sin acceso.
do $$
begin
  begin
    insert into public.anfitriones (correo, nombre) values ('Prueba.Alta@Example.invalid', 'Prueba');
    raise exception 'Se aceptó un correo con mayúsculas';
  exception when check_violation then null;
  end;
  begin
    insert into public.anfitriones (correo, nombre) values (' prueba-alta@example.invalid', 'Prueba');
    raise exception 'Se aceptó un correo con espacios';
  exception when check_violation then null;
  end;
  begin
    insert into public.anfitriones (correo, nombre) values ('sin-arroba', 'Prueba');
    raise exception 'Se aceptó un correo sin formato';
  exception when check_violation then null;
  end;
end $$;

-- (2) El alta normal nunca se bloquea por la sincronización (con o sin los
-- secretos de Vault cargados), y la baja tampoco.
insert into public.anfitriones (correo, nombre) values ('prueba-alta@example.invalid', 'Prueba');
update public.anfitriones set activo = false where correo = 'prueba-alta@example.invalid';
delete from public.anfitriones where correo = 'prueba-alta@example.invalid';

-- (3) El secreto nunca valida vacío, corto o equivocado.
do $$
begin
  if public.secreto_cuentas_anfitriones_valido(null)
     or public.secreto_cuentas_anfitriones_valido('')
     or public.secreto_cuentas_anfitriones_valido('corto')
     or public.secreto_cuentas_anfitriones_valido(repeat('x', 64)) then
    raise exception 'secreto_cuentas_anfitriones_valido aceptó un secreto inválido';
  end if;
end $$;

-- (4) Nadie fuera de service_role puede probar secretos ni disparar la
-- sincronización.
do $$
begin
  if has_function_privilege('anon', 'public.secreto_cuentas_anfitriones_valido(text)', 'execute')
     or has_function_privilege('authenticated', 'public.secreto_cuentas_anfitriones_valido(text)', 'execute') then
    raise exception 'anon/authenticated pueden ejecutar secreto_cuentas_anfitriones_valido';
  end if;
  if not has_function_privilege('service_role', 'public.secreto_cuentas_anfitriones_valido(text)', 'execute') then
    raise exception 'service_role no puede ejecutar secreto_cuentas_anfitriones_valido';
  end if;
  if has_function_privilege('authenticated', 'private.sincronizar_cuentas_anfitriones()', 'execute')
     or has_function_privilege('anon', 'private.sincronizar_cuentas_anfitriones()', 'execute') then
    raise exception 'anon/authenticated pueden disparar la sincronización de cuentas';
  end if;
end $$;

rollback;
