-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
--
-- Cubre la migración panel_contratistas_con_foto_praind.
--   1. `tiene_foto_praind` es true con foto y false sin ella.
--   2. Las columnas de antes siguen iguales (estado y adentro).
--   3. El administrador la lee; una sesión sin equipo ni administrador no ve filas.
--   4. `anon` no puede leer la vista.
begin;

insert into public.administradores_panel (correo) values ('diag-foto-admin@example.com');
insert into public.contratistas (id, nombre, tipo_ingreso, fecha_vencimiento_praind) values
  (gen_random_uuid(), 'Diagnóstico foto con', 'PRAIND', current_date + 100),
  (gen_random_uuid(), 'Diagnóstico foto sin', 'PRAIND', null);
select set_config('diag.con', (select id::text from public.contratistas where nombre = 'Diagnóstico foto con'), true),
       set_config('diag.sin', (select id::text from public.contratistas where nombre = 'Diagnóstico foto sin'), true);
-- Como dueño (salta el disparador que protege la columna de los clientes).
update public.contratistas set praind_foto = 'contratistas/' || id || '/' || repeat('a', 64) || '.png'
 where id = current_setting('diag.con')::uuid;

-- 1 y 2, como dueño.
do $$
declare con record; sin record;
begin
  select * into con from public.panel_contratistas_estado where id = current_setting('diag.con')::uuid;
  select * into sin from public.panel_contratistas_estado where id = current_setting('diag.sin')::uuid;
  if not con.tiene_foto_praind then raise exception '1a: con foto dice false'; end if;
  if sin.tiene_foto_praind then raise exception '1b: sin foto dice true'; end if;
  if con.estado_praind <> 'VIGENTE' or con.estado_acceso <> 'PERMITIDO' then
    raise exception '2a: cambió el estado (%, %)', con.estado_praind, con.estado_acceso;
  end if;
  if sin.estado_acceso <> 'PRAIND_NO_REGISTRADO' or sin.adentro_desde is not null then
    raise exception '2b: cambió el estado o el adentro (%)', sin.estado_acceso;
  end if;
end $$;

set local role authenticated;

-- 3. Administrador y sesión cualquiera.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', 'diag-foto-admin@example.com')::text, true);
do $$
begin
  if not (select tiene_foto_praind from public.panel_contratistas_estado where id = current_setting('diag.con')::uuid) then
    raise exception '3a: el administrador no ve la foto';
  end if;
end $$;
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', 'diag-foto-nadie@example.com')::text, true);
do $$
begin
  if exists (select 1 from public.panel_contratistas_estado where id = current_setting('diag.con')::uuid) then
    raise exception '3b: una sesión sin equipo ni administrador ve contratistas';
  end if;
end $$;

reset role;

-- 4. anon.
do $$
begin
  if has_table_privilege('anon', 'public.panel_contratistas_estado', 'select') then
    raise exception '4: anon puede leer la vista';
  end if;
end $$;

select 'panel_contratistas_con_foto_praind: todo bien' as resultado;
rollback;
