import { z } from "zod";

/** Todos los valores posibles, para leer cualquier contratista (también uno
 * viejo "PorCorreo"). Cuáles se pueden ELEGIR lo decide el núcleo
 * (`tiposIngresoSeleccionables`); si se intentara guardar uno nuevo con un
 * tipo retirado, el núcleo lo rechaza. */
export const TIPOS = ["Praind", "InHouse", "PorCorreo", "Swat"] as const;

// Sin reglas de negocio acá: cédula, nombre, PRAIND y personal de ruta los
// valida el núcleo (`ContratistaService`) y su mensaje se muestra tal cual.
// Sólo se pide la empresa, porque sin ella no hay `empresa_id` que mandar.
export const esquema = z.object({
  cedula: z.string(),
  nombre: z.string(),
  empresa_id: z.string().min(1, "Elija la empresa"),
  tipo_ingreso: z.enum(TIPOS),
  fecha_vencimiento_praind: z.string(),
  es_personal_ruta: z.boolean(),
  tiene_acceso: z.boolean(),
});
