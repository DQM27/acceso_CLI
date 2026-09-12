-- Cierra el hallazgo A-01 para `usuarios` (ver docs/arquitectura-supabase.md
-- 6.4 y docs/decisiones-tecnicas.md). Hasta hoy cualquier dispositivo con
-- sitio_id en el JWT podía crear/actualizar usuarios de su propio sitio
-- (incluido escalar el campo `rol` a ADMINISTRADOR) -- riesgo aceptado
-- mientras --tui-clasica y el desktop viejo todavía creaban usuarios
-- localmente y los empujaban por el outbox. Con el alta/edición de usuarios
-- migrada al panel web (admin-create-usuario/admin-reset-password-usuario)
-- y CLI/TUI clásica retiradas del crate raíz (2026-09-12, ver
-- docs/decisiones-tecnicas.md), ya no queda ninguna pantalla real que
-- dependa de que un dispositivo pueda escribir acá -- sincronizar el
-- catálogo sólo LEE usuarios, nunca escribe.
--
-- mobile/rust-core todavía expone AppCore::crear_usuario/actualizar_usuario
-- vía uniffi (dormido, sin pantalla de Kotlin que lo llame) -- si algo
-- llegara a invocarlo, el alta local seguiría funcionando pero el envío al
-- outbox fallaría con 403 al sincronizar (efecto esperado, no un bug).

drop policy "crear usuarios (propio sitio o admin_global)" on public.usuarios;
drop policy "actualizar usuarios (global)" on public.usuarios;

create policy "crear usuarios (solo admin_global)"
  on public.usuarios
  for insert
  to authenticated
  with check (private.es_admin_global());

create policy "actualizar usuarios (solo admin_global)"
  on public.usuarios
  for update
  to authenticated
  using (private.es_admin_global())
  with check (private.es_admin_global());
