-- Resumen del lienzo: "Ingresos hoy" se comparaba con el día COMPLETO de la
-- semana pasada. A media mañana eso daba -80 % o -100 % aunque el día viniera
-- normal: se comparaba un día a medias contra uno terminado.
--
-- Ahora el resumen trae también los ingresos del mismo día de la semana pasada
-- hasta esta misma hora (`ingresos_semana_pasada_a_esta_hora`), que es contra
-- lo que tiene sentido comparar. El resto de la función no cambia.

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
    -- El mismo día de la semana pasada, desde su inicio hasta esta misma hora.
    'ingresos_semana_pasada_a_esta_hora', (select count(*) from public.ingresos i
                      where i.hora_entrada >= ((v_hoy - 7)::timestamp at time zone 'America/Costa_Rica')
                        and i.hora_entrada < now() - interval '7 days'
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

-- Permisos explícitos (AGENTS.md), los mismos de antes: nadie por `public`, nada a `anon`.
revoke all on function public.lienzo_resumen_hoy(uuid) from public, anon;
grant execute on function public.lienzo_resumen_hoy(uuid) to authenticated, service_role;
