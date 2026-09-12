-- CREATE OR REPLACE con un parámetro nuevo al final no reemplazó la
-- función existente -- Postgres identifica funciones por tipo de
-- argumentos, así que (uuid,date,date,text,uuid[],jsonb) y
-- (uuid,date,date,text,uuid[],jsonb,time) quedaron como DOS sobrecargas
-- distintas, la vieja sin hora_estimada ni el hash actualizado. Se borra
-- la vieja -- PostgREST sigue resolviendo llamadas sin `hora_estimada` en
-- el cuerpo contra la que queda, gracias al `default null`.
drop function public.crear_cita_anfitrion(uuid, date, date, text, uuid[], jsonb);
