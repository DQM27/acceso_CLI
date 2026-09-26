import { z } from "zod";

export const esquemaCambioPassword = z
  .object({
    passwordActual: z.string().min(1, "La contraseña actual es obligatoria"),
    passwordNueva: z.string().min(8, "Al menos 8 caracteres"),
    confirmar: z.string().min(1, "Confirma la contraseña"),
  })
  .refine((valores) => valores.passwordNueva === valores.confirmar, {
    message: "Las contraseñas no coinciden",
    path: ["confirmar"],
  })
  .refine((valores) => valores.passwordNueva !== valores.passwordActual, {
    message: "La nueva tiene que ser distinta de la actual",
    path: ["passwordNueva"],
  });
