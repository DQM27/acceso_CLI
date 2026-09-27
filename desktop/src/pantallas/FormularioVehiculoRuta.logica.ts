import { z } from "zod";

export const esquema = z.object({
  placa: z.string().min(1, "La placa es obligatoria"),
  numero_unidad: z.string(),
  activo: z.boolean(),
});
