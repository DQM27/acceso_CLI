-- El advisor de rendimiento de Supabase marcaba 9 foreign keys sin indice
-- (contratistas, empresas, gafetes, ingresos, usuarios) -- ralentiza JOINs
-- y borrados en cascada a medida que crecen las tablas. Sin riesgo para
-- RLS/Realtime: son indices puros, no tocan ninguna politica.
create index if not exists contratistas_dispositivo_origen_id_idx on public.contratistas(dispositivo_origen_id);
create index if not exists contratistas_empresa_id_idx on public.contratistas(empresa_id);
create index if not exists empresas_dispositivo_origen_id_idx on public.empresas(dispositivo_origen_id);
create index if not exists gafetes_contratista_deudor_id_idx on public.gafetes(contratista_deudor_id);
create index if not exists gafetes_dispositivo_origen_id_idx on public.gafetes(dispositivo_origen_id);
create index if not exists ingresos_contratista_id_idx on public.ingresos(contratista_id);
create index if not exists ingresos_dispositivo_entrada_id_idx on public.ingresos(dispositivo_entrada_id);
create index if not exists ingresos_dispositivo_salida_id_idx on public.ingresos(dispositivo_salida_id);
create index if not exists usuarios_dispositivo_origen_id_idx on public.usuarios(dispositivo_origen_id);
create index if not exists usuarios_sitio_id_idx on public.usuarios(sitio_id);
