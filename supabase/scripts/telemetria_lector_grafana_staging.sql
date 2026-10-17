-- Usuario de SÓLO LECTURA para que Grafana consulte la telemetría.
--
-- SÓLO para el proyecto de STAGING (sandbox), igual que
-- telemetria_diagnostico_staging.sql, que debe aplicarse antes. A propósito
-- NO está en supabase/migrations/: producción no tiene esa tabla ni este
-- usuario. Se aplica a mano (SQL Editor). Es idempotente.
--
-- Qué puede hacer: `select` sobre public.telemetria_diagnostico y nada más.
-- No ve otras tablas, no escribe, no salta la seguridad por fila
-- (bypassrls = false: por eso necesita su propia política de lectura, ya que
-- la tabla sólo permite insertar a `anon` y `authenticated`). Tope de 15 s
-- por consulta y 5 conexiones.
--
-- LA CONTRASEÑA NO VA EN ESTE ARCHIVO NI EN NINGÚN OTRO DEL REPO. Después de
-- aplicarlo, quien administra la base la fija a mano y la escribe sólo en la
-- fuente de datos de Grafana:
--
--   alter role grafana_lector password '<contraseña larga y propia>';
--
-- Mientras no tenga contraseña, nadie puede entrar con este usuario.
-- Para quitarlo todo:
--
--   drop policy telemetria_lector_grafana on public.telemetria_diagnostico;
--   revoke all on public.telemetria_diagnostico from grafana_lector;
--   revoke usage on schema public from grafana_lector;
--   drop role grafana_lector;
--
-- Nota operativa: `create policy` pide un candado exclusivo sobre la tabla,
-- que la app mantiene ocupada insertando. Si se cuelga, aplicar sólo esa
-- sentencia con `set local lock_timeout = '5s';` antes y reintentar.

do $$
begin
    if not exists (select 1 from pg_roles where rolname = 'grafana_lector') then
        create role grafana_lector with login noinherit nosuperuser nocreatedb nocreaterole;
    end if;
end
$$;

grant usage on schema public to grafana_lector;
grant select on public.telemetria_diagnostico to grafana_lector;
alter role grafana_lector set statement_timeout = '15s';
alter role grafana_lector connection limit 5;

drop policy if exists telemetria_lector_grafana on public.telemetria_diagnostico;
create policy telemetria_lector_grafana
    on public.telemetria_diagnostico
    for select
    to grafana_lector
    using (true);
