-- `sitios` tenía RLS habilitado pero CERO políticas desde el arranque del
-- proyecto (nadie podía leerlo, ni siquiera un dispositivo) -- el panel
-- web necesita el nombre del sitio para mostrarlo en el historial
-- multi-sitio, no sólo el sitio_id crudo.
create policy "admin_global lee sitios"
  on public.sitios
  for select
  to authenticated
  using (public.es_admin_global());
