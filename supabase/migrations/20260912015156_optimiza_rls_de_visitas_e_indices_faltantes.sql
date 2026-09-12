-- Limpieza de hallazgos del Performance Advisor (2026-09-12), todos de bajo riesgo:
-- ninguno cambia QUIÉN puede leer/escribir qué -- son reescrituras mecánicas de las
-- mismas condiciones, más 3 índices que faltaban. No toca ninguna tabla publicada a
-- Realtime (ingresos/contratistas/empresas/usuarios/dispositivos/administradores_panel)
-- ni sus políticas -- esas ya estaban optimizadas desde envuelve_auth_jwt_en_select_para_rls.
--
-- 1) auth_rls_initplan: las políticas de citas/cita_sitios/cita_visitantes/sitios/
--    anfitriones llamaban `auth.email()` directo (sin envolver en `select`), lo que
--    Postgres reevalúa fila por fila en vez de una sola vez por consulta -- mismo
--    patrón que ya se corrigió para auth.jwt() en estas mismas políticas
--    (mueve_helpers_de_recursion_citas_a_esquema_privado) pero se dejó pasar auth.email().

alter policy "anfitrion actualiza sus propias citas" on public.citas
using (anfitrion_correo = (select auth.email()))
with check (anfitrion_correo = (select auth.email()));

alter policy "anfitrion crea sus propias citas" on public.citas
with check (anfitrion_correo = (select auth.email()));

alter policy "leer citas propias, del sitio, o admin" on public.citas
using (
  anfitrion_correo = (select auth.email())
  or public.es_admin_global()
  or ((select auth.jwt()) ->> 'sitio_id')::uuid in (select private.sitios_de_cita(citas.id))
);

alter policy "leer cita_sitios propios, del sitio, o admin" on public.cita_sitios
using (
  sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
  or public.es_admin_global()
  or private.anfitrion_de_cita(cita_sitios.cita_id) = (select auth.email())
);

alter policy "anfitrion agrega sitios a sus propias citas" on public.cita_sitios
with check (private.anfitrion_de_cita(cita_sitios.cita_id) = (select auth.email()));

alter policy "anfitrion quita sitios de sus propias citas" on public.cita_sitios
using (private.anfitrion_de_cita(cita_sitios.cita_id) = (select auth.email()));

alter policy "leer visitantes de citas propias, del sitio, o admin" on public.cita_visitantes
using (
  public.es_admin_global()
  or exists (
    select 1 from public.citas c
    where c.id = cita_visitantes.cita_id and c.anfitrion_correo = (select auth.email())
  )
  or exists (
    select 1 from public.cita_sitios cs
    where cs.cita_id = cita_visitantes.cita_id
      and cs.sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
  )
);

alter policy "anfitrion agrega visitantes a sus propias citas" on public.cita_visitantes
with check (
  exists (
    select 1 from public.citas c
    where c.id = cita_visitantes.cita_id and c.anfitrion_correo = (select auth.email())
  )
);

alter policy "anfitrion edita visitantes de sus propias citas" on public.cita_visitantes
using (
  exists (
    select 1 from public.citas c
    where c.id = cita_visitantes.cita_id and c.anfitrion_correo = (select auth.email())
  )
)
with check (
  exists (
    select 1 from public.citas c
    where c.id = cita_visitantes.cita_id and c.anfitrion_correo = (select auth.email())
  )
);

alter policy "anfitrion quita visitantes de sus propias citas" on public.cita_visitantes
using (
  exists (
    select 1 from public.citas c
    where c.id = cita_visitantes.cita_id and c.anfitrion_correo = (select auth.email())
  )
);

alter policy "anfitrion lee sitios" on public.sitios
using (exists (select 1 from public.anfitriones a where a.correo = (select auth.email())));

alter policy "cada anfitrion lee su propia fila" on public.anfitriones
using ((select auth.email()) = correo and activo);

-- 2) unindexed_foreign_keys: 3 FKs sin índice de cobertura, mismo criterio que
--    indexa_foreign_keys_faltantes (2026-09-05) para el resto del esquema.
create index if not exists idx_cita_sitios_sitio on public.cita_sitios(sitio_id);
create index if not exists idx_movimientos_visita_dispositivo_entrada on public.movimientos_visita(dispositivo_entrada_id);
create index if not exists idx_movimientos_visita_dispositivo_salida on public.movimientos_visita(dispositivo_salida_id);

-- 3) duplicate_index: gafetes_sitio_id_numero_key (índice del unique constraint real,
--    se conserva) y gafetes_sitio_numero_idx (índice manual idéntico, redundante) --
--    se borra el segundo, la garantía de unicidad la sigue dando el constraint.
drop index if exists public.gafetes_sitio_numero_idx;
