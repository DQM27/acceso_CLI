-- Reconexión de un equipo retirado (ver
-- docs/features-futuras/plan-reconexion-equipo-retirado.md):
--   sitio_id         unidad a la que pertenece este equipo (la del token),
--                    para saber si una reconexión lo cambia de unidad.
--   equipo_retirado  la nube dijo que no reconoce la clave del equipo (lo
--                    retiraron en el panel): habilita "Reconectar este
--                    equipo". Se borra con el próximo token válido.
--   descartar_base   el equipo se reconectó en OTRA unidad: esta base es de
--                    la unidad anterior. Mientras esté en 1 no se sincroniza,
--                    y al arrancar la app la borra y empieza con una nueva.
ALTER TABLE sincronizacion_estado ADD COLUMN sitio_id TEXT;
ALTER TABLE sincronizacion_estado ADD COLUMN equipo_retirado INTEGER NOT NULL DEFAULT 0
    CHECK (equipo_retirado IN (0, 1));
ALTER TABLE sincronizacion_estado ADD COLUMN descartar_base INTEGER NOT NULL DEFAULT 0
    CHECK (descartar_base IN (0, 1));
