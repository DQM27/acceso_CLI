
CREATE TABLE sincronizacion_estado (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    catalogo_actualizado_hasta TEXT
) STRICT;
INSERT INTO sincronizacion_estado (id, catalogo_actualizado_hasta) VALUES (1, NULL);
