import { z } from "zod";
import { requierePraind } from "../api";
import { cedulaSchema, nombreSchema } from "../validacion";

export const TIPOS = ["Praind", "InHouse", "PorCorreo", "Swat"] as const;

// La validación real vive en el core (services/contratista_service.rs) — este
// esquema es sólo para dar feedback inmediato sin ida y vuelta al backend.
export const esquema = z
  .object({
    cedula: cedulaSchema,
    nombre: nombreSchema,
    empresa_id: z.string().min(1, "Seleccioná una empresa"),
    tipo_ingreso: z.enum(TIPOS),
    fecha_vencimiento_praind: z.string(),
    es_personal_ruta: z.boolean(),
    tiene_acceso: z.boolean(),
  })
  .refine((datos) => !requierePraind(datos) || datos.fecha_vencimiento_praind !== "", {
    message: "Obligatoria para este tipo de contratista",
    path: ["fecha_vencimiento_praind"],
  });
