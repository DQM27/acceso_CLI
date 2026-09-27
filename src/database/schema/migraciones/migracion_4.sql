
CREATE TRIGGER contratistas_cedula_inmutable
BEFORE UPDATE OF cedula ON contratistas
WHEN NEW.cedula <> OLD.cedula
BEGIN
    SELECT RAISE(ABORT, 'La cedula del contratista es inmutable');
END;
