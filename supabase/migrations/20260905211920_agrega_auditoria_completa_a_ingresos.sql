-- Extiende `ingresos` con los mismos campos de auditoría que ya tiene
-- `registro_ingresos` local (resultado de la decisión de acceso, motivo,
-- versión de reglas, y si la empresa estaba activa en ese momento) --
-- hasta ahora `ingresos` solo llevaba los campos de exhibición. Sin esto,
-- un movimiento visto desde otro dispositivo del mismo sitio nunca podía
-- mostrar "PRAIND vencido" ni el resto del detalle de auditoría, aunque
-- sea la misma operación. Nullable: las 42 filas existentes nunca
-- tuvieron este dato y no se puede reconstruir retroactivamente.
alter table ingresos
  add column resultado_acceso text,
  add column motivo_resultado text,
  add column reglas_version bigint,
  add column empresa_activa_snapshot boolean;

comment on column ingresos.resultado_acceso is 'Espejo de registro_ingresos.resultado_acceso (local) -- PERMITIDO/PERMITIDO_CON_ADVERTENCIA/MIGRADO. NULL en filas anteriores a esta migración.';
comment on column ingresos.motivo_resultado is 'Espejo de registro_ingresos.motivo_resultado (local) -- PRAIND_PROXIMO_VENCER/DATOS_RECONSTRUIDOS, o NULL si el resultado fue simplemente PERMITIDO.';
comment on column ingresos.reglas_version is 'Espejo de registro_ingresos.reglas_version (local).';
comment on column ingresos.empresa_activa_snapshot is 'Espejo de registro_ingresos.empresa_activa_snapshot (local) -- si la empresa del contratista estaba activa al momento de este ingreso.';
