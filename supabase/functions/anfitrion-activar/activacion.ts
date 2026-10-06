// Activación de una cuenta de anfitrión con su código, sin red ni base de
// datos (ver activacion.test.ts). La usa `index.ts`.

import { normalizarCodigo, normalizarCorreo, problemaDeContrasena } from "../_shared/anfitriones.ts";

export interface Puertos {
  /** La cuenta de Auth si el código es válido; null por cualquier motivo
   * (y el fallo cuenta como intento). */
  verificar(correo: string, codigo: string): Promise<string | null>;
  ponerContrasena(id: string, contrasena: string): Promise<"ok" | "debil">;
  consumir(correo: string): Promise<void>;
  cerrarSesiones(id: string): Promise<void>;
}

export type Respuesta =
  | { estado: 200; cuerpo: { ok: true } }
  | { estado: 400 | 422; cuerpo: { error: string; detail: string } };

/** El mismo texto para todo rechazo del código: no revela si el correo es
 * de un anfitrión, si el código venció o si se agotaron los intentos. */
export const CODIGO_INVALIDO = {
  error: "codigo_invalido",
  detail: "Correo o código incorrecto, vencido o ya usado. Si no funciona, pida uno nuevo a administración.",
};

export async function activar(cuerpo: unknown, puertos: Puertos): Promise<Respuesta> {
  const datos = typeof cuerpo === "object" && cuerpo !== null ? cuerpo as Record<string, unknown> : {};
  const correo = normalizarCorreo(datos.correo);
  const codigo = normalizarCodigo(datos.codigo);
  const contrasena = typeof datos.contrasena === "string" ? datos.contrasena : "";
  if (!correo || !codigo) return { estado: 400, cuerpo: CODIGO_INVALIDO };

  // La contraseña se revisa antes del código: así una contraseña corta no
  // le gasta un intento a la persona.
  const problema = problemaDeContrasena(contrasena, correo);
  if (problema) return { estado: 422, cuerpo: { error: "contrasena_invalida", detail: problema } };

  const id = await puertos.verificar(correo, codigo);
  if (!id) return { estado: 400, cuerpo: CODIGO_INVALIDO };

  // El código se consume recién con la contraseña guardada: si Supabase la
  // rechaza (filtrada, requisitos), la persona prueba otra con el mismo código.
  if ((await puertos.ponerContrasena(id, contrasena)) === "debil") {
    return {
      estado: 422,
      cuerpo: {
        error: "contrasena_debil",
        detail: "Esa contraseña aparece en filtraciones conocidas o no cumple los requisitos. Use otra frase.",
      },
    };
  }
  await puertos.consumir(correo);
  await puertos.cerrarSesiones(id);
  return { estado: 200, cuerpo: { ok: true } };
}
