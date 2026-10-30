-- Historial de contratistas del panel: rápido con cualquier volumen.
--
-- Medido en un banco local con 52 millones de ingresos (PostgreSQL 17, los
-- mismos índices y la misma RLS que producción, 6 meses de período):
--
--   primera página                   6,5 s   ->  0,007 s
--   página 1.000 / tramo de exportar 13-17 s ->  0,007-0,02 s (cursor)
--   buscar un apellido poco común    82 s    ->  ~0,35 s
--   buscar dos palabras sin resultado 3-6 s  ->  ~0,35 s
--   filtrar empresa (valor que no está) 5,7 s ->  milisegundos
--
-- Tres causas, tres arreglos:
--
-- 1. El panel ordenaba por `hora_entrada desc nulls last, id` y el índice era
--    `hora_entrada desc` (nulls first): Postgres ordenaba el período entero
--    para dar 100 filas. Y saltar a una página lejana leía y descartaba todas
--    las anteriores. Índices nuevos `(hora_entrada desc, id desc)` -- con y
--    sin unidad adelante -- que sirven al orden por defecto y a la
--    paginación por cursor ("lo que sigue después de esta fila"). Reemplazan
--    a los de sólo `hora_entrada`: mismo prefijo, un índice menos que
--    mantener en cada registro.
--
-- 2. Los filtros por columna iban a la vista `panel_movimientos`, que lee con
--    RLS (`security_invoker`). Bajo RLS, Postgres no puede usar un índice
--    para `like` (no es `leakproof`) y recorría todo el período. Ahora el
--    Historial de contratistas se pide con UNA función,
--    `panel_historial_contratistas`, que valida que quien consulta sea
--    administrador del panel y arma la consulta completa (período, unidades,
--    búsqueda, filtros, orden y página) sin RLS, con los índices. Los
--    filtros llegan como datos (columna, operación, valor) y sólo se aceptan
--    columnas y operaciones de una lista cerrada; los valores van siempre
--    escapados (`format %L`).
--
-- 3. La búsqueda por nombre juntaba todas las coincidencias del período antes
--    de ordenar. Ahora cuenta primero cuántas PERSONAS del catálogo coinciden
--    (`contratistas.nombre_plegado`, índice de trigramas): con pocas (hasta
--    1.000) trae los ingresos de esas personas por `(contratista_id,
--    hora_entrada)`; con muchas, filtra los ingresos por sus trigramas, que
--    encuentran las primeras 100 enseguida justamente porque abundan. El
--    resultado es el mismo: ingresos cuyo nombre registrado tiene todas las
--    palabras.
--
-- `panel_buscar_movimientos` y la vista `panel_movimientos` quedan como están
-- (un panel viejo abierto durante el despliegue sigue andando).

-- 1. Orden por defecto y cursor.
create index ingresos_hora_id_idx on public.ingresos (hora_entrada desc, id desc);
create index ingresos_sitio_hora_id_idx on public.ingresos (sitio_id, hora_entrada desc, id desc);
drop index public.ingresos_hora_entrada_idx;
drop index public.ingresos_sitio_hora_entrada_idx;

-- Los ingresos de una persona, del más reciente al más viejo. También cubre
-- la clave foránea (`contratista_id` adelante), así que reemplaza al índice
-- simple.
create index ingresos_contratista_hora_idx on public.ingresos (contratista_id, hora_entrada desc);
drop index public.ingresos_contratista_id_idx;

-- Filtros de texto con valores poco comunes (empresa y operadores): la misma
-- expresión que las columnas `*_p` de la vista, así el índice se usa al
-- filtrar por ellas.
create index ingresos_empresa_plegado_trgm_idx on public.ingresos
  using gin (public.plegar_texto(coalesce(empresa_nombre, '')) extensions.gin_trgm_ops);
create index ingresos_usuario_entrada_plegado_trgm_idx on public.ingresos
  using gin (public.plegar_texto(coalesce(usuario_entrada_nombre, '')) extensions.gin_trgm_ops);
create index ingresos_usuario_salida_plegado_trgm_idx on public.ingresos
  using gin (public.plegar_texto(coalesce(usuario_salida_nombre, '')) extensions.gin_trgm_ops);

-- 3. Nombre del catálogo sin tildes ni mayúsculas, para buscar personas.
-- Columna generada: las apps no la mandan ni la leen (piden sus columnas una
-- por una).
alter table public.contratistas
  add column nombre_plegado text generated always as (public.plegar_texto(coalesce(nombre, ''))) stored;
create index contratistas_nombre_plegado_trgm_idx on public.contratistas
  using gin (nombre_plegado extensions.gin_trgm_ops);

-- La vista suma `contratista_id` al final (agregar columnas al final no
-- cambia las que ya existen): la búsqueda por persona filtra por él. Resto,
-- tal como estaba (`20260930100300_panel_movimientos_filtros`).
create or replace view public.panel_movimientos
with (security_invoker = true) as
select
  i.id,
  i.sitio_id,
  s.nombre as sitio_nombre,
  i.contratista_cedula,
  i.contratista_nombre,
  i.empresa_nombre,
  i.tipo_ingreso,
  i.medio_ingreso,
  i.gafete_numero,
  i.hora_entrada,
  i.hora_salida,
  i.usuario_entrada_nombre,
  i.usuario_salida_nombre,
  d.tipo as dispositivo_entrada_tipo,
  i.placa,
  coalesce(replace(i.tipo_ingreso, '_', ' '), '—') as tipo_texto,
  case
    when i.medio_ingreso = 'VEHICULO' and nullif(btrim(i.placa), '') is not null then btrim(i.placa)
    when i.medio_ingreso = 'VEHICULO' then 'VEHÍCULO'
    when i.medio_ingreso = 'CAMINANDO' then 'CAMINANDO'
    else '—'
  end as medio_texto,
  public.plegar_texto(coalesce(s.nombre, '')) as unidad_p,
  public.plegar_texto(coalesce(i.contratista_cedula, '')) as cedula_p,
  i.contratista_nombre_plegado as nombre_p,
  public.plegar_texto(coalesce(i.empresa_nombre, '')) as empresa_p,
  public.plegar_texto(coalesce(replace(i.tipo_ingreso, '_', ' '), '—')) as tipo_p,
  public.plegar_texto(
    case
      when i.medio_ingreso = 'VEHICULO' and nullif(btrim(i.placa), '') is not null then btrim(i.placa)
      when i.medio_ingreso = 'VEHICULO' then 'VEHÍCULO'
      when i.medio_ingreso = 'CAMINANDO' then 'CAMINANDO'
      else '—'
    end) as medio_p,
  public.plegar_texto(coalesce(i.usuario_entrada_nombre, '')) as usuario_entrada_p,
  public.plegar_texto(coalesce(i.usuario_salida_nombre, '')) as usuario_salida_p,
  i.contratista_id
from public.ingresos i
left join public.sitios s on s.id = i.sitio_id
left join public.dispositivos d on d.id = i.dispositivo_entrada_id;

-- Texto literal para `like`: `\`, `%` y `_` no actúan como comodines.
create function private.escapar_like(texto text)
returns text
language sql
immutable
parallel safe
set search_path to ''
as $$
  select replace(replace(replace(texto, '\', '\\'), '%', '\%'), '_', '\_');
$$;

-- Una condición de filtro de la grilla, como SQL sobre `m` (la vista
-- `panel_movimientos`). `p_columna` ya viene validada contra la lista cerrada
-- de `panel_historial_contratistas`; los valores se escapan acá.
--
-- Texto (columnas `*_p`, ya sin tildes ni mayúsculas; el valor se pliega
-- igual): contiene (todas las palabras, en cualquier orden), no_contiene,
-- igual, distinto, empieza, termina, vacio, no_vacio.
-- Número: igual, distinto, mayor, mayor_igual, menor, menor_igual, entre
-- (extremos excluidos, como AG Grid), vacio, no_vacio.
-- Fecha (instantes ya calculados por el panel con el día de Costa Rica):
-- rango (`desde` incluido, `hasta` excluido, cualquiera opcional), fuera
-- (antes de `desde` o desde `hasta`), vacio, no_vacio.
create function private.panel_condicion_filtro(p_columna text, p_tipo text, p_condicion jsonb)
returns text
language plpgsql
immutable
set search_path to ''
as $$
declare
  v_op text := p_condicion ->> 'op';
  v_col text := format('m.%I', p_columna);
  v_texto text;
  v_palabras text[];
  v_numero numeric;
  v_numero2 numeric;
  v_desde timestamptz;
  v_hasta timestamptz;
begin
  if v_op = 'vacio' then
    return case when p_tipo = 'texto' then format('(%s is null or %s = '''')', v_col, v_col)
                else format('%s is null', v_col) end;
  elsif v_op = 'no_vacio' then
    return case when p_tipo = 'texto' then format('(%s is not null and %s <> '''')', v_col, v_col)
                else format('%s is not null', v_col) end;
  end if;

  if p_tipo = 'texto' then
    v_texto := public.plegar_texto(btrim(coalesce(p_condicion ->> 'valor', '')));
    if v_texto = '' then
      return null;
    end if;
    case v_op
      when 'contiene' then
        select array_agg(format('%s like %L', v_col, '%' || private.escapar_like(palabra) || '%'))
          into v_palabras
          from regexp_split_to_table(v_texto, '\s+') palabra
         where palabra <> '';
        return '(' || array_to_string(v_palabras, ' and ') || ')';
      when 'no_contiene' then
        return format('(%s is null or %s not like %L)', v_col, v_col, '%' || private.escapar_like(v_texto) || '%');
      when 'igual' then
        return format('%s = %L', v_col, v_texto);
      when 'distinto' then
        return format('(%s is null or %s <> %L)', v_col, v_col, v_texto);
      when 'empieza' then
        return format('%s like %L', v_col, private.escapar_like(v_texto) || '%');
      when 'termina' then
        return format('%s like %L', v_col, '%' || private.escapar_like(v_texto));
      else null;
    end case;
  elsif p_tipo = 'numero' then
    v_numero := (p_condicion ->> 'valor')::numeric;
    v_numero2 := (p_condicion ->> 'valor2')::numeric;
    if v_numero is null then
      return null;
    end if;
    case v_op
      when 'igual' then return format('%s = %L::numeric', v_col, v_numero);
      when 'distinto' then return format('(%s is null or %s <> %L::numeric)', v_col, v_col, v_numero);
      when 'mayor' then return format('%s > %L::numeric', v_col, v_numero);
      when 'mayor_igual' then return format('%s >= %L::numeric', v_col, v_numero);
      when 'menor' then return format('%s < %L::numeric', v_col, v_numero);
      when 'menor_igual' then return format('%s <= %L::numeric', v_col, v_numero);
      when 'entre' then
        if v_numero2 is null then
          return null;
        end if;
        return format('(%s > %L::numeric and %s < %L::numeric)', v_col, v_numero, v_col, v_numero2);
      else null;
    end case;
  elsif p_tipo = 'fecha' then
    v_desde := (p_condicion ->> 'desde')::timestamptz;
    v_hasta := (p_condicion ->> 'hasta')::timestamptz;
    case v_op
      when 'rango' then
        if v_desde is null and v_hasta is null then
          return null;
        end if;
        return '(' || concat_ws(' and ',
          case when v_desde is not null then format('%s >= %L::timestamptz', v_col, v_desde) end,
          case when v_hasta is not null then format('%s < %L::timestamptz', v_col, v_hasta) end) || ')';
      when 'fuera' then
        if v_desde is null or v_hasta is null then
          return null;
        end if;
        return format('(%s < %L::timestamptz or %s >= %L::timestamptz)', v_col, v_desde, v_col, v_hasta);
      else null;
    end case;
  end if;

  raise exception 'Operación de filtro no permitida: % (%)', v_op, p_tipo using errcode = '22023';
end;
$$;

-- El Historial de contratistas del panel, una página por llamada.
--
-- p_desde / p_hasta    período por hora de entrada (`hasta` excluido).
-- p_sitio_ids          unidades; `null` = todas.
-- p_busqueda           cédula (por inicio, sin guiones ni ceros a la
--                      izquierda) o nombre (todas las palabras).
-- p_filtros            filtros de la grilla:
--                      [{"columna": "empresa_p", "union": "and"|"or",
--                        "condiciones": [{"op": "contiene", "valor": "..."}]}]
-- p_orden / p_descendente  columna de la lista cerrada; desempate por id.
-- p_limite             1 a 1.000 filas.
-- p_desplazamiento     filas a saltar (páginas sin cursor).
-- p_cursor_hora / p_cursor_id  sólo con el orden por hora de entrada: la
--                      página que sigue a esa fila, sin leer las anteriores.
create function public.panel_historial_contratistas(
  p_desde timestamptz default null,
  p_hasta timestamptz default null,
  p_sitio_ids uuid[] default null,
  p_busqueda text default null,
  p_filtros jsonb default '[]'::jsonb,
  p_orden text default 'hora_entrada',
  p_descendente boolean default true,
  p_limite integer default 100,
  p_desplazamiento integer default 0,
  p_cursor_hora timestamptz default null,
  p_cursor_id uuid default null
)
returns setof public.panel_movimientos
language plpgsql
stable
security definer
set search_path to ''
as $$
declare
  v_condiciones text[] := array['true'];
  v_texto text := btrim(coalesce(p_busqueda, ''));
  v_cedula text;
  v_patrones text[];
  v_personas integer;
  v_ids uuid[];
  v_filtro jsonb;
  v_condicion jsonb;
  v_columna text;
  v_tipo text;
  v_partes text[];
  v_parte text;
  v_direccion text := case when coalesce(p_descendente, true) then 'desc' else 'asc' end;
  v_orden text;
begin
  if not private.es_admin_global() then
    raise exception 'Sólo un administrador del panel puede ver el historial'
      using errcode = '42501';
  end if;
  if p_limite is null or p_limite < 1 or p_limite > 1000 then
    raise exception 'p_limite debe estar entre 1 y 1000' using errcode = '22023';
  end if;
  if coalesce(p_desplazamiento, 0) < 0 then
    raise exception 'p_desplazamiento no puede ser negativo' using errcode = '22023';
  end if;
  if jsonb_typeof(coalesce(p_filtros, '[]'::jsonb)) <> 'array' then
    raise exception 'p_filtros debe ser una lista' using errcode = '22023';
  end if;

  if p_desde is not null then
    v_condiciones := v_condiciones || format('m.hora_entrada >= %L::timestamptz', p_desde);
  end if;
  if p_hasta is not null then
    v_condiciones := v_condiciones || format('m.hora_entrada < %L::timestamptz', p_hasta);
  end if;
  if p_sitio_ids is not null then
    v_condiciones := v_condiciones || format('m.sitio_id = any(%L::uuid[])', p_sitio_ids);
  end if;

  -- Búsqueda.
  if v_texto <> '' then
    v_cedula := public.normalizar_cedula(v_texto);
    if v_cedula is not null and v_cedula ~ '[0-9]' then
      v_cedula := regexp_replace(v_cedula, '^0+', '');
      if v_cedula = '' then
        return;
      end if;
      v_condiciones := v_condiciones
        || format('m.contratista_cedula like %L', private.escapar_like(v_cedula) || '%');
    else
      select array_agg('%' || private.escapar_like(palabra) || '%')
        into v_patrones
        from regexp_split_to_table(public.plegar_texto(v_texto), '\s+') palabra
       where palabra <> '';
      if v_patrones is null then
        return;
      end if;
      select count(*) into v_personas
        from (select 1 from public.contratistas c
               where c.nombre_plegado like all (v_patrones) limit 1001) x;
      if v_personas = 0 then
        return;
      end if;
      if v_personas <= 1000 then
        select array_agg(c.id) into v_ids
          from public.contratistas c
         where c.nombre_plegado like all (v_patrones);
        v_condiciones := v_condiciones || format('m.contratista_id = any(%L::uuid[])', v_ids);
      end if;
      -- Siempre también contra el nombre registrado en el ingreso: con
      -- pocas personas sólo afina (mismo resultado que sin catálogo); con
      -- muchas es la búsqueda, por trigramas.
      v_condiciones := v_condiciones || format('m.nombre_p like all (%L::text[])', v_patrones);
    end if;
  end if;

  -- Filtros de la grilla.
  for v_filtro in select value from jsonb_array_elements(coalesce(p_filtros, '[]'::jsonb)) loop
    v_columna := v_filtro ->> 'columna';
    v_tipo := case
      when v_columna in ('unidad_p', 'cedula_p', 'nombre_p', 'empresa_p', 'dispositivo_entrada_tipo',
                         'tipo_p', 'medio_p', 'usuario_entrada_p', 'usuario_salida_p') then 'texto'
      when v_columna = 'gafete_numero' then 'numero'
      when v_columna in ('hora_entrada', 'hora_salida') then 'fecha'
    end;
    if v_tipo is null then
      raise exception 'Columna de filtro no permitida: %', v_columna using errcode = '22023';
    end if;
    v_partes := array[]::text[];
    for v_condicion in select value from jsonb_array_elements(coalesce(v_filtro -> 'condiciones', '[]'::jsonb)) loop
      v_parte := private.panel_condicion_filtro(v_columna, v_tipo, v_condicion);
      if v_parte is not null then
        v_partes := v_partes || v_parte;
      end if;
    end loop;
    if cardinality(v_partes) > 0 then
      v_condiciones := v_condiciones || ('(' || array_to_string(
        v_partes, case when v_filtro ->> 'union' = 'or' then ' or ' else ' and ' end) || ')');
    end if;
  end loop;

  -- Orden y página.
  v_orden := coalesce(p_orden, 'hora_entrada');
  if v_orden not in ('sitio_nombre', 'contratista_cedula', 'contratista_nombre', 'empresa_nombre',
                     'dispositivo_entrada_tipo', 'tipo_texto', 'medio_texto', 'gafete_numero',
                     'hora_entrada', 'hora_salida', 'usuario_entrada_nombre', 'usuario_salida_nombre') then
    raise exception 'Orden no permitido: %', v_orden using errcode = '22023';
  end if;
  if v_orden = 'hora_entrada' then
    if p_cursor_hora is not null and p_cursor_id is not null then
      v_condiciones := v_condiciones || format(
        '(m.hora_entrada, m.id) %s (%L::timestamptz, %L::uuid)',
        case when v_direccion = 'desc' then '<' else '>' end, p_cursor_hora, p_cursor_id);
    end if;
    v_orden := format('m.hora_entrada %s, m.id %s', v_direccion, v_direccion);
  else
    v_orden := format('m.%I %s nulls last, m.id', v_orden, v_direccion);
  end if;

  return query execute
    'select m.* from public.panel_movimientos m where ' || array_to_string(v_condiciones, ' and ')
    || ' order by ' || v_orden
    || format(' limit %s offset %s', p_limite, coalesce(p_desplazamiento, 0));
end;
$$;

grant execute on function public.panel_historial_contratistas(
  timestamptz, timestamptz, uuid[], text, jsonb, text, boolean, integer, integer, timestamptz, uuid
) to authenticated, service_role;
