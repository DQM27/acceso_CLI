-- Documenta un cambio que ya está en producción pero nunca quedó como
-- migración en el repo (drift real -- si alguien reconstruye la base desde
-- cero con `supabase db push`, le faltaría este paso). Verificado contra
-- pg_policy en vivo: las políticas de select/update de contratistas y
-- empresas ya son "(global)" con using/with check = true, no por sitio.
--
-- El modelo de contratistas/empresas se decidió global desde el principio
-- (ver 20260905063724_admin_global_gestiona_contratistas.sql: "el modelo
-- de contratistas ya se decidió global") -- un contratista no pertenece a
-- un sitio, puede entrar en cualquier unidad operativa salvo que se le
-- niegue el acceso (`tiene_acceso`), y esa baja tiene que verse en todos
-- los sitios a la vez. `sitio_id` en ambas tablas queda como dato
-- informativo ("de dónde es"/procedencia), nunca como filtro de lectura --
-- por eso el INSERT sigue acotado al sitio del dispositivo que lo crea,
-- pero SELECT/UPDATE no.

-- Los DROP cubren tanto el nombre viejo (por si esto corre contra una base
-- reconstruida desde cero, donde todavía existe) como el nuevo (por si
-- corre contra la base real, donde el rename ya está aplicado) -- para que
-- esta migración sea idempotente en los dos escenarios.

drop policy if exists "leer contratistas del propio sitio" on public.contratistas;
drop policy if exists "leer contratistas (global)" on public.contratistas;
create policy "leer contratistas (global)"
  on public.contratistas
  for select
  to authenticated
  using (true);

drop policy if exists "actualizar contratistas del propio sitio" on public.contratistas;
drop policy if exists "actualizar contratistas (global)" on public.contratistas;
create policy "actualizar contratistas (global)"
  on public.contratistas
  for update
  to authenticated
  using (true)
  with check (true);

drop policy if exists "leer empresas del propio sitio" on public.empresas;
drop policy if exists "leer empresas (global)" on public.empresas;
create policy "leer empresas (global)"
  on public.empresas
  for select
  to authenticated
  using (true);

drop policy if exists "actualizar empresas del propio sitio" on public.empresas;
drop policy if exists "actualizar empresas (global)" on public.empresas;
create policy "actualizar empresas (global)"
  on public.empresas
  for update
  to authenticated
  using (true)
  with check (true);
