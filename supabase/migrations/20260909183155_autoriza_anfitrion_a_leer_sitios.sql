-- Un anfitrión necesita ver la lista de sitios para elegir a cuál(es)
-- invita al agendar una cita (ver `cita_sitios`) -- hasta ahora `sitios`
-- sólo lo podía leer `es_admin_global()` (ver admin_global_lee_sitios).
create policy "anfitrion lee sitios"
  on public.sitios
  for select
  to authenticated
  using (exists (select 1 from public.anfitriones a where a.correo = auth.email()));
