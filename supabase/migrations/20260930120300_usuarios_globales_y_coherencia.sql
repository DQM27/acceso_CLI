-- Cierre de la optimización del 2026-09-30.
--
-- 1. Usuarios sin unidad. Igual que contratistas y empresas
--    (`catalogo_global_sin_unidad`): un operador es global, entra en
--    cualquier unidad y darlo de baja vale en todas. Su aviso en vivo salía
--    sólo por el canal de la unidad donde se creó, así que una baja tardaba
--    en verse en las demás hasta su siguiente sincronización. Ahora el aviso
--    llega a todas las unidades y la columna `sitio_id` se borra: el panel
--    (`admin-create-usuario`) y las apps (`cola.rs`) dejan de mandarla en la
--    misma entrega, y ninguna app la leía (`catalogo.rs`).
--
-- 2. Coherencia de la salida en `ingresos`: la misma regla que ya tenía
--    `ingresos_proveedor` -- abierto sin datos de salida, o cerrado con quién
--    lo cerró. A propósito NO se exige `hora_salida >= hora_entrada`: la
--    entrada y la salida pueden venir de equipos con relojes desfasados, y
--    esa regla rechazaría salidas legítimas al sincronizar. En producción y
--    en staging no hay filas que la incumplan.
--
-- 3. `plegar_texto` con `search_path` fijo (aviso de seguridad del linter de
--    Supabase). Su cuerpo ya usa nombres completos (`extensions.unaccent`),
--    así que el resultado no cambia y los índices que la usan siguen válidos.

-- 1. Usuarios sin unidad -----------------------------------------------------

alter table public.usuarios drop column sitio_id;

-- `private.emitir_cambio_nube_global` viene de `catalogo_global_sin_unidad`.
drop trigger if exists usuarios_emitir_cambio_nube on public.usuarios;
create trigger usuarios_emitir_cambio_nube
  after insert or update or delete on public.usuarios
  for each row execute function private.emitir_cambio_nube_global();

-- 2. Coherencia de la salida en ingresos ---------------------------------------

alter table public.ingresos
  add constraint ingresos_salida_coherente check (
    (hora_salida is null and dispositivo_salida_id is null and usuario_salida_nombre is null)
    or (hora_salida is not null and usuario_salida_nombre is not null)
  );

-- 3. plegar_texto ------------------------------------------------------------

alter function public.plegar_texto(text) set search_path = '';
