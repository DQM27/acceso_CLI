-- Delega la creación de Administrador/Operador al panel web (docs/
-- plan-panel-administrativo-web.md, punto 4 -- "Delegar la creación de
-- usuarios... al panel web", ver docs/pendientes.md). Hasta ahora sólo
-- había política de INSERT para un dispositivo autenticado ("crear
-- usuarios del propio sitio", exige sitio_id = auth.jwt()->>'sitio_id') --
-- el JWT de admin_global (Supabase Auth por Google, sin sitio_id) nunca
-- pasaba ese check.
--
-- El usuario nuevo se crea sin password_hash (columna que ni existe en
-- esta tabla) -- entra al mismo mecanismo SIN_PASSWORD_LOCAL que ya usan
-- los operadores creados localmente: el primer dispositivo donde esa
-- cédula inicia sesión es el que fija la contraseña real.
create policy "admin_global crea usuarios"
  on public.usuarios
  for insert
  to authenticated
  with check (public.es_admin_global());
