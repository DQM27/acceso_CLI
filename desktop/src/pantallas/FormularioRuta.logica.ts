import { z } from "zod";

export const numeroValido = (valor: string) => {
  const n = Number(valor);
  return valor.trim() !== "" && Number.isInteger(n) && n > 0;
};

// Mismo criterio que FormularioGafete: validación de UI para evitar un
// error de tecleo evidente (vacío, no numérico, rango descomunal) -- la
// validación real y atómica queda en RutaCatalogoService (núcleo).
export const esquema = z
  .object({
    modo: z.enum(["individual", "rango"]),
    numero: z.string(),
    desde: z.string(),
    hasta: z.string(),
  })
  .superRefine((valores, ctx) => {
    if (valores.modo === "individual") {
      if (!numeroValido(valores.numero)) {
        ctx.addIssue({
          code: "custom",
          path: ["numero"],
          message: "Ingrese un número de ruta válido",
        });
      }
      return;
    }
    if (!numeroValido(valores.desde)) {
      ctx.addIssue({ code: "custom", path: ["desde"], message: 'Ingrese un "desde" válido' });
      return;
    }
    if (!numeroValido(valores.hasta) || Number(valores.hasta) < Number(valores.desde)) {
      ctx.addIssue({ code: "custom", path: ["hasta"], message: "El rango no es válido" });
      return;
    }
    if (Number(valores.hasta) - Number(valores.desde) > 200) {
      ctx.addIssue({
        code: "custom",
        path: ["hasta"],
        message: "El rango es demasiado grande (máximo 200 a la vez)",
      });
    }
  });
