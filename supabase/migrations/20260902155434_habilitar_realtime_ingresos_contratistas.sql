-- Habilita el relevo en vivo (Postgres Changes) para el caso PC↔celular
-- del mismo sitio. RLS ya filtra qué fila le llega a cada dispositivo.
alter publication supabase_realtime add table ingresos;
alter publication supabase_realtime add table contratistas;
