-- Catálogo de números de ruta válidos (pedido explícito del usuario,
-- 2026-09-15): "sin restricción podrías poner la ruta 222 y no existe,
-- sino un número acotado de rutas". Por sitio (como gafetes) -- cada
-- sitio administra su propio catálogo -- pero con `numero` único EN TODA
-- LA TABLA (no por sitio, a diferencia de gafetes): el usuario fue
-- explícito en que dos sitios nunca comparten el mismo número de ruta
-- ("en cartago no saquen una ruta de brisas"). La reasignación entre
-- sitios (mencionada como caso raro, "no es lo normal") queda como una
-- operación administrativa manual (UPDATE de sitio_id), no expuesta
-- todavía en ninguna app -- las 3 políticas RLS son las mismas de
-- gafetes (select/insert/update, todas acotadas al propio sitio), así
-- que la reasignación necesita un camino con más privilegio (admin) que
-- esta migración no agrega porque no hay UI que lo use todavía.
CREATE TABLE public.rutas (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    sitio_id uuid NOT NULL REFERENCES public.sitios(id),
    dispositivo_origen_id uuid NOT NULL REFERENCES public.dispositivos(id),
    numero bigint NOT NULL UNIQUE CHECK (numero > 0),
    activo boolean NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

ALTER TABLE public.rutas ENABLE ROW LEVEL SECURITY;

CREATE POLICY "leer rutas del propio sitio" ON public.rutas FOR SELECT
    USING (sitio_id = ((SELECT auth.jwt()) ->> 'sitio_id')::uuid);
CREATE POLICY "crear rutas del propio sitio" ON public.rutas FOR INSERT
    WITH CHECK (sitio_id = ((SELECT auth.jwt()) ->> 'sitio_id')::uuid);
CREATE POLICY "actualizar rutas del propio sitio" ON public.rutas FOR UPDATE
    USING (sitio_id = ((SELECT auth.jwt()) ->> 'sitio_id')::uuid)
    WITH CHECK (sitio_id = ((SELECT auth.jwt()) ->> 'sitio_id')::uuid);

CREATE FUNCTION public.rutas_actualizar_updated_at()
RETURNS trigger
LANGUAGE plpgsql
AS $$
begin
  new.updated_at = now();
  return new;
end;
$$;

CREATE TRIGGER rutas_set_updated_at BEFORE UPDATE ON public.rutas
    FOR EACH ROW EXECUTE FUNCTION public.rutas_actualizar_updated_at();
CREATE TRIGGER rutas_emitir_cambio_nube AFTER INSERT OR DELETE OR UPDATE ON public.rutas
    FOR EACH ROW EXECUTE FUNCTION private.emitir_cambio_nube_sitio();

-- `salidas_ruta.numero_ruta` pasa de texto libre a entero (snapshot del
-- catálogo, mismo criterio que el lado local en MIGRACION_38) -- la tabla
-- sigue sin filas reales de producción (0 al momento de esta migración,
-- ver la migración anterior `control_de_rutas_...`), así que no hace
-- falta convertir datos.
ALTER TABLE public.salidas_ruta ALTER COLUMN numero_ruta TYPE bigint USING numero_ruta::bigint;
