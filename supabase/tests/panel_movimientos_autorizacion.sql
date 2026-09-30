-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
begin;

select set_config('diagnostico.correo_admin', 'diagnostico-admin@example.com', true),
       set_config('diagnostico.correo_normal', 'diagnostico-normal@example.com', true);

insert into public.administradores_panel (correo)
values (current_setting('diagnostico.correo_admin'));

insert into public.sitios (id, nombre)
values ('aaaaaaaa-0000-0000-0000-00000000a001', 'Unidad de prueba panel');

insert into public.dispositivos (id, sitio_id, tipo, etiqueta, clave_publica_jwk, clave_huella)
values ('cccccccc-0000-0000-0000-00000000c001', 'aaaaaaaa-0000-0000-0000-00000000a001',
        'pc', 'Diagnóstico panel', '{"kty":"EC"}', 'diag-huella-panel');

insert into public.contratistas (id, dispositivo_origen_id, nombre)
values ('dddddddd-0000-0000-0000-00000000d001',
        'cccccccc-0000-0000-0000-00000000c001', 'José Ñandú Prueba');

insert into public.ingresos (id, sitio_id, dispositivo_entrada_id, contratista_id,
                             contratista_cedula, contratista_nombre, empresa_nombre, hora_entrada)
values ('bbbbbbbb-0000-0000-0000-00000000b001', 'aaaaaaaa-0000-0000-0000-00000000a001',
        'cccccccc-0000-0000-0000-00000000c001', 'dddddddd-0000-0000-0000-00000000d001',
        '1-1111-1111', 'José Ñandú Prueba', 'Empresa Acentuada', now());

set local role authenticated;

-- Un admin_global ve la fila, con el nombre de la unidad ya resuelto.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_admin'))::text,
  true);
do $$
begin
  if not exists (
    select 1 from public.panel_movimientos
    where id = 'bbbbbbbb-0000-0000-0000-00000000b001' and sitio_nombre = 'Unidad de prueba panel'
  ) then
    raise exception 'admin_global no ve el movimiento con su unidad en panel_movimientos';
  end if;

  -- La búsqueda ignora tildes y mayúsculas.
  if not exists (
    select 1 from public.panel_movimientos
    where texto_busqueda like '%' || public.plegar_texto('JOSE NANDU') || '%'
      and id = 'bbbbbbbb-0000-0000-0000-00000000b001'
  ) then
    raise exception 'texto_busqueda no ignora tildes/mayusculas';
  end if;
end $$;

-- Quien no es admin_global (ni un equipo de ese sitio) no ve nada.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_normal'))::text,
  true);
do $$
begin
  if exists (select 1 from public.panel_movimientos where id = 'bbbbbbbb-0000-0000-0000-00000000b001') then
    raise exception 'Un usuario sin permiso ve movimientos en panel_movimientos';
  end if;
end $$;

-- anon no puede consultar la vista.
reset role;
set local role anon;
do $$
begin
  perform 1 from public.panel_movimientos limit 1;
  raise exception 'anon pudo leer panel_movimientos';
exception
  when insufficient_privilege then null;
end $$;

rollback;
