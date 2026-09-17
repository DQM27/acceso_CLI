-- Se me pasó agregar `search_path` fijo en la migración anterior --
-- mismo patrón que el resto de funciones `*_actualizar_updated_at`
-- (ej. vehiculos_ruta_actualizar_updated_at), que sí lo tienen.
ALTER FUNCTION public.rutas_actualizar_updated_at() SET search_path = public;
