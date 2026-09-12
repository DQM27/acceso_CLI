-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
--
-- OJO: este test documenta a propósito un riesgo real y ya conocido, no
-- solo defiende el comportamiento "bueno". La política de INSERT es una
-- sola, "crear usuarios (propio sitio o admin_global)" (fusionada
-- 2026-09-12 desde "crear usuarios del propio sitio" + "admin_global crea
-- usuarios" -- ver fusiona_politicas_permisivas_duplicadas_de_visitas_y_
-- panel). Las políticas de SELECT/UPDATE, "leer usuarios (global)" y
-- "actualizar usuarios (global)" (migración crea_usuarios_globales,
-- cerrada parcialmente por cierra_acceso_global_a_cuentas_sin_dispositivo_
-- ni_admin), siguen separadas -- el advisor nunca las marcó como
-- duplicadas porque no comparten exactamente la misma condición. Dan SELECT/
-- UPDATE sin restricción a cualquier sesión autenticada que sea un
-- DISPOSITIVO (JWT con `sitio_id`) o admin_global -- incluye poder
-- cambiar el campo `rol` a ADMINISTRADOR de un usuario de otro sitio. Es
-- intencional (mismo criterio que contratistas/empresas, para que una
-- baja propague a todos los sitios), pero sigue siendo un radio de
-- exposición grande: cualquier dispositivo con un JWT válido puede
-- promoverse a administrador sin pasar por ninguna pantalla. Si algún día
-- se decide acotar esa política más, HAY que actualizar este test junto
-- con ella -- que quede fallando es la señal de que el cambio de RLS pasó
-- y alguien lo tiene que revisar, no un bug del test.
--
-- Lo que SÍ se cerró (ver el mismo hallazgo): una sesión `authenticated`
-- que no es ni un dispositivo (sin `sitio_id`) ni admin_global -- p. ej.
-- cualquier cuenta de Google que complete el login OAuth del panel sin
-- estar en administradores_panel -- ya no puede leer ni escribir esta
-- tabla. Antes de esa migración, `using (true)` no distinguía nada de eso.
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

-- El hueco real que se cerró: una cuenta de Google cualquiera (login
-- OAuth exitoso, JWT `authenticated`) que ni es dispositivo (sin
-- `sitio_id`) ni está en administradores_panel NO puede leer ni escribir
-- usuarios -- antes de la migración de este fix, `using (true)` la dejaba
-- pasar igual que a un dispositivo o a un admin real.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', 'diagnostico-sin-permiso@example.com')::text,
  true);
do $$
begin
  if exists (select 1 from public.usuarios where cedula = 'diag-cedula-1') then
    raise exception 'Una sesión sin sitio_id ni admin_global pudo leer usuarios (hueco de seguridad reabierto)';
  end if;
end $$;
-- A diferencia de un INSERT que viola `with check` (eso sí lanza
-- insufficient_privilege de inmediato), acá la fila ni siquiera pasa el
-- filtro `using` de la política -- el UPDATE simplemente no encuentra
-- ninguna fila que tocar, sin excepción. `found` en false es la señal.
do $$
begin
  update public.usuarios set rol = 'ADMINISTRADOR' where cedula = 'diag-cedula-1';
  if found then
    raise exception 'Una sesión sin sitio_id ni admin_global pudo escalar el rol de un usuario (hueco de seguridad reabierto)';
  end if;
end $$;

-- Riesgo real, ya documentado arriba: ese mismo dispositivo de OTRO sitio puede cambiarle el rol
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

-- admin_global también puede CREAR usuarios (migración admin_global_
-- crea_usuarios), sin sitio_id en el JWT -- el panel web delega la
-- creación de Administrador/Operador acá, ver docs/plan-panel-
-- administrativo-web.md punto 4. sitio_id lo elige el panel (hoy hay un
-- solo sitio, "Diagnóstico A" acá).
do $$
begin
  insert into public.usuarios (id, sitio_id, cedula, nombre, rol)
  values (gen_random_uuid(), current_setting('diagnostico.sitio_a')::uuid, 'diag-cedula-3', 'Diagnóstico admin_global', 'ADMINISTRADOR');
  if not found then
    raise exception 'admin_global no pudo crear un usuario';
  end if;
end $$;

select '8 comprobaciones de autorización correctas' as resultado;
rollback;
