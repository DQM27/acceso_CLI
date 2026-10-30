-- Incidente 2026-10-30: un teléfono con una versión vieja (anterior al
-- arreglo de `sitio_id` en el alta, 2026-09-30) no pudo dar de alta empresas
-- de proveedor (403). Agotaron sus 20 intentos y desde ahí todo ingreso de
-- esas empresas falló por la clave foránea (409,
-- `ingresos_proveedor_empresa_id_fkey`), sin que nada volviera a mandarlas.
--
-- 1. Las empresas de proveedor que quedaron con error vuelven a la cola:
--    con el alta nueva (`crear_empresa_proveedor`) suben, o adoptan la de la
--    nube si ese nombre ya existe.
-- 2. Los ingresos de proveedor que fallaron por la empresa faltante se
--    descartan de la cola (decisión del dueño: son de días atrás y no hace
--    falta subirlos). El registro local no se toca. Los que vengan de ahora
--    en más los repara la cola sola (`reparar_empresa_del_ingreso_proveedor`).
UPDATE cola_salida
SET estado = 'pendiente',
    intentos = 0,
    motivo_fallo = NULL,
    actualizado_en = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
WHERE entidad = 'empresa_proveedor'
  AND estado = 'fallido'
  AND COALESCE(motivo_fallo, 'error') = 'error';

DELETE FROM cola_salida
WHERE entidad = 'ingreso_proveedor'
  AND estado = 'fallido'
  AND ultimo_error LIKE '%ingresos_proveedor_empresa_id_fkey%';
