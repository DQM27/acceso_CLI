import { z } from "zod";

export const esquemaLogin = z.object({
  cedula: z.string().min(1, "La cédula es obligatoria"),
  password: z.string().min(1, "La contraseña es obligatoria"),
});

export const esquemaCambioObligatorio = z
  .object({
    passwordNueva: z.string().min(8, "Al menos 8 caracteres"),
    confirmar: z.string().min(1, "Confirmá la contraseña"),
  })
  .refine((valores) => valores.passwordNueva === valores.confirmar, {
    message: "Las contraseñas no coinciden",
    path: ["confirmar"],
  });
