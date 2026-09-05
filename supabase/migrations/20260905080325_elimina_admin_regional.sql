-- admin_regional nunca tuvo alcance real (administradores_panel no
-- guardaba qué sitios administraba cada quien) y se decidió eliminar el
-- concepto en vez de construirlo: administradores_panel vuelve a ser una
-- lista simple de "quién puede entrar", sin distinción de rol.
alter table public.administradores_panel drop column rol;

create or replace function public.es_admin_global()
returns boolean
language sql
stable security definer
set search_path = public
as $$
  select exists (
    select 1 from public.administradores_panel
    where correo = auth.email()
  );
$$;
