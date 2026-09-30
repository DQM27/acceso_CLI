-- Resumen agregado de movimientos para la pantalla "Análisis" del panel.
--
-- El panel ve el historial de TODAS las unidades operativas, y con varias
-- unidades conectadas son millones de filas. Analizar eso en el navegador
-- (tablas dinámicas, gráficos) obligaría a descargarlas todas; en cambio, esta
-- función agrega en la base y devuelve pocos miles de filas como mucho, sin
-- importar cuánto crezca el historial.
--
-- Devuelve UN solo `jsonb` (no `setof`) a propósito: PostgREST corta cada
-- respuesta en `max_rows` (1.000, `supabase/config.toml`), y el resumen diario
-- de un período largo pasa de eso. Un solo valor no se corta.
--
-- Cuatro agregados sobre contratistas (`ingresos`) y proveedores
-- (`ingresos_proveedor`):
--   * `diario`:   día × unidad × tipo de persona × tipo de ingreso × medio.
--                 Alimenta la tabla dinámica y la tendencia por día.
--   * `por_hora`: día de la semana (1 = lunes) × hora del día. Horas pico.
--   * `empresas`: empresa × tipo de persona, las 100 con más ingresos.
--   * `total`:    totales del período, con personas distintas.
-- Día, hora y día de la semana son los de Costa Rica, como en el historial.
--
-- Medidas: `ingresos` (cantidad), `con_salida` (ya salieron) y
-- `minutos_adentro` (suma de permanencias de los que salieron): el promedio
-- de permanencia es `minutos_adentro / con_salida`, y así se puede sumar
-- entre filas sin sesgo. `personas` (distintas) sólo va en `empresas` y
-- `total`: sumar personas distintas de varios días contaría dos veces a quien
-- vino dos días, así que el diario no la trae.
--
-- `security invoker`: corre con los permisos de quien consulta, así que valen
-- las políticas RLS de `ingresos` e `ingresos_proveedor` (el panel ve todas
-- las unidades por `es_admin_global()`; un equipo, sólo la suya; nadie más,
-- nada). La condición de rango sobre `hora_entrada` usa operadores
-- `leakproof`, así que bajo RLS sí aprovecha `*_hora_entrada_idx` y
-- `*_sitio_hora_entrada_idx` (a diferencia del `like` del buscador, ver
-- `historial_busqueda_indexada`).
--
-- Rendimiento, medido en local (Postgres 16) con 1,2 millones de movimientos
-- sintéticos de 10 unidades en un año, consultando como administrador (bajo
-- RLS la consulta no se paraleliza): 6 meses (600.000 filas) ~2 s, un año
-- ~4,5 s, un mes de dos unidades ~0,1 s. Son dos recorridos del índice por
-- `hora_entrada` por tabla; el resto trabaja sobre resultados ya agregados.
-- El costo crece con las filas del período (~4 µs por fila), no con el
-- historial total.
--
-- Por eso el rango se limita a 366 días: el tiempo de `authenticated` en
-- Supabase se corta a los 8 s, y un año es el período más largo que tiene
-- sentido mirar de una vez (comparar dos años son dos consultas). Si el
-- volumen por año pasa de unos 2 millones, el siguiente paso es una tabla de
-- agregados por hora que se actualice al escribir, no subir este límite.
--
-- Proveedores: no tienen tipo de ingreso ('—'); el medio sale de la placa
-- (con placa = 'VEHÍCULO', sin placa = 'SIN DATO', porque la ausencia de placa
-- no prueba que haya entrado caminando).

create or replace function public.panel_resumen_movimientos(
  p_desde timestamptz,
  p_hasta timestamptz,
  p_sitio_ids uuid[] default null
)
returns jsonb
language plpgsql
stable
security invoker
set search_path = ''
-- Los agregados son con hash; con el `work_mem` por defecto se desbordaban a
-- disco con unos cientos de miles de filas.
set work_mem = '64MB'
as $$
declare
  v_resultado jsonb;
begin
  if p_desde is null or p_hasta is null or p_hasta <= p_desde then
    raise exception 'Rango de fechas inválido para el resumen de movimientos'
      using errcode = '22023';
  end if;
  if p_hasta - p_desde > interval '366 days' then
    raise exception 'El resumen admite hasta un año; acote el período'
      using errcode = '22023';
  end if;

  -- Primer recorrido: una fila por hora (UTC) × unidad × tipo × medio, con los
  -- valores crudos. Todo lo que es texto de pantalla y zona horaria se
  -- calcula después, sobre este resultado ya chico, no fila por fila.
  with por_bloque as materialized (
    select
      date_trunc('hour', i.hora_entrada, 'UTC') as bloque_utc,
      i.sitio_id,
      'CONTRATISTA'::text as tipo_persona,
      i.tipo_ingreso,
      i.medio_ingreso as medio,
      count(*) as ingresos,
      count(i.hora_salida) as con_salida,
      sum(extract(epoch from (i.hora_salida - i.hora_entrada))) as segundos_adentro
    from public.ingresos i
    where i.hora_entrada >= p_desde
      and i.hora_entrada < p_hasta
      and (p_sitio_ids is null or i.sitio_id = any (p_sitio_ids))
    group by 1, 2, 4, 5
    union all
    select
      date_trunc('hour', p.hora_entrada, 'UTC'),
      p.sitio_id,
      'PROVEEDOR'::text,
      null,
      case when nullif(btrim(p.placa), '') is not null then 'VEHICULO' end,
      count(*),
      count(p.hora_salida),
      sum(extract(epoch from (p.hora_salida - p.hora_entrada)))
    from public.ingresos_proveedor p
    where p.hora_entrada >= p_desde
      and p.hora_entrada < p_hasta
      and (p_sitio_ids is null or p.sitio_id = any (p_sitio_ids))
    group by 1, 2, 5
  ),
  -- Costa Rica está en UTC-6 fijo (sin horario de verano): un bloque de una
  -- hora UTC cae entero dentro de una hora local, así que agrupar en UTC y
  -- convertir después da el mismo día y la misma hora que convertir cada fila.
  por_bloque_local as (
    select
      (b.bloque_utc at time zone 'America/Costa_Rica') as bloque_local,
      b.sitio_id,
      b.tipo_persona,
      coalesce(replace(b.tipo_ingreso, '_', ' '), '—') as tipo_ingreso,
      case
        when b.medio = 'VEHICULO' then 'VEHÍCULO'
        when b.medio = 'CAMINANDO' then 'CAMINANDO'
        else 'SIN DATO'
      end as medio,
      b.ingresos,
      b.con_salida,
      b.segundos_adentro
    from por_bloque b
  ),
  diario as materialized (
    select
      b.bloque_local::date as dia,
      b.sitio_id,
      b.tipo_persona,
      b.tipo_ingreso,
      b.medio,
      sum(b.ingresos) as ingresos,
      sum(b.con_salida) as con_salida,
      coalesce(sum(b.segundos_adentro), 0) / 60 as minutos_adentro
    from por_bloque_local b
    group by 1, 2, 3, 4, 5
  ),
  por_hora as (
    select
      extract(isodow from b.bloque_local)::int as dia_semana,
      extract(hour from b.bloque_local)::int as hora,
      sum(b.ingresos) as ingresos
    from por_bloque_local b
    group by 1, 2
  ),
  -- Segundo recorrido: una fila por empresa y persona. De aquí salen los
  -- ingresos y las personas distintas de cada empresa, y las del total.
  por_persona as materialized (
    select
      coalesce(nullif(btrim(x.empresa_nombre), ''), 'SIN EMPRESA') as empresa,
      x.tipo_persona,
      x.identificacion,
      x.ingresos
    from (
      select i.empresa_nombre, 'CONTRATISTA'::text as tipo_persona,
             i.contratista_cedula as identificacion, count(*) as ingresos
      from public.ingresos i
      where i.hora_entrada >= p_desde
        and i.hora_entrada < p_hasta
        and (p_sitio_ids is null or i.sitio_id = any (p_sitio_ids))
      group by 1, 3
      union all
      select p.empresa_nombre, 'PROVEEDOR'::text, p.cedula, count(*)
      from public.ingresos_proveedor p
      where p.hora_entrada >= p_desde
        and p.hora_entrada < p_hasta
        and (p_sitio_ids is null or p.sitio_id = any (p_sitio_ids))
      group by 1, 3
    ) x
  ),
  empresas as (
    select
      pp.empresa,
      pp.tipo_persona,
      sum(pp.ingresos) as ingresos,
      count(distinct pp.identificacion) as personas
    from por_persona pp
    group by 1, 2
    order by ingresos desc, pp.empresa
    limit 100
  )
  select jsonb_build_object(
    'diario', coalesce((
      select jsonb_agg(jsonb_build_object(
               'dia', d.dia,
               'unidad', coalesce(s.nombre, 'SIN UNIDAD'),
               'tipo_persona', d.tipo_persona,
               'tipo_ingreso', d.tipo_ingreso,
               'medio', d.medio,
               'ingresos', d.ingresos,
               'con_salida', d.con_salida,
               'minutos_adentro', round(d.minutos_adentro)::bigint
             ) order by d.dia, s.nombre, d.tipo_persona, d.tipo_ingreso, d.medio)
        from diario d
        left join public.sitios s on s.id = d.sitio_id
    ), '[]'::jsonb),
    'por_hora', coalesce((
      select jsonb_agg(jsonb_build_object(
               'dia_semana', h.dia_semana,
               'hora', h.hora,
               'ingresos', h.ingresos
             ) order by h.dia_semana, h.hora)
        from por_hora h
    ), '[]'::jsonb),
    'empresas', coalesce((
      select jsonb_agg(jsonb_build_object(
               'empresa', e.empresa,
               'tipo_persona', e.tipo_persona,
               'ingresos', e.ingresos,
               'personas', e.personas
             ) order by e.ingresos desc, e.empresa)
        from empresas e
    ), '[]'::jsonb),
    'total', (
      select jsonb_build_object(
               'ingresos', coalesce(sum(d.ingresos), 0),
               'con_salida', coalesce(sum(d.con_salida), 0),
               'minutos_adentro', round(coalesce(sum(d.minutos_adentro), 0))::bigint,
               'personas', (select count(distinct pp.identificacion) from por_persona pp)
             )
        from diario d
    )
  )
  into v_resultado;

  return v_resultado;
end;
$$;

revoke all on function public.panel_resumen_movimientos(timestamptz, timestamptz, uuid[]) from public, anon;
grant execute on function public.panel_resumen_movimientos(timestamptz, timestamptz, uuid[]) to authenticated;
