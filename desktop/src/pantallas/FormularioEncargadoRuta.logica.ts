import { z } from "zod";

export const esquema = z.object({
  codigo_empleado: z
    .string()
    .regex(/^\d{5,7}$/, "El código de empleado debe tener entre 5 y 7 dígitos"),
  nombre: z.string().min(1, "El nombre es obligatorio"),
  activo: z.boolean(),
});
