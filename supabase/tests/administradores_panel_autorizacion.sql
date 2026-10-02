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

-- Ni siquiera un admin_global puede agregar o quitar administradores con su
-- sesión: la migración `retira_escritura_directa_de_administradores_panel`
-- quitó esas políticas a propósito (una sesión de admin robada no debe poder
-- fabricarse más acceso). El alta/baja se hace con service_role.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_admin'))::text,
  true);
do $$
begin
  insert into public.administradores_panel (correo) values (current_setting('diagnostico.correo_normal'));
  raise exception 'admin_global pudo agregar un administrador con su sesión';
exception
  when insufficient_privilege then null;
end $$;

-- Tampoco puede borrar a nadie: sin política de borrado, el DELETE no
-- alcanza ninguna fila (ni la propia ni la de otro).
reset role;
insert into public.administradores_panel (correo)
values (current_setting('diagnostico.correo_normal'));
set local role authenticated;
do $$
begin
  delete from public.administradores_panel
   where correo in (current_setting('diagnostico.correo_admin'),
                    current_setting('diagnostico.correo_normal'));
  if found then
    raise exception 'admin_global pudo borrar administradores con su sesión';
  end if;
end $$;

select '5 comprobaciones de autorización correctas' as resultado;
rollback;
