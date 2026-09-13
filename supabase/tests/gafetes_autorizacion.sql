-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
--
-- `gafetes` está acotado a su propio sitio para leer/crear/actualizar --
-- sin política admin_global (a diferencia de contratistas/usuarios/
-- ingresos, que sí tienen acceso global para el panel web). Documentado:
-- si el panel web algún día necesita ver gafetes, hoy no puede.
begin;

insert into public.sitios (id, nombre) values
  (gen_random_uuid(), 'Diagnóstico A'),
  (gen_random_uuid(), 'Diagnóstico B');
select set_config('diagnostico.sitio_a', (select id::text from public.sitios where nombre = 'Diagnóstico A'), true),
       set_config('diagnostico.sitio_b', (select id::text from public.sitios where nombre = 'Diagnóstico B'), true),
       set_config('diagnostico.correo_admin', 'diagnostico-admin@example.com', true);

insert into public.dispositivos (id, sitio_id, tipo, etiqueta, secret_hash)
values (gen_random_uuid(), current_setting('diagnostico.sitio_a')::uuid, 'pc', 'Diagnóstico PC A', 'diag-hash-a');
select set_config('diagnostico.dispositivo_a', (select id::text from public.dispositivos where etiqueta = 'Diagnóstico PC A'), true);

insert into public.administradores_panel (correo) values (current_setting('diagnostico.correo_admin'));

set local role authenticated;

-- Crear/leer/actualizar dentro del propio sitio: permitido.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'sitio_id', current_setting('diagnostico.sitio_a'))::text,
  true);
do $$
begin
  insert into public.gafetes (id, sitio_id, dispositivo_origen_id, numero, estado)
  values (gen_random_uuid(), current_setting('diagnostico.sitio_a')::uuid, current_setting('diagnostico.dispositivo_a')::uuid, 999001, 'DISPONIBLE');
  if not found then
    raise exception 'Un dispositivo no pudo crear un gafete en su propio sitio';
  end if;

  update public.gafetes set estado = 'PERDIDO' where numero = 999001;
  if not found then
    raise exception 'Un dispositivo no pudo actualizar un gafete de su propio sitio';
  end if;
end $$;

-- Crear a nombre de otro sitio: bloqueado.
do $$
begin
  begin
    insert into public.gafetes (id, sitio_id, dispositivo_origen_id, numero, estado)
    values (gen_random_uuid(), current_setting('diagnostico.sitio_b')::uuid, current_setting('diagnostico.dispositivo_a')::uuid, 999002, 'DISPONIBLE');
    raise exception 'Un dispositivo pudo crear un gafete para otro sitio';
  exception
    when insufficient_privilege then null;
  end;
end $$;

-- Leer/actualizar el gafete de otro sitio: bloqueado.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'sitio_id', current_setting('diagnostico.sitio_b'))::text,
  true);
do $$
begin
  if exists (select 1 from public.gafetes where numero = 999001) then
    raise exception 'Un dispositivo puede leer gafetes de otro sitio';
  end if;
end $$;

-- admin_global HOY tampoco puede leer gafetes -- gap conocido, igual que
-- dispositivos.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_admin'))::text,
  true);
do $$
begin
  if exists (select 1 from public.gafetes where numero = 999001) then
    raise exception 'admin_global ya puede leer gafetes -- actualizar este comentario';
  end if;
end $$;

-- Mismo número, tipo distinto -- ya no colisiona (unicidad real es
-- (sitio_id, numero, tipo), no (sitio_id, numero) -- ver migración
-- agrega_tipo_y_portador_visita_a_gafetes).
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'sitio_id', current_setting('diagnostico.sitio_a'))::text,
  true);
do $$
begin
  insert into public.gafetes (id, sitio_id, dispositivo_origen_id, numero, tipo, estado)
  values (gen_random_uuid(), current_setting('diagnostico.sitio_a')::uuid, current_setting('diagnostico.dispositivo_a')::uuid, 999001, 'VISITA', 'DISPONIBLE');
  if not found then
    raise exception 'Un gafete de tipo VISITA no pudo coexistir con el mismo número que uno CONTRATISTA';
  end if;

  begin
    insert into public.gafetes (id, sitio_id, dispositivo_origen_id, numero, tipo, estado)
    values (gen_random_uuid(), current_setting('diagnostico.sitio_a')::uuid, current_setting('diagnostico.dispositivo_a')::uuid, 999001, 'CONTRATISTA', 'DISPONIBLE');
    raise exception 'Un gafete CONTRATISTA duplicado (mismo numero y tipo) no debio poder crearse';
  exception
    when unique_violation then null;
  end;
end $$;

select '5 comprobaciones de autorización correctas' as resultado;
rollback;
