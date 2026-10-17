-- Retención de la telemetría de diagnóstico: se conservan 30 días.
--
-- SÓLO para el proyecto de STAGING (sandbox), igual que
-- telemetria_diagnostico_staging.sql. Se aplica a mano (SQL Editor) y es
-- idempotente.
--
-- Por qué: una PC con la telemetría encendida manda unas 400 filas por hora y
-- una regla (`telemetria_reglas`) puede durar días. Sin un tope la tabla crece
-- sin límite y llena el plan gratis de la base. Se cuenta desde `recibido_en`
-- (la hora del servidor) y no desde `ocurrido_en`: el reloj de un equipo mal
-- puesto no debe librar a sus filas de la limpieza.
--
-- Cómo se usa:
--   select public.limpiar_telemetria_diagnostico();   -- devuelve cuántas borró
-- Si pg_cron está instalado, el final de este script la programa cada noche; si
-- no, hay que correrla a mano de vez en cuando (por ejemplo, junto con
-- `limpiar_errores_app()`).
begin;

-- Borrar por fecha de recepción sin recorrer toda la tabla.
create index if not exists telemetria_diagnostico_recibido_idx
    on public.telemetria_diagnostico (recibido_en);

create or replace function public.limpiar_telemetria_diagnostico()
returns integer
language plpgsql
security definer
set search_path = ''
as $$
declare
    v_borradas integer;
begin
    delete from public.telemetria_diagnostico
    where recibido_en < now() - interval '30 days';
    get diagnostics v_borradas = row_count;
    return v_borradas;
end;
$$;

-- Sólo el dueño y el rol de servicio: ni la app (`anon`) ni un usuario con
-- sesión pueden borrar telemetría.
revoke all on function public.limpiar_telemetria_diagnostico() from public, anon, authenticated;
grant execute on function public.limpiar_telemetria_diagnostico() to service_role;

-- Cada noche a las 03:23 (hora del servidor), si pg_cron está instalado.
do $$
begin
    if exists (select 1 from pg_extension where extname = 'pg_cron') then
        perform cron.schedule(
            'limpiar-telemetria-diagnostico',
            '23 3 * * *',
            'select public.limpiar_telemetria_diagnostico()'
        );
    end if;
end
$$;

commit;
