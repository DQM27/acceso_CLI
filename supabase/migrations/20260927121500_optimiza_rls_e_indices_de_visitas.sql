-- Ajustes de rendimiento sobre 20260927120000_rediseno_visitas_tablas_nuevas
-- (Performance Advisor de Supabase, corrido justo después de aplicarla en
-- staging), mismo criterio que
-- 20260912015156_optimiza_rls_de_visitas_e_indices_faltantes.sql aplicó en
-- su momento al esquema viejo de citas:
--
-- 1. Índices faltantes en FKs (dispositivo_entrada_id/dispositivo_salida_id/
--    usuario_entrada_id/usuario_salida_id de visita_movimientos,
--    sitio_id de aceptaciones y de anfitrion_sitios).
-- 2. `auth.email()` sin envolver en `(select ...)` se re-evalúa por FILA en
--    vez de una vez por consulta (`auth.jwt()` ya estaba bien envuelto en
--    la migración anterior, `auth.email()` no).

create index aceptaciones_sitio_idx on public.aceptaciones(sitio_id);
create index anfitrion_sitios_sitio_idx on public.anfitrion_sitios(sitio_id);
create index visita_movimientos_dispositivo_entrada_idx on public.visita_movimientos(dispositivo_entrada_id);
create index visita_movimientos_dispositivo_salida_idx on public.visita_movimientos(dispositivo_salida_id);
create index visita_movimientos_usuario_entrada_idx on public.visita_movimientos(usuario_entrada_id);
create index visita_movimientos_usuario_salida_idx on public.visita_movimientos(usuario_salida_id);

alter policy "leer visitantes propios, del sitio, o admin"
  on public.visitantes
  using (
    private.es_admin_global()
    or exists (
      select 1
      from public.visita_invitados vi
      join public.visitas v on v.id = vi.visita_id
      where vi.visitante_id = visitantes.id
        and (
          v.sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
          or v.anfitrion_id = (select a.id from public.anfitriones a where a.correo = (select auth.email()))
        )
    )
  );

alter policy "anfitrion activo crea visitantes"
  on public.visitantes
  with check (
    exists (select 1 from public.anfitriones a where a.correo = (select auth.email()) and a.activo)
  );

alter policy "anfitrion o admin corrige visitantes que invito"
  on public.visitantes
  using (
    private.es_admin_global()
    or exists (
      select 1
      from public.visita_invitados vi
      join public.visitas v on v.id = vi.visita_id
      where vi.visitante_id = visitantes.id
        and v.anfitrion_id = (select a.id from public.anfitriones a where a.correo = (select auth.email()))
    )
  )
  with check (
    private.es_admin_global()
    or exists (
      select 1
      from public.visita_invitados vi
      join public.visitas v on v.id = vi.visita_id
      where vi.visitante_id = visitantes.id
        and v.anfitrion_id = (select a.id from public.anfitriones a where a.correo = (select auth.email()))
    )
  );

alter policy "leer anfitrion_sitios propios o admin"
  on public.anfitrion_sitios
  using (
    private.es_admin_global()
    or anfitrion_id = (select a.id from public.anfitriones a where a.correo = (select auth.email()))
  );

alter policy "leer requisitos_sitio (anfitrion con acceso, dispositivo, o admin)"
  on public.requisitos_sitio
  using (
    private.es_admin_global()
    or sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
    or exists (
      select 1 from public.anfitrion_sitios ans
      join public.anfitriones a on a.id = ans.anfitrion_id
      where a.correo = (select auth.email()) and ans.sitio_id = requisitos_sitio.sitio_id
    )
  );

alter policy "leer visitas propias, del sitio, o admin"
  on public.visitas
  using (
    private.es_admin_global()
    or sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
    or anfitrion_id = (select a.id from public.anfitriones a where a.correo = (select auth.email()))
  );

alter policy "anfitrion crea sus propias visitas"
  on public.visitas
  with check (anfitrion_id = (select a.id from public.anfitriones a where a.correo = (select auth.email())));

alter policy "anfitrion actualiza sus propias visitas"
  on public.visitas
  using (anfitrion_id = (select a.id from public.anfitriones a where a.correo = (select auth.email())))
  with check (anfitrion_id = (select a.id from public.anfitriones a where a.correo = (select auth.email())));

alter policy "leer visita_invitados de visitas propias, del sitio, o admin"
  on public.visita_invitados
  using (
    private.es_admin_global()
    or exists (
      select 1 from public.visitas v
      where v.id = visita_invitados.visita_id
        and (
          v.sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
          or v.anfitrion_id = (select a.id from public.anfitriones a where a.correo = (select auth.email()))
        )
    )
  );

alter policy "anfitrion o dispositivo agrega invitados"
  on public.visita_invitados
  with check (
    exists (
      select 1 from public.visitas v
      where v.id = visita_invitados.visita_id
        and (
          v.anfitrion_id = (select a.id from public.anfitriones a where a.correo = (select auth.email()))
          or (
            v.sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
            and ((select auth.jwt()) ->> 'tipo') <> 'visor'
          )
        )
    )
  );

alter policy "anfitrion o dispositivo actualiza invitados"
  on public.visita_invitados
  using (
    exists (
      select 1 from public.visitas v
      where v.id = visita_invitados.visita_id
        and (
          v.anfitrion_id = (select a.id from public.anfitriones a where a.correo = (select auth.email()))
          or v.sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
        )
    )
  )
  with check (
    exists (
      select 1 from public.visitas v
      where v.id = visita_invitados.visita_id
        and (
          v.anfitrion_id = (select a.id from public.anfitriones a where a.correo = (select auth.email()))
          or (
            v.sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
            and ((select auth.jwt()) ->> 'tipo') <> 'visor'
          )
        )
    )
  );

alter policy "leer visita_movimientos del propio sitio, del anfitrion, o admin"
  on public.visita_movimientos
  using (
    private.es_admin_global()
    or sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
    or exists (
      select 1
      from public.visita_invitados vi
      join public.visitas v on v.id = vi.visita_id
      where vi.id = visita_movimientos.invitado_id
        and v.anfitrion_id = (select a.id from public.anfitriones a where a.correo = (select auth.email()))
    )
  );
