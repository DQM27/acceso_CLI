-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
begin;

-- Dos correos de prueba: uno admin_global real (nunca se toca; solo lo usamos
-- para pasar la condición de es_admin_global()), y uno normal.
select set_config('diagnostico.correo_admin', 'diagnostico-admin@example.com', true),
       set_config('diagnostico.correo_normal', 'diagnostico-normal@example.com', true);

insert into public.administradores_panel (correo)
values (current_setting('diagnostico.correo_admin'));

set local role authenticated;

-- Un admin_global puede leer TODAS las filas, no solo la propia.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_admin'))::text,
  true);
do $$
begin
  if (select count(*) from public.administradores_panel) < 1 then
    raise exception 'admin_global no puede leer administradores_panel';
  end if;
end $$;

-- Alguien que NO está en la tabla no ve ninguna fila (ni siquiera la de otros).
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_normal'))::text,
  true);
do $$
begin
  if exists (select 1 from public.administradores_panel) then
    raise exception 'Un correo fuera de administradores_panel puede leer la tabla';
  end if;
end $$;

-- Ese mismo correo tampoco puede agregarse a sí mismo como admin.
do $$
begin
  insert into public.administradores_panel (correo) values (current_setting('diagnostico.correo_normal'));
  raise exception 'Un correo no-admin pudo insertarse a sí mismo en administradores_panel';
exception
  when insufficient_privilege then null;
end $$;

-- Un admin_global SÍ puede agregar a otro correo.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_admin'))::text,
  true);
do $$
begin
  insert into public.administradores_panel (correo) values (current_setting('diagnostico.correo_normal'));
  if not found then
    raise exception 'admin_global no pudo agregar un nuevo administrador';
  end if;
end $$;

-- Un admin_global no puede borrarse a sí mismo (evita quedarse sin ningún
-- admin_global si es el único).
do $$
begin
  delete from public.administradores_panel where correo = current_setting('diagnostico.correo_admin');
  if found then
    raise exception 'admin_global pudo borrar su propia fila';
  end if;
end $$;

-- Pero sí puede borrar a otro admin.
do $$
begin
  delete from public.administradores_panel where correo = current_setting('diagnostico.correo_normal');
  if not found then
    raise exception 'admin_global no pudo borrar a otro administrador';
  end if;
end $$;

select '6 comprobaciones de autorización correctas' as resultado;
rollback;
