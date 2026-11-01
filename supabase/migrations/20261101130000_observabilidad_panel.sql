-- Lectura de la telemetría para el panel (fase 4 de
-- docs/features-futuras/plan-observabilidad-en-produccion.md). Va después de
-- 20261101120000_observabilidad_telemetria.
--
-- Tres consultas, sólo para el administrador del panel (`es_admin_global`):
--
-- - `panel_telemetria_flota`: el último estado de cada equipo que manda
--   telemetría (versión, sistema, modelo, último lote, filas y descartes de
--   las últimas 24 h). Sale de `sesiones` y `lotes`, que son chicas.
-- - `panel_telemetria_rendimiento(p_dias)`: por equipo, versión y operación,
--   de los últimos 1 a 30 días del detalle (`llamadas`).
-- - `panel_telemetria_versiones(p_operacion, p_dias)`: por versión y
--   plataforma, del resumen diario (hasta un año): para ver si una versión
--   nueva empeoró una operación.
--
-- Mismo patrón que `registrar_telemetria`: la función real vive en
-- `observabilidad` (no expuesto por la API) y es `security definer`; en
-- `public` queda un envoltorio `security invoker`, así el linter no suma
-- avisos. Un equipo (JWT sin correo de administrador) o una sesión humana que
-- no es administradora reciben 42501.
--
-- Los percentiles son aproximados: percentil de los p50/p90 de cada ventana
-- de medición, no de cada llamada (igual que el resumen diario).

create function observabilidad.exigir_administrador()
returns void
language plpgsql
stable
security definer
set search_path = ''
as $$
begin
  if not coalesce((select private.es_admin_global()), false) then
    raise exception 'Sólo el administrador del panel puede leer la telemetría' using errcode = '42501';
  end if;
end;
$$;

revoke all on function observabilidad.exigir_administrador() from public;

-- ---------------------------------------------------------------------------
create function observabilidad.panel_telemetria_flota()
returns table (
  dispositivo_id uuid,
  etiqueta text,
  plataforma text,
  version_app text,
  version_codigo int,
  sistema text,
  modelo text,
  sesion_iniciada_en timestamptz,
  ultimo_lote_en timestamptz,
  filas_24h bigint,
  descartadas_24h bigint
)
language plpgsql
stable
security definer
set search_path = ''
as $$
begin
  perform observabilidad.exigir_administrador();
  return query
    with ultima as (
      select distinct on (s.dispositivo_id) s.*
        from observabilidad.sesiones s
       order by s.dispositivo_id, s.iniciada_en desc
    ), lotes as (
      select l.dispositivo_id,
             pg_catalog.max(l.recibido_en) as ultimo,
             coalesce(pg_catalog.sum(l.filas) filter (where l.recibido_en > pg_catalog.now() - interval '1 day'), 0)::bigint as filas,
             coalesce(pg_catalog.sum(l.descartadas) filter (where l.recibido_en > pg_catalog.now() - interval '1 day'), 0)::bigint as descartadas
        from observabilidad.lotes l
       group by l.dispositivo_id
    )
    select u.dispositivo_id, d.etiqueta, u.plataforma, u.version_app, u.version_codigo,
           u.sistema, u.modelo, u.iniciada_en, lo.ultimo,
           coalesce(lo.filas, 0), coalesce(lo.descartadas, 0)
      from ultima u
      left join public.dispositivos d on d.id = u.dispositivo_id
      left join lotes lo on lo.dispositivo_id = u.dispositivo_id
     order by lo.ultimo desc nulls last;
end;
$$;

revoke all on function observabilidad.panel_telemetria_flota() from public;
grant execute on function observabilidad.panel_telemetria_flota() to authenticated, service_role;

-- ---------------------------------------------------------------------------
create function observabilidad.panel_telemetria_rendimiento(p_dias int)
returns table (
  dispositivo_id uuid,
  etiqueta text,
  plataforma text,
  version_app text,
  operacion text,
  llamadas bigint,
  errores bigint,
  p50_ms numeric,
  p90_ms numeric,
  max_ms numeric,
  ultima_en timestamptz
)
language plpgsql
stable
security definer
set search_path = ''
as $$
begin
  perform observabilidad.exigir_administrador();
  return query
    select l.dispositivo_id, d.etiqueta, s.plataforma, s.version_app, l.operacion,
           pg_catalog.sum(l.llamadas)::bigint,
           pg_catalog.sum(l.errores)::bigint,
           pg_catalog.round((pg_catalog.percentile_cont(0.5) within group (order by l.p50_ms))::numeric, 1),
           pg_catalog.round((pg_catalog.percentile_cont(0.9) within group (order by l.p90_ms))::numeric, 1),
           pg_catalog.max(l.max_ms),
           pg_catalog.max(l.ocurrido_en)
      from observabilidad.llamadas l
      join observabilidad.sesiones s on s.id = l.sesion_id
      left join public.dispositivos d on d.id = l.dispositivo_id
     where l.ocurrido_en >= pg_catalog.now()
                            - pg_catalog.make_interval(days => pg_catalog.least(pg_catalog.greatest(coalesce(p_dias, 1), 1), 30))
     group by l.dispositivo_id, d.etiqueta, s.plataforma, s.version_app, l.operacion
     order by l.operacion, s.version_app desc, d.etiqueta;
end;
$$;

revoke all on function observabilidad.panel_telemetria_rendimiento(int) from public;
grant execute on function observabilidad.panel_telemetria_rendimiento(int) to authenticated, service_role;

-- ---------------------------------------------------------------------------
create function observabilidad.panel_telemetria_versiones(p_operacion text, p_dias int)
returns table (
  plataforma text,
  version_app text,
  primer_dia date,
  ultimo_dia date,
  equipos bigint,
  llamadas bigint,
  errores bigint,
  p50_ms numeric,
  p90_ms numeric,
  max_ms numeric
)
language plpgsql
stable
security definer
set search_path = ''
as $$
begin
  perform observabilidad.exigir_administrador();
  return query
    select r.plataforma, r.version_app,
           pg_catalog.min(r.dia), pg_catalog.max(r.dia),
           pg_catalog.count(distinct r.dispositivo_id),
           pg_catalog.sum(r.llamadas)::bigint,
           pg_catalog.sum(r.errores)::bigint,
           pg_catalog.round((pg_catalog.percentile_cont(0.5) within group (order by r.p50_ms))::numeric, 1),
           pg_catalog.round((pg_catalog.percentile_cont(0.9) within group (order by r.p90_ms))::numeric, 1),
           pg_catalog.max(r.max_ms)
      from observabilidad.resumen_llamadas_diario r
     where r.operacion = p_operacion
       and r.dia >= (pg_catalog.now() at time zone 'America/Costa_Rica')::date
                    - pg_catalog.least(pg_catalog.greatest(coalesce(p_dias, 90), 1), 3650)
     group by r.plataforma, r.version_app
     order by r.plataforma, pg_catalog.min(r.dia);
end;
$$;

revoke all on function observabilidad.panel_telemetria_versiones(text, int) from public;
grant execute on function observabilidad.panel_telemetria_versiones(text, int) to authenticated, service_role;

-- ---------------------------------------------------------------------------
-- Lo que expone la API: sin `security definer`.
create function public.panel_telemetria_flota()
returns table (
  dispositivo_id uuid, etiqueta text, plataforma text, version_app text, version_codigo int,
  sistema text, modelo text, sesion_iniciada_en timestamptz, ultimo_lote_en timestamptz,
  filas_24h bigint, descartadas_24h bigint
)
language sql
stable
security invoker
set search_path = ''
as $$ select * from observabilidad.panel_telemetria_flota(); $$;

create function public.panel_telemetria_rendimiento(p_dias int)
returns table (
  dispositivo_id uuid, etiqueta text, plataforma text, version_app text, operacion text,
  llamadas bigint, errores bigint, p50_ms numeric, p90_ms numeric, max_ms numeric, ultima_en timestamptz
)
language sql
stable
security invoker
set search_path = ''
as $$ select * from observabilidad.panel_telemetria_rendimiento(p_dias); $$;

create function public.panel_telemetria_versiones(p_operacion text, p_dias int)
returns table (
  plataforma text, version_app text, primer_dia date, ultimo_dia date, equipos bigint,
  llamadas bigint, errores bigint, p50_ms numeric, p90_ms numeric, max_ms numeric
)
language sql
stable
security invoker
set search_path = ''
as $$ select * from observabilidad.panel_telemetria_versiones(p_operacion, p_dias); $$;

revoke all on function public.panel_telemetria_flota() from public;
revoke all on function public.panel_telemetria_rendimiento(int) from public;
revoke all on function public.panel_telemetria_versiones(text, int) from public;
grant execute on function public.panel_telemetria_flota() to authenticated, service_role;
grant execute on function public.panel_telemetria_rendimiento(int) to authenticated, service_role;
grant execute on function public.panel_telemetria_versiones(text, int) to authenticated, service_role;
