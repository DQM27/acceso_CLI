-- El panel web necesita el tipo de dispositivo (pc/mobile/visor) para
-- mostrar la columna "Dispositivo" en el historial multi-sitio, igual que
-- ya hace con sitios (ver admin_global_lee_sitios). Sin esto, el embed
-- `dispositivos!ingresos_dispositivo_entrada_id_fkey(tipo)` vuelve null
-- por RLS, no un error -- silencioso y confuso.
create policy "admin_global lee dispositivos"
  on public.dispositivos
  for select
  to authenticated
  using (public.es_admin_global());
