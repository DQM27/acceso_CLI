-- Tabla de telemetría del build `diagnostico` de la app Android
-- (mobile/android/app/.../Telemetria.kt, ver
-- mobile/android/docs/telemetria-diagnostico.md).
--
-- SÓLO para el proyecto de STAGING (sandbox). A propósito NO está en
-- supabase/migrations/: producción nunca recibe telemetría y la tabla no
-- debe existir ahí. Se aplica a mano (SQL Editor o
-- `supabase db execute -f este-archivo`) contra staging. Es idempotente.
--
-- Contenido: sólo métricas técnicas (tiempos, memoria, CPU, batería, red,
-- frames, violaciones de StrictMode, objetos retenidos). La app NUNCA manda
-- texto leído por el OCR, cédulas, nombres ni imágenes.
--
-- Acceso: la app inserta con la llave publicable (rol `anon`) y NO puede
-- leer ni modificar filas. El análisis se hace desde el SQL Editor (rol
-- `postgres`, que salta RLS). Sin límites de tamaño ni limpieza automática:
-- son datos de desarrollo y se borran a mano cuando dejen de servir
-- (`truncate public.telemetria_diagnostico;`).
begin;

create table if not exists public.telemetria_diagnostico (
    id           bigint generated always as identity primary key,
    recibido_en  timestamptz not null default now(),
    -- Reloj del teléfono: puede diferir del servidor; `recibido_en` es la
    -- referencia confiable del orden de llegada.
    ocurrido_en  timestamptz not null,
    -- UUID aleatorio por instalación (no el ANDROID_ID ni el IMEI).
    dispositivo  text not null,
    -- UUID aleatorio por arranque del proceso.
    sesion       text not null,
    version_app  text not null,
    tipo         text not null,
    datos        jsonb not null default '{}'::jsonb
);

comment on table public.telemetria_diagnostico is
    'Telemetría técnica del build "diagnostico" de la app Android (sólo staging). Sin datos personales. Ver mobile/android/docs/telemetria-diagnostico.md.';

create index if not exists telemetria_diagnostico_tipo_ocurrido_idx
    on public.telemetria_diagnostico (tipo, ocurrido_en);
create index if not exists telemetria_diagnostico_sesion_idx
    on public.telemetria_diagnostico (sesion);

alter table public.telemetria_diagnostico enable row level security;

-- Sólo insertar. Ni `anon` ni `authenticated` pueden leer, actualizar ni
-- borrar: una llave publicable filtrada no expone lo ya recolectado.
revoke all on public.telemetria_diagnostico from anon, authenticated;
grant insert on public.telemetria_diagnostico to anon, authenticated;

drop policy if exists telemetria_diagnostico_insertar on public.telemetria_diagnostico;
create policy telemetria_diagnostico_insertar
    on public.telemetria_diagnostico
    for insert
    to anon, authenticated
    with check (true);

commit;

-- Consultas útiles para el análisis (correr desde el SQL Editor):
--
-- Resumen por tipo de evento:
--   select tipo, count(*), min(ocurrido_en), max(ocurrido_en)
--   from public.telemetria_diagnostico group by tipo order by 2 desc;
--
-- Arranques en frío (ms hasta el primer frame):
--   select ocurrido_en, datos->>'tipo_arranque', (datos->>'ms_desde_inicio_proceso')::numeric
--   from public.telemetria_diagnostico where tipo = 'arranque' order by ocurrido_en desc;
--
-- Evolución de la memoria (PSS) por sesión, para ver si crece sin bajar:
--   select sesion, ocurrido_en, (datos->>'pss_total_mb')::numeric, datos->>'pantalla'
--   from public.telemetria_diagnostico where tipo = 'muestra_sistema' order by sesion, ocurrido_en;
--
-- Objetos retenidos (posibles fugas):
--   select ocurrido_en, datos from public.telemetria_diagnostico where tipo = 'retencion';
--
-- Llamadas al núcleo más lentas:
--   select datos->>'nombre', max((datos->>'max_ms')::numeric), sum((datos->>'llamadas')::int)
--   from public.telemetria_diagnostico
--   where tipo = 'llamada_nucleo' group by 1 order by 2 desc;
--
-- Sesiones de cámara OCR (fps, tiempo hasta confirmar):
--   select ocurrido_en, datos->>'pantalla', datos->>'fps', datos->>'ms_hasta_confirmar'
--   from public.telemetria_diagnostico where tipo = 'ocr_sesion' order by ocurrido_en desc;
