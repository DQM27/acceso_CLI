import { z } from "zod";

// Sin restricción de caracteres más allá de "no vacío" — a diferencia del
// nombre de un contratista, el de una empresa puede tener números o símbolos
// (S.A., 3M, etc.), no tiene sentido restringirlo.
export const esquema = z.object({
  nombre: z.string().min(1, "El nombre es obligatorio"),
});
