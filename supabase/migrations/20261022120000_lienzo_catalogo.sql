-- Catálogo de consultas del lienzo de datos del panel (plan del asistente,
-- Parte A, etapa 1). Tres funciones de sólo lectura, con parámetros tipados y
-- sin SQL dinámico: lo que el lienzo puede pedir es exactamente lo que estas
-- firmas aceptan.
--
-- - `lienzo_contratistas`: filas de `panel_contratistas_estado` (las mismas
--   reglas de acceso y PRAIND que `verificar_acceso`) con los filtros del
--   catálogo. El lienzo agrupa y cuenta en el cliente.
-- - `lienzo_ingresos`: ingresos de contratistas por día (Costa Rica) en un rango
--   de hasta 92 días.
-- - `lienzo_resumen_hoy`: los números de la pantalla inicial (adentro ahora,
--   ingresos de hoy, PRAIND vencidos y por vencer, ingresos por día de las
--   últimas 2 semanas).
--
-- Sólo los administradores del panel: `security invoker` (corren con la RLS de
-- quien llama) más una comprobación explícita: cada administrador lee su propia
-- fila de `administradores_panel` (no se llama a `private.es_admin_global()`
-- porque `authenticated` no tiene USAGE sobre `private` fuera de las políticas).
-- La RLS también deja a un equipo ver su propia unidad, y esto es del panel.
-- Sin permisos para `anon`.

create or replace function public.lienzo_contratistas(
  p_empresas text[] default null,
  p_empresas_excluir text[] default null,
  p_contiene_empresa text default null,
  p_estados_praind text[] default null,
  p_tipos_ingreso text[] default null,
  p_sitio text default null,
  p_acceso_activo boolean default null,
  p_vencimiento_desde date default null,
  p_vencimiento_hasta date default null,
  p_contiene_nombre text default null,
  p_dias_desde integer default null,
  p_dias_hasta integer default null
)
returns table (
  nombre text,
  empresa text,
  estado_praind text,
  vencimiento date,
  dias_para_vencer integer,
  tipo_ingreso text,
  sitio text,
  acceso_activo boolean
)
language plpgsql
stable
security invoker
set search_path = ''
as $$
begin
  if not exists (select 1 from public.administradores_panel a where a.correo = (select auth.email())) then
    raise exception 'Sólo los administradores del panel pueden usar el lienzo'
      using errcode = '42501';
  end if;

  return query
  select c.nombre::text,
         c.empresa_nombre::text,
         c.estado_praind::text,
         c.fecha_vencimiento_praind,
         c.dias_para_vencer::integer,
         c.tipo_ingreso::text,
         c.adentro_sitio_nombre::text,
         c.activo
    from public.panel_contratistas_estado c
   where (p_empresas is null or c.empresa_nombre = any(p_empresas))
     and (p_empresas_excluir is null or c.empresa_nombre is null
          or c.empresa_nombre <> all(p_empresas_excluir))
     and (p_contiene_empresa is null
          or c.empresa_nombre ilike '%' || p_contiene_empresa || '%')
     and (p_estados_praind is null or c.estado_praind = any(p_estados_praind))
     and (p_tipos_ingreso is null or c.tipo_ingreso = any(p_tipos_ingreso))
     and (p_sitio is null or c.adentro_sitio_nombre = p_sitio)
     and (p_acceso_activo is null or c.activo = p_acceso_activo)
     and (p_vencimiento_desde is null or c.fecha_vencimiento_praind >= p_vencimiento_desde)
     and (p_vencimiento_hasta is null or c.fecha_vencimiento_praind <= p_vencimiento_hasta)
     and (p_contiene_nombre is null or c.nombre ilike '%' || p_contiene_nombre || '%')
     and (p_dias_desde is null or c.dias_para_vencer >= p_dias_desde)
     and (p_dias_hasta is null or c.dias_para_vencer <= p_dias_hasta)
   order by c.nombre
   limit 5000;
end;
$$;

create or replace function public.lienzo_ingresos(
  p_desde date,
  p_hasta date,
  p_empresas text[] default null,
  p_empresas_excluir text[] default null,
  p_contiene_empresa text default null,
  p_tipos_ingreso text[] default null,
  p_sitio text default null,
  p_contiene_nombre text default null
)
returns table (
  dia date,
  nombre text,
  empresa text,
  tipo_ingreso text,
  sitio text
)
language plpgsql
stable
security invoker
set search_path = ''
as $$
begin
  if not exists (select 1 from public.administradores_panel a where a.correo = (select auth.email())) then
    raise exception 'Sólo los administradores del panel pueden usar el lienzo'
      using errcode = '42501';
  end if;
  if p_desde is null or p_hasta is null or p_hasta < p_desde or p_hasta - p_desde > 92 then
    raise exception 'Rango de fechas inválido (máximo 92 días)' using errcode = '22023';
  end if;

  return query
  select (i.hora_entrada at time zone 'America/Costa_Rica')::date,
         i.contratista_nombre::text,
         i.empresa_nombre::text,
         i.tipo_ingreso::text,
         s.nombre::text
    from public.ingresos i
    left join public.sitios s on s.id = i.sitio_id
   where i.hora_entrada >= (p_desde::timestamp at time zone 'America/Costa_Rica')
     and i.hora_entrada < ((p_hasta + 1)::timestamp at time zone 'America/Costa_Rica')
     and (p_empresas is null or i.empresa_nombre = any(p_empresas))
     and (p_empresas_excluir is null or i.empresa_nombre is null
          or i.empresa_nombre <> all(p_empresas_excluir))
     and (p_contiene_empresa is null
          or i.empresa_nombre ilike '%' || p_contiene_empresa || '%')
     and (p_tipos_ingreso is null or i.tipo_ingreso = any(p_tipos_ingreso))
     and (p_sitio is null or s.nombre = p_sitio)
     and (p_contiene_nombre is null or i.contratista_nombre ilike '%' || p_contiene_nombre || '%')
   order by i.hora_entrada desc
   limit 20000;
end;
$$;

-- Resumen de hoy; `p_sitio_id` null = todos los sitios. Los 14 días van todos,
-- con 0 en los que no hubo ingresos, el último es hoy.
create or replace function public.lienzo_resumen_hoy(p_sitio_id uuid default null)
returns jsonb
language plpgsql
stable
security invoker
set search_path = ''
as $$
declare
  v_hoy date := (now() at time zone 'America/Costa_Rica')::date;
  v_resultado jsonb;
begin
  if not exists (select 1 from public.administradores_panel a where a.correo = (select auth.email())) then
    raise exception 'Sólo los administradores del panel pueden usar el lienzo'
      using errcode = '42501';
  end if;

  select pg_catalog.jsonb_build_object(
    'hoy', v_hoy,
    'adentro_contratistas', (select count(*) from public.panel_adentro_ahora a
                              where a.tipo = 'CONTRATISTA'
                                and (p_sitio_id is null or a.sitio_id = p_sitio_id)),
    'adentro_proveedores', (select count(*) from public.panel_adentro_ahora a
                             where a.tipo = 'PROVEEDOR'
                               and (p_sitio_id is null or a.sitio_id = p_sitio_id)),
    'ingresos_hoy', (select count(*) from public.ingresos i
                      where i.hora_entrada >= (v_hoy::timestamp at time zone 'America/Costa_Rica')
                        and (p_sitio_id is null or i.sitio_id = p_sitio_id)),
    -- Estado de acceso, igual que `verificar_acceso`: los contratistas que
    -- están activos y no pueden entrar por el PRAIND, y los que entran con aviso.
    'praind_vencidos', (select count(*) from public.panel_contratistas_estado c
                         where c.estado_acceso = 'PRAIND_VENCIDO'),
    'praind_por_vencer', (select count(*) from public.panel_contratistas_estado c
                           where c.estado_acceso = 'PERMITIDO_CON_ADVERTENCIA'),
    'ingresos_por_dia', (
      select pg_catalog.jsonb_agg(pg_catalog.jsonb_build_object('dia', d.dia, 'cantidad', coalesce(n.cantidad, 0))
                                  order by d.dia)
        from (select v_hoy - 13 + k as dia from pg_catalog.generate_series(0, 13) k) d
        left join (
          select (i.hora_entrada at time zone 'America/Costa_Rica')::date as dia, count(*) as cantidad
            from public.ingresos i
           where i.hora_entrada >= ((v_hoy - 13)::timestamp at time zone 'America/Costa_Rica')
             and (p_sitio_id is null or i.sitio_id = p_sitio_id)
           group by 1
        ) n on n.dia = d.dia
    )
  ) into v_resultado;

  return v_resultado;
end;
$$;

-- Permisos explícitos (AGENTS.md): nadie por `public`, nada a `anon`.
revoke all on function public.lienzo_contratistas(text[], text[], text, text[], text[], text, boolean, date, date, text, integer, integer) from public, anon;
revoke all on function public.lienzo_ingresos(date, date, text[], text[], text, text[], text, text) from public, anon;
revoke all on function public.lienzo_resumen_hoy(uuid) from public, anon;

grant execute on function public.lienzo_contratistas(text[], text[], text, text[], text[], text, boolean, date, date, text, integer, integer) to authenticated, service_role;
grant execute on function public.lienzo_ingresos(date, date, text[], text[], text, text[], text, text) to authenticated, service_role;
grant execute on function public.lienzo_resumen_hoy(uuid) to authenticated, service_role;
