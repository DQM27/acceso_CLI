-- Hora del servidor en milisegundos, para medir el desfase del reloj de
-- cada equipo con precisión de milisegundos (al estilo NTP, ver
-- `src/nube/reloj_preciso.rs`). Antes el desfase se medía sólo con el
-- header HTTP `Date`, que trae segundos enteros: error de hasta 1 s.
--
-- `clock_timestamp()` (no `now()`): la hora real en el momento de
-- ejecutarse, no la del inicio de la transacción.
--
-- Sin datos de nadie: sólo devuelve la hora, por eso se permite también a
-- `anon`. Las apps que no la encuentran (proyecto sin esta migración)
-- siguen con el header `Date`, así que se puede aplicar antes o después
-- de actualizarlas.
create or replace function public.hora_servidor_ms()
returns bigint
language sql
volatile
security invoker
set search_path = ''
as $$
  select (extract(epoch from pg_catalog.clock_timestamp()) * 1000)::bigint
$$;

comment on function public.hora_servidor_ms() is
  'Hora del servidor en milisegundos desde 1970 (UTC). La usan los equipos para medir el desfase de su reloj (src/nube/reloj_preciso.rs).';

revoke all on function public.hora_servidor_ms() from public;
grant execute on function public.hora_servidor_ms() to anon, authenticated;
