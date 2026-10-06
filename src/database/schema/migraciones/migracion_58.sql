-- Visitas agendadas abiertas por el OTRO equipo de la unidad (pedido del
-- dueño 2026-10-06): el teléfono y la PC muestran quién está adentro en toda
-- la unidad y pueden darle salida, igual que con contratistas y "por
-- correo". La llena la sincronización (`recibir_movimientos_visita_abiertos`).
CREATE TABLE movimientos_visita_remotos (
    uuid TEXT PRIMARY KEY,
    sitio_id TEXT NOT NULL,
    cedula TEXT NOT NULL,
    nombre TEXT NOT NULL,
    empresa TEXT,
    anfitrion_nombre TEXT,
    motivo TEXT,
    gafete_numero INTEGER,
    placa TEXT,
    hora_entrada TEXT NOT NULL,
    usuario_entrada_nombre TEXT,
    dispositivo_entrada_id TEXT NOT NULL,
    actualizado_en TEXT NOT NULL
) STRICT;

-- La lápida de un cierre hecho acá también vale para esta caché. SQLite no
-- cambia un CHECK en su lugar: se rehace la tabla con lo que ya tenía.
CREATE TABLE remotos_cerrados_aca_nueva (
    uuid TEXT PRIMARY KEY,
    tabla TEXT NOT NULL CHECK (
        tabla IN (
            'ingresos_remotos', 'ingresos_proveedor_remotos', 'ingresos_correo_remotos',
            'prestamos_gafete_provisional_remotos', 'movimientos_visita_remotos'
        )
    ),
    cerrado_en TEXT NOT NULL
) STRICT;
INSERT INTO remotos_cerrados_aca_nueva (uuid, tabla, cerrado_en)
SELECT uuid, tabla, cerrado_en FROM remotos_cerrados_aca;
DROP TABLE remotos_cerrados_aca;
ALTER TABLE remotos_cerrados_aca_nueva RENAME TO remotos_cerrados_aca;
