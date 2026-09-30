-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
-- Cubre `panel_buscar_movimientos`: sólo un admin_global puede usarla (corre
-- sin RLS, así que valida el permiso ella misma), cédula por prefijo, nombre
-- por palabras sin tildes ni mayúsculas, y fechas/unidades aplicadas adentro.
begin;

select set_config('diagnostico.correo_admin', 'diagnostico-admin@example.com', true),
       set_config('diagnostico.correo_normal', 'diagnostico-normal@example.com', true);

insert into public.administradores_panel (correo)
values (current_setting('diagnostico.correo_admin'));

insert into public.sitios (id, nombre)
values ('aaaaaaaa-0000-0000-0000-00000000a101', 'Unidad búsqueda A'),
       ('aaaaaaaa-0000-0000-0000-00000000a102', 'Unidad búsqueda B');

insert into public.dispositivos (id, sitio_id, tipo, etiqueta, clave_publica_jwk, clave_huella)
values ('cccccccc-0000-0000-0000-00000000c101', 'aaaaaaaa-0000-0000-0000-00000000a101',
        'pc', 'Diagnóstico búsqueda', '{"kty":"EC"}', 'diag-huella-busqueda');

insert into public.contratistas (id, dispositivo_origen_id, nombre)
values ('dddddddd-0000-0000-0000-00000000d101',
        'cccccccc-0000-0000-0000-00000000c101', 'María Zúñiga Solano');

insert into public.ingresos (id, sitio_id, dispositivo_entrada_id, contratista_id,
                             contratista_cedula, contratista_nombre, hora_entrada)
values
  -- Hoy, unidad A.
  ('bbbbbbbb-0000-0000-0000-00000000b101', 'aaaaaaaa-0000-0000-0000-00000000a101',
   'cccccccc-0000-0000-0000-00000000c101', 'dddddddd-0000-0000-0000-00000000d101',
   '987650001', 'María Zúñiga Solano', now()),
  -- Hace 40 días, unidad B.
  ('bbbbbbbb-0000-0000-0000-00000000b102', 'aaaaaaaa-0000-0000-0000-00000000a102',
   'cccccccc-0000-0000-0000-00000000c101', 'dddddddd-0000-0000-0000-00000000d101',
   '987650001', 'María Zúñiga Solano', now() - interval '40 days'),
  -- Otra persona cuya cédula contiene (pero no empieza con) el prefijo.
  ('bbbbbbbb-0000-0000-0000-00000000b103', 'aaaaaaaa-0000-0000-0000-00000000a101',
   'cccccccc-0000-0000-0000-00000000c101', 'dddddddd-0000-0000-0000-00000000d101',
   '198765000', 'Pedro Araya', now());

set local role authenticated;

select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_admin'))::text,
  true);
do $$
declare
  v_ids uuid[];
begin
  -- Cédula por prefijo, con guiones y un cero inicial como los teclea la gente.
  select array_agg(id order by id) into v_ids
    from public.panel_buscar_movimientos('0-9876-5')
   where id::text like 'bbbbbbbb-0000-0000-0000-00000000b1%';
  if v_ids is distinct from array['bbbbbbbb-0000-0000-0000-00000000b101',
                                  'bbbbbbbb-0000-0000-0000-00000000b102']::uuid[] then
    raise exception 'La búsqueda por prefijo de cédula devolvió %', v_ids;
  end if;

  -- Nombre: palabras en cualquier orden, sin tildes ni mayúsculas.
  select array_agg(id order by id) into v_ids
    from public.panel_buscar_movimientos('ZUNIGA maría')
   where id::text like 'bbbbbbbb-0000-0000-0000-00000000b1%';
  if v_ids is distinct from array['bbbbbbbb-0000-0000-0000-00000000b101',
                                  'bbbbbbbb-0000-0000-0000-00000000b102']::uuid[] then
    raise exception 'La búsqueda por nombre devolvió %', v_ids;
  end if;

  -- Comodines de like en el texto se buscan literales.
  if exists (select 1 from public.panel_buscar_movimientos('%')) then
    raise exception 'Un %% suelto no debe coincidir con todo';
  end if;

  -- Fechas y unidades se aplican adentro de la función.
  select array_agg(id order by id) into v_ids
    from public.panel_buscar_movimientos('maria', now() - interval '1 day', null,
                                         array['aaaaaaaa-0000-0000-0000-00000000a101']::uuid[]);
  if v_ids is distinct from array['bbbbbbbb-0000-0000-0000-00000000b101']::uuid[] then
    raise exception 'Fechas/unidades no se aplicaron: %', v_ids;
  end if;

  -- Devuelve filas con la forma de la vista (unidad ya resuelta).
  if not exists (
    select 1 from public.panel_buscar_movimientos('987650001')
     where id = 'bbbbbbbb-0000-0000-0000-00000000b101' and sitio_nombre = 'Unidad búsqueda A'
  ) then
    raise exception 'La fila no trae el nombre de la unidad';
  end if;
end $$;

-- Quien no es admin_global no puede usarla (la función corre sin RLS).
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_normal'))::text,
  true);
do $$
begin
  perform 1 from public.panel_buscar_movimientos('maria');
  raise exception 'Un usuario sin permiso pudo buscar movimientos';
exception
  when insufficient_privilege then null;
end $$;

-- anon ni siquiera puede ejecutarla.
reset role;
set local role anon;
do $$
begin
  perform 1 from public.panel_buscar_movimientos('maria');
  raise exception 'anon pudo ejecutar panel_buscar_movimientos';
exception
  when insufficient_privilege then null;
end $$;

rollback;
