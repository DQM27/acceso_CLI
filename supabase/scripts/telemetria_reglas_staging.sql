-- Reglas de telemetría: el interruptor remoto de la medición (ver
-- docs/features-futuras/plan-telemetria-ajustable.md, fase 1).
--
-- SÓLO para el proyecto de STAGING (sandbox). A propósito NO está en
-- supabase/migrations/: producción no recibe esta tabla ni esta función.
-- Se aplica a mano (SQL Editor o `supabase db execute -f este-archivo`)
-- contra staging. Es idempotente.
--
-- Cómo funciona: la telemetría está APAGADA por defecto en las apps. Al
-- abrir y después de cada login (sin sondeo) la app pregunta a
-- `reglas_telemetria_vigentes` con su UUID de instalación, su plataforma y
-- su versión. Un cambio se nota en el siguiente arranque o login. Si hay una regla
-- vigente que le toca, mide con el detalle que pide; si no, no manda nada.
-- Toda regla vence (por defecto en 24 horas): apagar es borrar la regla o
-- poner `activa = false`, y si se olvida se apaga sola.
--
-- Acceso: las reglas se crean y se apagan desde el SQL Editor (rol
-- `postgres`). La app (rol `anon`) no puede leer la tabla: sólo puede
-- llamar a la función, que devuelve únicamente las reglas que le tocan a
-- quien pregunta.
begin;

create table if not exists public.telemetria_reglas (
    id           bigint generated always as identity primary key,
    creada_en    timestamptz not null default now(),
    vence_en     timestamptz not null default now() + interval '24 hours',
    activa       boolean not null default true,
    -- Para qué se creó (se ve en el SQL Editor y en Grafana).
    nota         text not null default '',
    -- A quién le toca. `null` = a cualquiera. Todos los filtros que no son
    -- `null` deben cumplirse a la vez.
    dispositivo  text,        -- UUID de la instalación (telemetria_dispositivo)
    plataforma   text check (plataforma in ('movil', 'escritorio')),
    version_desde text,       -- prefijo de `version_app`, p. ej. 'desktop-1.8.'
    -- Qué tipos de evento se mandan. `null` = todos.
    eventos      text[],
    -- 'ligero': las métricas de siempre. 'diagnostico': además el detalle.
    nivel        text not null default 'ligero' check (nivel in ('ligero', 'diagnostico'))
);

comment on table public.telemetria_reglas is
    'Interruptor remoto de la telemetría (sólo staging). Sin regla vigente, las apps no miden. Ver docs/features-futuras/plan-telemetria-ajustable.md.';

alter table public.telemetria_reglas enable row level security;
revoke all on public.telemetria_reglas from anon, authenticated;

-- Las reglas que le tocan a quien pregunta. `security definer` para que
-- `anon` no necesite leer la tabla; `search_path` vacío por seguridad.
create or replace function public.reglas_telemetria_vigentes(
    p_dispositivo text,
    p_plataforma  text,
    p_version     text
)
returns table (id bigint, nivel text, eventos text[], vence_en timestamptz)
language sql
stable
security definer
set search_path = ''
as $$
    select r.id, r.nivel, r.eventos, r.vence_en
    from public.telemetria_reglas r
    where r.activa
      and r.vence_en > now()
      and (r.dispositivo is null or r.dispositivo = left(p_dispositivo, 80))
      and (r.plataforma is null or r.plataforma = left(p_plataforma, 20))
      and (r.version_desde is null or starts_with(left(p_version, 80), r.version_desde))
    order by r.id
    limit 20;
$$;

revoke all on function public.reglas_telemetria_vigentes(text, text, text) from public;
grant execute on function public.reglas_telemetria_vigentes(text, text, text) to anon, authenticated;

commit;

-- Uso (desde el SQL Editor):
--
-- Encender la medición de una instalación por 24 h:
--   insert into public.telemetria_reglas (dispositivo, nota)
--   values ('<uuid de telemetria_dispositivo>', 'por qué se pega el ingreso');
--
-- Encender para todas las PC de escritorio 1.8.x por 2 h:
--   insert into public.telemetria_reglas (plataforma, version_desde, vence_en, nota)
--   values ('escritorio', 'desktop-1.8.', now() + interval '2 hours', 'prueba de flota');
--
-- Apagar todo ya:
--   update public.telemetria_reglas set activa = false where activa;
--
-- Ver las vigentes:
--   select * from public.telemetria_reglas where activa and vence_en > now();
