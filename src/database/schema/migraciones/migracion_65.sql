-- Catálogo de gafetes más estricto (pedido del usuario, 2026-11-16).
--
-- Quedan dos estados: DISPONIBLE y PERDIDO (no asignable). Tres cambios:
--
-- 1. PERDIDO ya no exige un responsable. Un gafete puede faltar sin que se
--    sepa quién lo tenía, y un gafete ya pagado sigue PERDIDO (sin marca en
--    la persona) hasta que se reimprima: lo que importa es que nadie lo
--    asigne por error mientras no exista físicamente.
-- 2. DE_BAJA desaparece: era un estado sin regreso y un gafete siempre se
--    puede reimprimir. Los que estaban de baja pasan a PERDIDO, de donde se
--    recuperan como cualquier otro. Se encola su cambio para que la nube y
--    el otro equipo del sitio se enteren.
-- 3. Cada reporte del historial lleva un detalle en texto libre, y hay dos
--    tipos nuevos: PAGADO (se quita la marca a la persona, el gafete sigue
--    perdido) y RECUPERADO (apareció o se reimprimió, vuelve a DISPONIBLE).
--    RESUELTO queda sólo para las filas anteriores a esta migración.
--
-- `SQLite` no permite alterar un CHECK: se recrean las dos tablas (mismo
-- patrón que MIGRACION_39/41, con `foreign_keys = OFF`).
INSERT INTO cola_salida (entidad, entidad_uuid, operacion, creado_en, actualizado_en)
SELECT 'gafete', uuid, 'actualizar',
       strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
FROM gafetes
WHERE estado = 'DE_BAJA' AND uuid IS NOT NULL;

CREATE TABLE gafetes_nueva (
    id INTEGER PRIMARY KEY,
    numero INTEGER NOT NULL,
    tipo TEXT NOT NULL CHECK (tipo IN ('CONTRATISTA', 'VISITA', 'PROVEEDOR', 'PROVISIONAL_KOF')),
    estado TEXT NOT NULL CHECK (estado IN ('DISPONIBLE', 'PERDIDO')),
    contratista_portador_id INTEGER REFERENCES contratistas(id) ON DELETE RESTRICT,
    visita_portador_id INTEGER REFERENCES cita_visitantes(id) ON DELETE RESTRICT,
    encargado_portador_id INTEGER REFERENCES encargados_ruta(id) ON DELETE RESTRICT,
    proveedor_portador_id INTEGER REFERENCES registro_ingresos_proveedor(id) ON DELETE RESTRICT,
    uuid TEXT,
    CHECK (
        (tipo = 'CONTRATISTA'
            AND visita_portador_id IS NULL AND encargado_portador_id IS NULL
            AND proveedor_portador_id IS NULL)
        OR (tipo = 'VISITA'
            AND contratista_portador_id IS NULL AND encargado_portador_id IS NULL
            AND proveedor_portador_id IS NULL)
        OR (tipo = 'PROVEEDOR'
            AND contratista_portador_id IS NULL AND visita_portador_id IS NULL
            AND encargado_portador_id IS NULL)
        OR (tipo = 'PROVISIONAL_KOF'
            AND contratista_portador_id IS NULL AND visita_portador_id IS NULL
            AND proveedor_portador_id IS NULL)
    ),
    -- Sólo un gafete PERDIDO puede tener responsable, y puede no tenerlo.
    CHECK (
        estado = 'PERDIDO'
        OR (contratista_portador_id IS NULL AND visita_portador_id IS NULL
            AND encargado_portador_id IS NULL AND proveedor_portador_id IS NULL)
    )
) STRICT;
INSERT INTO gafetes_nueva (
    id, numero, tipo, estado, contratista_portador_id, visita_portador_id,
    encargado_portador_id, proveedor_portador_id, uuid
)
SELECT id, numero, tipo,
    CASE estado WHEN 'DE_BAJA' THEN 'PERDIDO' ELSE estado END,
    contratista_portador_id, visita_portador_id,
    encargado_portador_id, proveedor_portador_id, uuid
FROM gafetes;

CREATE TABLE gafetes_incidentes_nueva (
    id INTEGER PRIMARY KEY,
    gafete_id INTEGER NOT NULL REFERENCES gafetes(id) ON DELETE RESTRICT,
    tipo TEXT NOT NULL CHECK (tipo IN ('PERDIDO', 'PAGADO', 'RECUPERADO', 'RESUELTO')),
    fecha_hora TEXT NOT NULL,
    usuario_id INTEGER NOT NULL REFERENCES usuarios(id) ON DELETE RESTRICT,
    contratista_id INTEGER REFERENCES contratistas(id) ON DELETE RESTRICT,
    visita_portador_id INTEGER REFERENCES cita_visitantes(id) ON DELETE RESTRICT,
    encargado_portador_id INTEGER REFERENCES encargados_ruta(id) ON DELETE RESTRICT,
    proveedor_portador_id INTEGER REFERENCES registro_ingresos_proveedor(id) ON DELETE RESTRICT,
    motivo_resolucion TEXT CHECK (
        motivo_resolucion IS NULL OR motivo_resolucion IN ('PAGADO', 'APARECIDO')
    ),
    -- Lo que escribió quien hizo el reporte. Texto libre.
    detalle TEXT CHECK (detalle IS NULL OR length(detalle) BETWEEN 1 AND 1000),
    -- A lo sumo un responsable, y sólo en PERDIDO (a quién se le marca) o
    -- PAGADO (a quién se le quita la marca).
    CHECK (
        (contratista_id IS NOT NULL) + (visita_portador_id IS NOT NULL)
        + (encargado_portador_id IS NOT NULL) + (proveedor_portador_id IS NOT NULL) <= 1
    ),
    CHECK (
        tipo IN ('PERDIDO', 'PAGADO')
        OR (contratista_id IS NULL AND visita_portador_id IS NULL
            AND encargado_portador_id IS NULL AND proveedor_portador_id IS NULL)
    ),
    -- Un perdido sin responsable tiene que explicar qué pasó.
    CHECK (
        tipo <> 'PERDIDO'
        OR contratista_id IS NOT NULL OR visita_portador_id IS NOT NULL
        OR encargado_portador_id IS NOT NULL OR proveedor_portador_id IS NOT NULL
        OR detalle IS NOT NULL
    ),
    CHECK (
        tipo <> 'PAGADO'
        OR contratista_id IS NOT NULL OR visita_portador_id IS NOT NULL
        OR encargado_portador_id IS NOT NULL OR proveedor_portador_id IS NOT NULL
    ),
    CHECK (tipo <> 'RECUPERADO' OR detalle IS NOT NULL),
    CHECK ((tipo = 'RESUELTO') = (motivo_resolucion IS NOT NULL))
) STRICT;
INSERT INTO gafetes_incidentes_nueva (
    id, gafete_id, tipo, fecha_hora, usuario_id, contratista_id, visita_portador_id,
    encargado_portador_id, proveedor_portador_id, motivo_resolucion
)
SELECT id, gafete_id, tipo, fecha_hora, usuario_id, contratista_id, visita_portador_id,
    encargado_portador_id, proveedor_portador_id, motivo_resolucion
FROM gafetes_incidentes;

DROP TABLE gafetes_incidentes;
DROP TABLE gafetes;
ALTER TABLE gafetes_nueva RENAME TO gafetes;
ALTER TABLE gafetes_incidentes_nueva RENAME TO gafetes_incidentes;

CREATE UNIQUE INDEX idx_gafetes_numero_tipo ON gafetes(numero, tipo);
CREATE UNIQUE INDEX idx_gafetes_uuid ON gafetes(uuid);
CREATE INDEX idx_gafetes_estado ON gafetes(estado);
CREATE INDEX idx_gafetes_contratista_portador
ON gafetes(contratista_portador_id) WHERE contratista_portador_id IS NOT NULL;
CREATE INDEX idx_gafetes_visita_portador
ON gafetes(visita_portador_id) WHERE visita_portador_id IS NOT NULL;
CREATE INDEX idx_gafetes_encargado_portador
ON gafetes(encargado_portador_id) WHERE encargado_portador_id IS NOT NULL;
CREATE INDEX idx_gafetes_proveedor_portador
ON gafetes(proveedor_portador_id) WHERE proveedor_portador_id IS NOT NULL;
CREATE INDEX idx_gafetes_incidentes_gafete ON gafetes_incidentes(gafete_id, id DESC);
CREATE INDEX idx_gafetes_incidentes_fecha ON gafetes_incidentes(fecha_hora DESC, id DESC);
