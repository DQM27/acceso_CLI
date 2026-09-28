
CREATE INDEX idx_registro_ingresos_fecha_salida
ON registro_ingresos(fecha_hora_salida)
WHERE fecha_hora_salida IS NOT NULL;
