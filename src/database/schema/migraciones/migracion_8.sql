
ALTER TABLE registro_ingresos
ADD COLUMN empresa_activa_snapshot INTEGER NOT NULL DEFAULT 1
CHECK (empresa_activa_snapshot IN (0, 1));

DROP TRIGGER registro_ingresos_entrada_inmutable;
CREATE TRIGGER registro_ingresos_entrada_inmutable
BEFORE UPDATE OF
    contratista_id, empresa_id, fecha_hora_ingreso, medio_ingreso, tipo_ingreso,
    gafete_numero, usuario_ingreso_id, contratista_cedula, contratista_nombre,
    empresa_nombre, usuario_ingreso_nombre, fecha_vencimiento_praind,
    es_personal_ruta, tiene_acceso, resultado_acceso, motivo_resultado,
    reglas_version, empresa_activa_snapshot
ON registro_ingresos
WHEN
    NEW.contratista_id IS NOT OLD.contratista_id
    OR NEW.empresa_id IS NOT OLD.empresa_id
    OR NEW.fecha_hora_ingreso IS NOT OLD.fecha_hora_ingreso
    OR NEW.medio_ingreso IS NOT OLD.medio_ingreso
    OR NEW.tipo_ingreso IS NOT OLD.tipo_ingreso
    OR NEW.gafete_numero IS NOT OLD.gafete_numero
    OR NEW.usuario_ingreso_id IS NOT OLD.usuario_ingreso_id
    OR NEW.contratista_cedula IS NOT OLD.contratista_cedula
    OR NEW.contratista_nombre IS NOT OLD.contratista_nombre
    OR NEW.empresa_nombre IS NOT OLD.empresa_nombre
    OR NEW.usuario_ingreso_nombre IS NOT OLD.usuario_ingreso_nombre
    OR NEW.fecha_vencimiento_praind IS NOT OLD.fecha_vencimiento_praind
    OR NEW.es_personal_ruta IS NOT OLD.es_personal_ruta
    OR NEW.tiene_acceso IS NOT OLD.tiene_acceso
    OR NEW.resultado_acceso IS NOT OLD.resultado_acceso
    OR NEW.motivo_resultado IS NOT OLD.motivo_resultado
    OR NEW.reglas_version IS NOT OLD.reglas_version
    OR NEW.empresa_activa_snapshot IS NOT OLD.empresa_activa_snapshot
BEGIN
    SELECT RAISE(ABORT, 'Los datos historicos del ingreso son inmutables');
END;
