-- Medio de ingreso de las visitas agendadas (pedido del dueño 2026-10-05):
-- la placa si entró en vehículo, NULL si entró caminando, igual que
-- `registro_ingresos_correo.placa`. También en la caché del historial de la
-- unidad, para mostrarla junto a los movimientos de las otras PCs.
ALTER TABLE movimientos_visita ADD COLUMN placa TEXT;
ALTER TABLE historial_visitas_sitio ADD COLUMN placa TEXT;
