-- Constraint real contra que dos dispositivos del mismo sitio le asignen
-- el mismo número de gafete a dos ingresos activos a la vez -- hoy sólo lo
-- evita el índice único LOCAL de cada dispositivo
-- (`idx_registro_ingresos_gafete_activo`, `src/database/schema.rs`), que
-- nunca ve lo que hizo el otro dispositivo hasta sincronizar. Mismo
-- criterio de "excluir con WHERE" que ya usa `ingresos_activos_idx`
-- (`20260902153640_esquema_inicial_receptor.sql`) para "hora_salida is
-- null" -- acá se suma "gafete_numero is not null" porque un ingreso a
-- pie no lleva gafete y no debe competir por este cupo.
--
-- Rompe el upsert (`Prefer: resolution=merge-duplicates`, que sólo
-- resuelve conflicto por `id`) el día que dos dispositivos efectivamente
-- choquen: el segundo POST recibe 409 con `code: "23505"` en vez de
-- aceptarse en silencio -- ver el manejo de ese caso puntual, por nombre
-- de este índice, en `src/nube/sincronizacion.rs::procesar_fila_individual`
-- (crate Rust, PR #62).
--
-- Aplicada y probada contra control-acceso-staging (pmrytjktlyiuikxuuxpr)
-- antes de este commit: choque real de gafete rechazado con el mensaje
-- esperado, y los dos casos que NO deben chocar (sin gafete; mismo gafete
-- reusado tras cerrar el primero) confirmados sin falsos positivos.
create unique index ingresos_gafete_activo_sitio_idx
  on public.ingresos (sitio_id, gafete_numero)
  where hora_salida is null and gafete_numero is not null;
