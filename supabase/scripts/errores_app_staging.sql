-- Errores de las apps: el "Sentry propio" (ver
-- docs/features-futuras/plan-telemetria-ajustable.md, fases 4 y 5).
--
-- SÓLO para el proyecto de STAGING (sandbox). A propósito NO está en
-- supabase/migrations/: producción no recibe esta tabla. Se aplica a mano
-- (SQL Editor) contra staging. Es idempotente.
--
-- Cómo funciona: la Edge Function `receptor-errores`
-- (supabase/functions-staging/receptor-errores) recibe lo que manda el SDK de
-- Sentry, lo limpia de datos personales y lo guarda con `registrar_error_app`.
-- La app (rol `anon`) no puede leer ni escribir esta tabla.
--
-- Estado de un error: se cambia desde el SQL Editor, por huella:
--   update public.errores_app set estado = 'resuelto' where huella = '<huella>';
--
-- Retención: `limpiar_errores_app()` borra lo que tiene más de 90 días; queda
-- programada cada noche con pg_cron (el script lo instala).
begin;

create table if not exists public.errores_app (
    id           bigint generated always as identity primary key,
    recibido_en  timestamptz not null default now(),
    ocurrido_en  timestamptz,
    -- Id del evento que asigna el SDK: evita guardar dos veces un reintento.
    event_id     text not null unique,
    plataforma   text not null check (plataforma in ('movil', 'escritorio', 'otra')),
    version_app  text,
    entorno      text,
    nivel        text not null,
    tipo         text not null check (tipo in ('excepcion', 'mensaje')),
    -- Agrupa los errores iguales (ver `calcularHuella` en logica.ts).
    huella       text not null,
    -- Siempre ya limpios de datos personales (ver `limpiarTexto`).
    titulo       text not null,
    traza        jsonb not null default '[]'::jsonb,
    contexto     jsonb not null default '{}'::jsonb,
    estado       text not null default 'nuevo' check (estado in ('nuevo', 'visto', 'resuelto', 'ignorado'))
);

comment on table public.errores_app is
    'Errores de las apps recibidos por receptor-errores (sólo staging). Sin datos personales por construcción. Ver docs/features-futuras/plan-telemetria-ajustable.md.';

create index if not exists errores_app_huella_idx on public.errores_app (huella, recibido_en desc);
create index if not exists errores_app_recibido_idx on public.errores_app (recibido_en desc);

alter table public.errores_app enable row level security;
revoke all on public.errores_app from anon, authenticated;

-- Los errores agrupados: una fila por huella. Es lo que se mira en Grafana.
create or replace view public.errores_agrupados as
select
    huella,
    plataforma,
    max(titulo)                                              as titulo,
    count(*)                                                 as eventos,
    min(recibido_en)                                         as primera_vez,
    max(recibido_en)                                         as ultima_vez,
    array_agg(distinct version_app) filter (where version_app is not null) as versiones,
    (array_agg(estado order by recibido_en desc))[1]         as estado
from public.errores_app
group by huella, plataforma;

revoke all on public.errores_agrupados from anon, authenticated;

-- Guarda un error ya limpio. `security definer` y sólo para el rol de servicio:
-- lo llama la Edge Function. Devuelve 'ok', 'repetido' (el SDK reintentó) o
-- 'limite' (más de 120 errores en el último minuto: alguien está inundando).
create or replace function public.registrar_error_app(p jsonb)
returns text
language plpgsql
security definer
set search_path = ''
as $$
declare
    v_filas integer;
begin
    if (select count(*) from public.errores_app where recibido_en > now() - interval '1 minute') >= 120 then
        return 'limite';
    end if;

    insert into public.errores_app (
        ocurrido_en, event_id, plataforma, version_app, entorno, nivel, tipo,
        huella, titulo, traza, contexto
    ) values (
        nullif(p->>'ocurrido_en', '')::timestamptz,
        left(p->>'event_id', 64),
        p->>'plataforma',
        left(p->>'version_app', 80),
        left(p->>'entorno', 80),
        left(coalesce(p->>'nivel', 'error'), 20),
        p->>'tipo',
        left(p->>'huella', 32),
        left(p->>'titulo', 300),
        coalesce(p->'traza', '[]'::jsonb),
        coalesce(p->'contexto', '{}'::jsonb)
    )
    on conflict (event_id) do nothing;

    get diagnostics v_filas = row_count;
    return case when v_filas = 0 then 'repetido' else 'ok' end;
end;
$$;

revoke all on function public.registrar_error_app(jsonb) from public, anon, authenticated;
grant execute on function public.registrar_error_app(jsonb) to service_role;

-- Retención: 90 días.
create or replace function public.limpiar_errores_app()
returns integer
language plpgsql
security definer
set search_path = ''
as $$
declare
    v_borradas integer;
begin
    delete from public.errores_app where recibido_en < now() - interval '90 days';
    get diagnostics v_borradas = row_count;
    return v_borradas;
end;
$$;

revoke all on function public.limpiar_errores_app() from public, anon, authenticated;

-- Grafana, de sólo lectura (ver telemetria_lector_grafana_staging.sql).
do $$
begin
    if exists (select 1 from pg_roles where rolname = 'grafana_lector') then
        grant select on public.errores_app to grafana_lector;
        grant select on public.errores_agrupados to grafana_lector;
        drop policy if exists errores_lector_grafana on public.errores_app;
        create policy errores_lector_grafana on public.errores_app
            for select to grafana_lector using (true);
    end if;
end
$$;

-- Cada noche a las 03:17 (hora del servidor). Antes la programación dependía de
-- que pg_cron ya estuviera instalado, y en staging no lo estaba: la limpieza
-- nunca corrió (se notó el 2026-10-20). Ahora el script lo instala.
create extension if not exists pg_cron with schema pg_catalog;
select cron.schedule('limpiar-errores-app', '17 3 * * *', 'select public.limpiar_errores_app()');

commit;
