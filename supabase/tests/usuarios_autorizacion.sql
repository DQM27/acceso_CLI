-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
--
-- OJO: este test documenta a propósito un riesgo real y ya conocido, no
-- solo defiende el comportamiento "bueno". Las políticas
-- "leer usuarios (global)" y "actualizar usuarios (global)" (migración
-- crea_usuarios_globales) dan SELECT/UPDATE sin restricción a CUALQUIER
-- sesión autenticada -- incluye poder cambiar el campo `rol` a
-- ADMINISTRADOR. Es intencional (mismo criterio que contratistas/empresas,
-- para que una baja propague a todos los sitios), pero es un radio de
-- exposición grande: cualquier dispositivo con un JWT válido puede
-- promoverse a administrador sin pasar por ninguna pantalla. Si algún día
-- se decide cerrar esa política, HAY que actualizar este test junto con
-- ella -- que quede fallando es la señal de que el cambio de RLS pasó y
-- alguien lo tiene que revisar, no un bug del test.
begin;

insert into public.sitios (id, nombre) values
  (gen_random_uuid(), 'Diagnóstico A'),
  (gen_random_uuid(), 'Diagnóstico B');

select set_config('diagnostico.sitio_a', (select id::text from public.sitios where nombre = 'Diagnóstico A'), true),
       set_config('diagnostico.sitio_b', (select id::text from public.sitios where nombre = 'Diagnóstico B'), true),
       set_config('diagnostico.correo_admin', 'diagnostico-admin@example.com', true);

insert into public.dispositivos (id, sitio_id, tipo, etiqueta, secret_hash) values
  (gen_random_uuid(), current_setting('diagnostico.sitio_a')::uuid, 'pc', 'Diagnóstico PC A', 'diag-hash-a');

select set_config('diagnostico.dispositivo_a', (select id::text from public.dispositivos where etiqueta = 'Diagnóstico PC A'), true);

insert into public.administradores_panel (correo) values (current_setting('diagnostico.correo_admin'));

set local role authenticated;

-- Crear: acotado al propio sitio (sí está restringido).
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'sitio_id', current_setting('diagnostico.sitio_a'))::text,
  true);
do $$
begin
  insert into public.usuarios (id, sitio_id, cedula, nombre, rol)
  values (gen_random_uuid(), current_setting('diagnostico.sitio_a')::uuid, 'diag-cedula-1', 'Diagnóstico operador', 'OPERADOR');
  if not found then
    raise exception 'Un dispositivo no pudo crear un usuario en su propio sitio';
  end if;
end $$;

do $$
begin
  begin
    insert into public.usuarios (id, sitio_id, cedula, nombre, rol)
    values (gen_random_uuid(), current_setting('diagnostico.sitio_b')::uuid, 'diag-cedula-2', 'Diagnóstico cruzado', 'OPERADOR');
    raise exception 'Un dispositivo del sitio A pudo crear un usuario para el sitio B';
  exception
    when insufficient_privilege then null;
  end;
end $$;

-- Leer: cualquier sesión autenticada ve TODOS los usuarios de TODOS los
-- sitios (comportamiento documentado, no un bug).
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'sitio_id', current_setting('diagnostico.sitio_b'))::text,
  true);
do $$
begin
  if not exists (select 1 from public.usuarios where cedula = 'diag-cedula-1') then
    raise exception 'Un dispositivo de otro sitio no puede leer un usuario ajeno (se esperaba lectura global)';
  end if;
end $$;

-- Riesgo real: ese mismo dispositivo de OTRO sitio puede cambiarle el rol
-- a ADMINISTRADOR a un usuario ajeno. Esto pasa HOY. Si este test falla
-- porque ya no puede, quiere decir que se cerró la política -- actualizar
-- el comentario de arriba y el hallazgo de seguridad correspondiente.
do $$
begin
  update public.usuarios set rol = 'ADMINISTRADOR' where cedula = 'diag-cedula-1';
  if not found then
    raise exception 'Un dispositivo de otro sitio no pudo escalar el rol de un usuario ajeno (¿ya se cerró la política? actualizar este test)';
  end if;
end $$;

-- admin_global también puede leer/actualizar sin necesitar sitio_id.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_admin'))::text,
  true);
do $$
begin
  if not exists (select 1 from public.usuarios where cedula = 'diag-cedula-1') then
    raise exception 'admin_global no puede leer usuarios';
  end if;
  update public.usuarios set activo = false where cedula = 'diag-cedula-1';
  if not found then
    raise exception 'admin_global no puede actualizar usuarios';
  end if;
end $$;

select '5 comprobaciones de autorización correctas' as resultado;
rollback;
