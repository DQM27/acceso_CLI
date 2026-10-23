-- Ejecutar en una sola sesión. Todo se revierte.
-- Cubre `lienzo_vistas_guardadas`: cada administrador guarda, ve, cambia y
-- borra sólo sus vistas; no puede guardar a nombre de otro; quien no es admin
-- no ve ni guarda nada; anon no tiene permisos; límites de la tabla.
begin;

select set_config('diagnostico.admin', 'diagnostico-vistas-admin@example.com', true),
       set_config('diagnostico.otro', 'diagnostico-vistas-otro@example.com', true),
       set_config('diagnostico.normal', 'diagnostico-vistas-normal@example.com', true);

insert into public.administradores_panel (correo) values
  (current_setting('diagnostico.admin')),
  (current_setting('diagnostico.otro'));

-- La vista del otro admin (con SQL adentro: no debe verla nadie más).
insert into public.lienzo_vistas_guardadas (correo, nombre, lienzo) values
  (current_setting('diagnostico.otro'), 'Proveedores del mes',
   '{"fuente": "consulta", "consulta": {"sql": "select 1 as n from ia.sitios", "columnas": []}, "filtros": [], "widgets": []}');

set local role authenticated;
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.admin'))::text, true);

do $$
declare
  v_id uuid;
begin
  -- Guarda, ve, cambia y borra lo suyo.
  insert into public.lienzo_vistas_guardadas (correo, nombre, lienzo)
  values (current_setting('diagnostico.admin'), 'Vencidos', '{"fuente": "contratistas", "filtros": [], "widgets": []}')
  returning id into v_id;
  if (select count(*) from public.lienzo_vistas_guardadas where correo like 'diagnostico-vistas-%') <> 1 then
    raise exception 'Un admin ve vistas que no son suyas (o no ve la suya)';
  end if;
  update public.lienzo_vistas_guardadas set nombre = 'Vencidos 2', actualizado_en = now() where id = v_id;
  if not found then raise exception 'Un admin no pudo renombrar su vista'; end if;

  -- El mismo nombre dos veces no.
  begin
    insert into public.lienzo_vistas_guardadas (correo, nombre, lienzo)
    values (current_setting('diagnostico.admin'), 'Vencidos 2', '{}');
    raise exception 'Aceptó dos vistas con el mismo nombre';
  exception when unique_violation then null; end;

  -- No puede guardar a nombre de otro, ni pasarle su vista.
  begin
    insert into public.lienzo_vistas_guardadas (correo, nombre, lienzo)
    values (current_setting('diagnostico.otro'), 'Intrusa', '{}');
    raise exception 'Un admin guardó una vista a nombre de otro';
  exception when insufficient_privilege then null; end;
  begin
    update public.lienzo_vistas_guardadas set correo = current_setting('diagnostico.otro') where id = v_id;
    raise exception 'Un admin pasó su vista a otro';
  exception when insufficient_privilege then null; end;

  -- No toca las del otro.
  update public.lienzo_vistas_guardadas set nombre = 'tocada' where correo = current_setting('diagnostico.otro');
  if found then raise exception 'Un admin cambió la vista de otro'; end if;
  delete from public.lienzo_vistas_guardadas where correo = current_setting('diagnostico.otro');
  if found then raise exception 'Un admin borró la vista de otro'; end if;

  -- Límites.
  begin
    insert into public.lienzo_vistas_guardadas (correo, nombre, lienzo) values (current_setting('diagnostico.admin'), '   ', '{}');
    raise exception 'Aceptó un nombre vacío';
  exception when check_violation then null; end;
  begin
    insert into public.lienzo_vistas_guardadas (correo, nombre, lienzo) values (current_setting('diagnostico.admin'), 'x', '[]');
    raise exception 'Aceptó un documento que no es un objeto';
  exception when check_violation then null; end;

  delete from public.lienzo_vistas_guardadas where id = v_id;
  if not found then raise exception 'Un admin no pudo borrar su vista'; end if;
end $$;

-- Quien no es admin no ve ni guarda nada.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.normal'))::text, true);
do $$
begin
  if exists (select 1 from public.lienzo_vistas_guardadas where correo like 'diagnostico-vistas-%') then
    raise exception 'Quien no es admin ve vistas guardadas';
  end if;
  begin
    insert into public.lienzo_vistas_guardadas (correo, nombre, lienzo)
    values (current_setting('diagnostico.normal'), 'Mía', '{}');
    raise exception 'Quien no es admin pudo guardar una vista';
  exception when insufficient_privilege then null; end;
end $$;

reset role;
do $$
begin
  if has_table_privilege('anon', 'public.lienzo_vistas_guardadas', 'select')
     or has_table_privilege('anon', 'public.lienzo_vistas_guardadas', 'insert') then
    raise exception 'anon tiene permisos sobre lienzo_vistas_guardadas';
  end if;
end $$;
set local role anon;
do $$
begin
  perform 1 from public.lienzo_vistas_guardadas limit 1;
  raise exception 'anon pudo leer lienzo_vistas_guardadas';
exception
  when insufficient_privilege then null;
end $$;

rollback;
