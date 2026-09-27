
ALTER TABLE contratistas ADD COLUMN uuid TEXT;
UPDATE contratistas SET uuid = (
    lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(2))) || '-4'
    || substr(lower(hex(randomblob(2))), 2) || '-'
    || substr('89ab', abs(random()) % 4 + 1, 1) || substr(lower(hex(randomblob(2))), 2)
    || '-' || lower(hex(randomblob(6)))
) WHERE uuid IS NULL;
CREATE UNIQUE INDEX idx_contratistas_uuid ON contratistas(uuid);

ALTER TABLE registro_ingresos ADD COLUMN uuid TEXT;
UPDATE registro_ingresos SET uuid = (
    lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(2))) || '-4'
    || substr(lower(hex(randomblob(2))), 2) || '-'
    || substr('89ab', abs(random()) % 4 + 1, 1) || substr(lower(hex(randomblob(2))), 2)
    || '-' || lower(hex(randomblob(6)))
) WHERE uuid IS NULL;
CREATE UNIQUE INDEX idx_registro_ingresos_uuid ON registro_ingresos(uuid);
