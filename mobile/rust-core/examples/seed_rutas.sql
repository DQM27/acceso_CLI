-- Catálogo de prueba para el mecanismo de rutas (encargados, vehículos,
-- rutas) -- sin esto los 3 buscadores de `PantallaRutas` (todos BLOQUEANTES
-- desde 2026-09-19) no tienen contra qué buscar en una base recién sembrada.
INSERT OR IGNORE INTO encargados_ruta (codigo_empleado, nombre, cedula, activo, uuid) VALUES
    ('E001', 'Carlos Mendez', '101110111', 1, 'a0000000-0000-4000-8000-000000000001'),
    ('E002', 'Ana Solano', '102220222', 1, 'a0000000-0000-4000-8000-000000000002'),
    ('E003', 'Luis Rojas', '103330333', 1, 'a0000000-0000-4000-8000-000000000003'),
    ('E004', 'Maria Vargas', '104440444', 1, 'a0000000-0000-4000-8000-000000000004'),
    ('E005', 'Jose Castro', '105550555', 0, 'a0000000-0000-4000-8000-000000000005');

INSERT OR IGNORE INTO vehiculos_ruta (numero_unidad, placa, activo, uuid) VALUES
    ('U-10', 'C12345', 1, 'b0000000-0000-4000-8000-000000000001'),
    ('U-11', 'C99999', 1, 'b0000000-0000-4000-8000-000000000002'),
    ('U-12', 'CL54321', 1, 'b0000000-0000-4000-8000-000000000003'),
    (NULL, 'BBB123', 1, 'b0000000-0000-4000-8000-000000000004'),
    ('U-13', 'C00000', 0, 'b0000000-0000-4000-8000-000000000005');

INSERT OR IGNORE INTO rutas (numero, activo, uuid) VALUES
    (1, 1, 'c0000000-0000-4000-8000-000000000001'),
    (2, 1, 'c0000000-0000-4000-8000-000000000002'),
    (79, 1, 'c0000000-0000-4000-8000-000000000003'),
    (150, 1, 'c0000000-0000-4000-8000-000000000004'),
    (222, 0, 'c0000000-0000-4000-8000-000000000005');
