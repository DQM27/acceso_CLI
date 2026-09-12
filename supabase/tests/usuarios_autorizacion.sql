-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
--
-- Cierre del hallazgo A-01 para `usuarios` (2026-09-12, ver
-- docs/decisiones-tecnicas.md y docs/arquitectura-supabase.md 6.4): hasta
-- ahora cualquier dispositivo con `sitio_id` en el JWT podía crear/editar
-- usuarios de su propio sitio -- incluido escalar el campo `rol` a
-- ADMINISTRADOR de un usuario de OTRO sitio, riesgo documentado y aceptado
-- mientras `--tui-clasica`/desktop viejo todavía creaban usuarios
-- localmente y los empujaban por el outbox. Con el alta/edición migrada al
-- panel (admin-create-usuario/admin-reset-password-usuario) y CLI/TUI
-- clásica retiradas del crate raíz, INSERT/UPDATE quedan exclusivos de
-- admin_global -- "crear usuarios (solo admin_global)"/"actualizar usuarios
-- (solo admin_global)" (antes "... (propio sitio o admin_global)"/"...
-- (global)"). SELECT sigue igual: cualquier sesión autenticada que sea
-- dispositivo o admin_global puede leer el catálogo completo (hace falta
-- para sincronizar), sólo se cerró la escritura.
begin;

insert into public.sitios (id, nombre) values
  (gen_random_uuid(), 'Diagnóstico A'),
  (gen_random_uuid(), 'Diagnóstico B');

select set_config('diagnostico.sitio_a', (select id::text from public.sitios where nombre = 'Diagnóstico A'), true),
       set_config('diagnostico.sitio_b', (select id::text from public.sitios where nombre = 'Diagnóstico B'), true),
       set_config('diagnostico.correo_admin', 'diagnostico-admin@example.com', true);

insert into public.dispositivos (id, sitio_id, tipo, etiqueta, secret_hash) values
  (gen_random_uuid(), current_setting('diagnostico.sitio_a')::uuid, 'pc', 'Diagnóstico PC A', 'diag-hash-a');

insert into public.administradores_panel (correo) values (current_setting('diagnostico.correo_admin'));

-- Sembrado directo (bypassa RLS, como service_role) -- ya no hay forma de
-- crear este usuario de prueba vía INSERT normal, que es justo lo que este
-- test verifica.
insert into public.usuarios (id, sitio_id, cedula, nombre, rol) values
  (gen_random_uuid(), current_setting('diagnostico.sitio_a')::uuid, 'diag-cedula-1', 'Diagnóstico operador', 'OPERADOR');

set local role authenticated;

-- Crear: un dispositivo de su propio sitio YA NO puede -- antes de este
-- cierre esto sí funcionaba.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'sitio_id', current_setting('diagnostico.sitio_a'))::text,
  true);
do $$
begin
  begin
    insert into public.usuarios (id, sitio_id, cedula, nombre, rol)
    values (gen_random_uuid(), current_setting('diagnostico.sitio_a')::uuid, 'diag-cedula-2', 'Diagnóstico bloqueado', 'OPERADOR');
    raise exception 'Un dispositivo pudo crear un usuario (se esperaba que ya no pudiera)';
  exception
    when insufficient_privilege then null;
  end;
end $$;

-- Leer: sigue sin restricción por sitio -- cualquier dispositivo ve TODOS
-- los usuarios de TODOS los sitios (sin cambios, hace falta para
-- sincronizar el catálogo).
do $$
begin
  if not exists (select 1 from public.usuarios where cedula = 'diag-cedula-1') then
    raise exception 'Un dispositivo no puede leer usuarios (se esperaba lectura global sin cambios)';
  end if;
end $$;

-- Actualizar (incluido escalar rol): un dispositivo de su propio sitio YA
-- NO puede -- antes de este cierre esto sí funcionaba, incluso contra un
-- usuario de OTRO sitio.
do $$
begin
  update public.usuarios set rol = 'ADMINISTRADOR' where cedula = 'diag-cedula-1';
  if found then
    raise exception 'Un dispositivo pudo escalar el rol de un usuario (se esperaba que ya no pudiera)';
  end if;
end $$;

-- Una sesión `authenticated` que no es ni dispositivo ni admin_global sigue
-- sin poder leer ni escribir -- sin cambios respecto al hallazgo anterior.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', 'diagnostico-sin-permiso@example.com')::text,
  true);
do $$
begin
  if exists (select 1 from public.usuarios where cedula = 'diag-cedula-1') then
    raise exception 'Una sesión sin sitio_id ni admin_global pudo leer usuarios (hueco de seguridad reabierto)';
  end if;
end $$;

-- admin_global sigue pudiendo leer/crear/actualizar sin restricción --
-- único camino de escritura que queda, el panel delega acá.
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
  insert into public.usuarios (id, sitio_id, cedula, nombre, rol)
  values (gen_random_uuid(), current_setting('diagnostico.sitio_a')::uuid, 'diag-cedula-3', 'Diagnóstico admin_global', 'ADMINISTRADOR');
  if not found then
    raise exception 'admin_global no pudo crear un usuario';
  end if;
end $$;

select '6 comprobaciones de autorización correctas' as resultado;
rollback;
