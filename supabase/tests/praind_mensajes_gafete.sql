-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
--
-- Cubre la migración praind_mensajes_gafete: los errores de las funciones de
-- gafetes PRAIND dicen "gafete PRAIND" y no "carné", con los mismos códigos
-- de error de antes.
--   1. Sin permiso de carga: 42501, "No tiene permiso para cargar gafetes PRAIND."
--   2. Tipo que no es PNG ni JPEG: 22023, "El gafete PRAIND debe ser una imagen PNG o JPEG."
--   3. Ruta que no corresponde: 22023, "La ruta del gafete PRAIND no corresponde…"
--   4. Quitar sin ser el panel: 42501, "Sólo el panel puede quitar gafetes PRAIND."
--   5. Ninguna de las dos funciones dice "carné" en su código.
--   6. `anon` sigue sin poder ejecutarlas.
begin;

insert into public.administradores_panel (correo) values ('diag-praind-msj-admin@example.com');
insert into public.contratistas (id, nombre) values (gen_random_uuid(), 'Diagnóstico PRAIND mensajes');
select set_config('diag.c', (select id::text from public.contratistas where nombre = 'Diagnóstico PRAIND mensajes'), true),
       set_config('diag.sha', repeat('c', 64), true);

create temporary table diag_sesiones (nombre text primary key, claims text) on commit drop;
insert into diag_sesiones values
  ('admin', json_build_object('role', 'authenticated', 'email', 'diag-praind-msj-admin@example.com')::text),
  ('nadie', json_build_object('role', 'authenticated', 'email', 'diag-praind-msj-nadie@example.com')::text);
grant select on diag_sesiones to authenticated;

set local role authenticated;

-- 1 y 4. Sesión sin equipo ni administrador.
select set_config('request.jwt.claims', (select claims from diag_sesiones where nombre = 'nadie'), true);
do $$
declare
  v_mensaje text;
begin
  begin
    perform public.praind_registrar(current_setting('diag.c')::uuid,
      'contratistas/' || current_setting('diag.c') || '/' || current_setting('diag.sha') || '.png',
      current_setting('diag.sha'), 100, 'image/png');
    raise exception '1a: se registró sin permiso';
  exception when insufficient_privilege then
    get stacked diagnostics v_mensaje = message_text;
  end;
  if v_mensaje is distinct from 'No tiene permiso para cargar gafetes PRAIND.' then
    raise exception '1b: mensaje inesperado: %', v_mensaje;
  end if;

  begin
    perform public.praind_quitar(current_setting('diag.c')::uuid);
    raise exception '4a: se quitó sin ser el panel';
  exception when insufficient_privilege then
    get stacked diagnostics v_mensaje = message_text;
  end;
  if v_mensaje is distinct from 'Sólo el panel puede quitar gafetes PRAIND.' then
    raise exception '4b: mensaje inesperado: %', v_mensaje;
  end if;
end $$;

-- 2 y 3. Administrador con datos malos.
select set_config('request.jwt.claims', (select claims from diag_sesiones where nombre = 'admin'), true);
do $$
declare
  v_mensaje text;
begin
  begin
    perform public.praind_registrar(current_setting('diag.c')::uuid,
      'contratistas/' || current_setting('diag.c') || '/' || current_setting('diag.sha') || '.png',
      current_setting('diag.sha'), 100, 'image/gif');
    raise exception '2a: se aceptó un tipo que no es PNG ni JPEG';
  exception when invalid_parameter_value then
    get stacked diagnostics v_mensaje = message_text;
  end;
  if v_mensaje is distinct from 'El gafete PRAIND debe ser una imagen PNG o JPEG.' then
    raise exception '2b: mensaje inesperado: %', v_mensaje;
  end if;

  begin
    perform public.praind_registrar(current_setting('diag.c')::uuid,
      'contratistas/' || gen_random_uuid() || '/' || current_setting('diag.sha') || '.png',
      current_setting('diag.sha'), 100, 'image/png');
    raise exception '3a: se aceptó la ruta de otro contratista';
  exception when invalid_parameter_value then
    get stacked diagnostics v_mensaje = message_text;
  end;
  if v_mensaje is distinct from 'La ruta del gafete PRAIND no corresponde al contratista ni al archivo.' then
    raise exception '3b: mensaje inesperado: %', v_mensaje;
  end if;
end $$;

reset role;

-- 5. Ningún "carné" queda en el código de las funciones.
do $$
begin
  if exists (
    select 1 from pg_proc p join pg_namespace n on n.oid = p.pronamespace
     where n.nspname = 'public' and p.proname in ('praind_registrar', 'praind_quitar')
       and p.prosrc ilike '%carn%'
  ) then
    raise exception '5: una función de gafetes PRAIND todavía dice "carné"';
  end if;
end $$;

-- 6. anon no las ejecuta.
do $$
begin
  if has_function_privilege('anon', 'public.praind_registrar(uuid, text, text, bigint, text, text, text)', 'execute')
     or has_function_privilege('anon', 'public.praind_quitar(uuid)', 'execute') then
    raise exception '6: anon puede ejecutar una función de gafetes PRAIND';
  end if;
  if not has_function_privilege('authenticated', 'public.praind_registrar(uuid, text, text, bigint, text, text, text)', 'execute') then
    raise exception '6b: authenticated perdió el permiso de praind_registrar';
  end if;
end $$;

select 'praind_mensajes_gafete: todas las pruebas pasaron' as resultado;

rollback;
