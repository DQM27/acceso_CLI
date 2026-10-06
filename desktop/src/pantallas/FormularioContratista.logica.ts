import { z } from "zod";

export const TIPOS = ["Praind", "InHouse", "PorCorreo", "Swat"] as const;

/** Los que ofrece el selector: sin "PorCorreo", que dejó de ser un tipo de
 * contratista (2026-10-03; esas visitas se registran en la sección "Por
 * correo"). `TIPOS` sigue completo para leer un contratista viejo; si se
 * intentara guardar uno nuevo con ese tipo, el núcleo lo rechaza. */
export const TIPOS_ELEGIBLES = TIPOS.filter((tipo) => tipo !== "PorCorreo");

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
