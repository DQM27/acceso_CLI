CREATE TABLE remotos_cerrados_aca (
    uuid TEXT PRIMARY KEY,
    tabla TEXT NOT NULL CHECK (
        tabla IN (
            'ingresos_remotos', 'ingresos_proveedor_remotos', 'ingresos_correo_remotos',
            'prestamos_gafete_provisional_remotos'
        )
    ),
    cerrado_en TEXT NOT NULL
) STRICT;
