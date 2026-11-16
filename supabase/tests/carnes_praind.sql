-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
--
-- Cubre la migración carnes_praind.
--   1.  Un equipo vigente ve carnés pero no carga con el interruptor apagado.
--   2.  Un equipo no puede escribir `praind_foto` con un UPDATE ni un INSERT
--       directo: la columna conserva su valor.
--   3.  Una sesión sin equipo ni administrador no ve ni carga.
--   4.  El administrador carga; la ruta mala o el tipo malo se rechazan.
--   5.  Con el interruptor encendido el equipo carga y recibe la clave anterior.
--   6.  Volver a cargar el mismo archivo no cambia nada.
--   7.  El equipo no quita carnés, no cambia el ajuste ni lee la bitácora.
--   8.  Un equipo retirado no ve ni carga.
--   9.  El administrador quita el carné.
--   10. `anon` no ejecuta ninguna función.
--   11. El catálogo de los equipos (`catalogo_del_sitio`) trae `praind_foto`
--       (migración catalogo_del_sitio_con_carne).
begin;

insert into public.sitios (id, nombre) values (gen_random_uuid(), 'Diagnóstico PRAIND');
select set_config('diag.sitio', (select id::text from public.sitios where nombre = 'Diagnóstico PRAIND'), true);

insert into public.dispositivos (id, sitio_id, tipo, etiqueta, clave_publica_jwk, clave_huella, revoked_at) values
  (gen_random_uuid(), current_setting('diag.sitio')::uuid, 'pc', 'Diag PRAIND vigente', '{"kty":"EC"}', 'diag-praind-v', null),
  (gen_random_uuid(), current_setting('diag.sitio')::uuid, 'mobile', 'Diag PRAIND retirado', '{"kty":"EC"}', 'diag-praind-r', now());
select set_config('diag.vigente', (select id::text from public.dispositivos where etiqueta = 'Diag PRAIND vigente'), true),
       set_config('diag.retirado', (select id::text from public.dispositivos where etiqueta = 'Diag PRAIND retirado'), true);

insert into public.administradores_panel (correo) values ('diag-praind-admin@example.com');

insert into public.contratistas (id, nombre) values (gen_random_uuid(), 'Diagnóstico PRAIND contratista');
select set_config('diag.c', (select id::text from public.contratistas where nombre = 'Diagnóstico PRAIND contratista'), true),
       set_config('diag.sha1', repeat('a', 64), true),
       set_config('diag.sha2', repeat('b', 64), true);
select set_config('diag.clave1', 'contratistas/' || current_setting('diag.c') || '/' || current_setting('diag.sha1') || '.png', true),
       set_config('diag.clave2', 'contratistas/' || current_setting('diag.c') || '/' || current_setting('diag.sha2') || '.jpg', true);

-- Las sesiones simuladas.
create temporary table diag_sesiones (nombre text primary key, claims text) on commit drop;
insert into diag_sesiones values
  ('equipo', json_build_object('role', 'authenticated', 'sub', current_setting('diag.vigente'),
                               'huella', 'diag-praind-v', 'sitio_id', current_setting('diag.sitio'))::text),
  ('retirado', json_build_object('role', 'authenticated', 'sub', current_setting('diag.retirado'),
                                 'huella', 'diag-praind-r', 'sitio_id', current_setting('diag.sitio'))::text),
  ('admin', json_build_object('role', 'authenticated', 'email', 'diag-praind-admin@example.com')::text),
  ('nadie', json_build_object('role', 'authenticated', 'email', 'diag-praind-nadie@example.com')::text);
grant select on diag_sesiones to authenticated;

set local role authenticated;

-- 1. Equipo vigente, interruptor apagado.
select set_config('request.jwt.claims', (select claims from diag_sesiones where nombre = 'equipo'), true);
do $$
declare v jsonb := public.praind_permisos();
begin
  if not (v ->> 'puede_ver')::boolean then raise exception '1a: el equipo vigente no puede ver'; end if;
  if (v ->> 'puede_cargar')::boolean then raise exception '1b: el equipo carga con el interruptor apagado'; end if;
  begin
    perform public.praind_registrar(current_setting('diag.c')::uuid, current_setting('diag.clave1'),
                                    current_setting('diag.sha1'), 100, 'image/png');
    raise exception '1c: el equipo registró un carné con el interruptor apagado';
  exception when insufficient_privilege then null;
  end;
end $$;

-- 2. Escritura directa de la columna.
update public.contratistas set praind_foto = 'contratistas/x/y.png' where id = current_setting('diag.c')::uuid;
insert into public.contratistas (id, dispositivo_origen_id, nombre, praind_foto)
values (gen_random_uuid(), current_setting('diag.vigente')::uuid, 'Diagnóstico PRAIND insertado', 'contratistas/x/z.png');
do $$
begin
  if (select praind_foto from public.contratistas where id = current_setting('diag.c')::uuid) is not null then
    raise exception '2a: un UPDATE de equipo cambió praind_foto';
  end if;
  if (select praind_foto from public.contratistas where nombre = 'Diagnóstico PRAIND insertado') is not null then
    raise exception '2b: un INSERT de equipo trajo praind_foto';
  end if;
end $$;

-- 3. Sesión sin equipo ni administrador.
select set_config('request.jwt.claims', (select claims from diag_sesiones where nombre = 'nadie'), true);
do $$
declare v jsonb := public.praind_permisos();
begin
  if (v ->> 'puede_ver')::boolean or (v ->> 'puede_cargar')::boolean then
    raise exception '3: una sesión sin permisos ve o carga';
  end if;
end $$;

-- 4. Administrador.
select set_config('request.jwt.claims', (select claims from diag_sesiones where nombre = 'admin'), true);
do $$
declare v jsonb := public.praind_permisos();
begin
  if not ((v ->> 'puede_ver')::boolean and (v ->> 'puede_cargar')::boolean and (v ->> 'es_admin')::boolean) then
    raise exception '4a: el administrador no tiene todos los permisos: %', v;
  end if;
  begin
    perform public.praind_registrar(current_setting('diag.c')::uuid,
      'contratistas/' || gen_random_uuid() || '/' || current_setting('diag.sha1') || '.png',
      current_setting('diag.sha1'), 100, 'image/png');
    raise exception '4b: se aceptó la ruta de otro contratista';
  exception when invalid_parameter_value then null;
  end;
  begin
    perform public.praind_registrar(current_setting('diag.c')::uuid, current_setting('diag.clave1'),
                                    current_setting('diag.sha1'), 100, 'image/gif');
    raise exception '4c: se aceptó un tipo que no es PNG ni JPEG';
  exception when invalid_parameter_value then null;
  end;
  begin
    perform public.praind_registrar(gen_random_uuid(), current_setting('diag.clave1'),
                                    current_setting('diag.sha1'), 100, 'image/png');
    raise exception '4d: se aceptó un contratista inexistente';
  exception when invalid_parameter_value or no_data_found then null;
  end;
  if public.praind_registrar(current_setting('diag.c')::uuid, current_setting('diag.clave1'),
                             current_setting('diag.sha1'), 1240249, 'image/png',
                             'https://prd.pw/diag', 'Diagnóstico PRAIND contratista') is not null then
    raise exception '4e: el primer carné devolvió una clave anterior';
  end if;
  if (select praind_foto from public.contratistas where id = current_setting('diag.c')::uuid)
     is distinct from current_setting('diag.clave1') then
    raise exception '4f: praind_foto no quedó con el carné cargado';
  end if;
  if (select cargado_por from public.fotos_praind where clave = current_setting('diag.clave1'))
     is distinct from 'diag-praind-admin@example.com' then
    raise exception '4g: la bitácora no anotó al administrador';
  end if;
  perform public.praind_cambiar_carga_desde_equipos(true);
end $$;

-- 5. Equipo con el interruptor encendido (antes, 11: el catálogo trae el carné).
select set_config('request.jwt.claims', (select claims from diag_sesiones where nombre = 'equipo'), true);
do $$
begin
  if not exists (
    select 1 from jsonb_array_elements(public.catalogo_del_sitio(null, null, 1000) -> 'contratistas') c
     where c ->> 'id' = current_setting('diag.c') and c ->> 'praind_foto' = current_setting('diag.clave1')
  ) then
    raise exception '11: el catálogo del equipo no trae praind_foto';
  end if;
  if not (public.praind_permisos() ->> 'puede_cargar')::boolean then
    raise exception '5a: el equipo no carga con el interruptor encendido';
  end if;
  if public.praind_registrar(current_setting('diag.c')::uuid, current_setting('diag.clave2'),
                             current_setting('diag.sha2'), 500, 'image/jpeg')
     is distinct from current_setting('diag.clave1') then
    raise exception '5b: reemplazar no devolvió la clave anterior';
  end if;
end $$;

-- 6. Mismo archivo otra vez.
do $$
begin
  if public.praind_registrar(current_setting('diag.c')::uuid, current_setting('diag.clave2'),
                             current_setting('diag.sha2'), 500, 'image/jpeg') is not null then
    raise exception '6: recargar el mismo archivo devolvió una clave para borrar';
  end if;
end $$;

-- 7. Lo que el equipo no puede aunque cargue.
do $$
begin
  begin
    perform public.praind_quitar(current_setting('diag.c')::uuid);
    raise exception '7a: el equipo quitó un carné';
  exception when insufficient_privilege then null;
  end;
  begin
    perform public.praind_cambiar_carga_desde_equipos(false);
    raise exception '7b: el equipo cambió el interruptor';
  exception when insufficient_privilege then null;
  end;
  if exists (select 1 from public.fotos_praind) then
    raise exception '7c: el equipo lee la bitácora';
  end if;
  if exists (select 1 from public.ajustes_praind) then
    raise exception '7d: el equipo lee los ajustes';
  end if;
end $$;

-- 8. Equipo retirado.
select set_config('request.jwt.claims', (select claims from diag_sesiones where nombre = 'retirado'), true);
do $$
declare v jsonb := public.praind_permisos();
begin
  if (v ->> 'puede_ver')::boolean or (v ->> 'puede_cargar')::boolean then
    raise exception '8: un equipo retirado ve o carga';
  end if;
end $$;

-- 9. El administrador revisa la bitácora y quita el carné.
select set_config('request.jwt.claims', (select claims from diag_sesiones where nombre = 'admin'), true);
do $$
begin
  if (select count(*) from public.fotos_praind where contratista_id = current_setting('diag.c')::uuid) <> 2 then
    raise exception '9a: la bitácora no tiene los dos carnés';
  end if;
  if (select count(*) from public.fotos_praind
       where contratista_id = current_setting('diag.c')::uuid and reemplazada_en is null) <> 1 then
    raise exception '9b: no hay exactamente un carné vigente';
  end if;
  if public.praind_quitar(current_setting('diag.c')::uuid) is distinct from current_setting('diag.clave2') then
    raise exception '9c: quitar no devolvió la clave del carné vigente';
  end if;
  if (select praind_foto from public.contratistas where id = current_setting('diag.c')::uuid) is not null then
    raise exception '9d: praind_foto sigue con valor después de quitar';
  end if;
  if public.praind_quitar(current_setting('diag.c')::uuid) is not null then
    raise exception '9e: quitar sin carné devolvió una clave';
  end if;
end $$;

-- 10. anon.
reset role;
set local role anon;
do $$
begin
  begin
    perform public.praind_permisos();
    raise exception '10: anon ejecutó praind_permisos';
  exception when insufficient_privilege then null;
  end;
end $$;

reset role;
select 'carnes_praind: todas las pruebas pasaron' as resultado;

rollback;
