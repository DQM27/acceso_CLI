
CREATE TRIGGER registro_ingresos_fecha_utc_insert
BEFORE INSERT ON registro_ingresos
WHEN
    strftime('%Y-%m-%dT%H:%M:%SZ', NEW.fecha_hora_ingreso) IS NOT NEW.fecha_hora_ingreso
    OR (
        NEW.fecha_hora_salida IS NOT NULL
        AND strftime('%Y-%m-%dT%H:%M:%SZ', NEW.fecha_hora_salida) IS NOT NEW.fecha_hora_salida
    )
BEGIN
    SELECT RAISE(ABORT, 'Las fechas de movimientos deben estar normalizadas en UTC');
END;

CREATE TRIGGER registro_ingresos_salida_utc
BEFORE UPDATE OF fecha_hora_salida ON registro_ingresos
WHEN
    NEW.fecha_hora_salida IS NOT NULL
    AND strftime('%Y-%m-%dT%H:%M:%SZ', NEW.fecha_hora_salida) IS NOT NEW.fecha_hora_salida
BEGIN
    SELECT RAISE(ABORT, 'La fecha de salida debe estar normalizada en UTC');
END;

CREATE TRIGGER registro_ingresos_entrada_inmutable
BEFORE UPDATE OF
    contratista_id, empresa_id, fecha_hora_ingreso, medio_ingreso, tipo_ingreso,
    gafete_numero, usuario_ingreso_id, contratista_cedula, contratista_nombre,
    empresa_nombre, usuario_ingreso_nombre, fecha_vencimiento_praind,
    es_personal_ruta, tiene_acceso, resultado_acceso, motivo_resultado,
    reglas_version
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
BEGIN
    SELECT RAISE(ABORT, 'Los datos historicos del ingreso son inmutables');
END;

CREATE TRIGGER registro_ingresos_salida_unica
BEFORE UPDATE OF fecha_hora_salida, usuario_salida_id, usuario_salida_nombre
ON registro_ingresos
WHEN
    OLD.fecha_hora_salida IS NOT NULL
    OR NEW.fecha_hora_salida IS NULL
    OR NEW.usuario_salida_id IS NULL
    OR NEW.usuario_salida_nombre IS NULL
BEGIN
    SELECT RAISE(ABORT, 'La salida solo puede registrarse una vez');
END;
