import { useState } from "react";
import { supabase } from "../lib/supabase";

/**
 * Step-up authentication por correo antes de una acción sensible (agregar
 * o quitar un administrador) -- reusa el OTP nativo de Supabase
 * (`signInWithOtp` + `verifyOtp`) en vez de armar envío de correo a mano.
 * Manda un código de 6 dígitos, no un link: la plantilla "Magic Link" del
 * proyecto usa `{{ .Token }}` en vez de `{{ .ConfirmationURL }}` (Auth >
 * Emails en el dashboard de Supabase) -- ese cambio de plantilla es lo que
 * decide link vs. código, no algo que se controle desde acá.
 *
 * `verifyOtp` sí crea/renueva una sesión real (a diferencia de sólo mandar
 * el correo) -- pero es la sesión de la MISMA persona que ya está logueada
 * (confirma su propio correo), así que no hay cambio de identidad, sólo
 * queda su sesión refrescada.
 */
/** Supabase manda sus mensajes de error en inglés -- acá se traducen los
 * que de verdad puede ver alguien usando el panel (el límite de reenvío es
 * el más común, si se piden dos códigos seguidos). Cualquier otro queda con
 * un mensaje genérico en vez del inglés crudo. */
function mensajeErrorEnEspanol(error: unknown): string {
  const mensaje = error instanceof Error ? error.message : String(error);
  const limite = mensaje.match(/only request this after (\d+) seconds?/i);
  if (limite) {
    return `Por seguridad, esperá ${limite[1]} segundos antes de pedir otro código.`;
  }
  return "No se pudo enviar el código -- intentá de nuevo en un momento.";
}

export function useVerificacionPorCorreo(correo: string) {
  const [enviado, setEnviado] = useState(false);
  const [enviando, setEnviando] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function pedirConfirmacion() {
    setEnviando(true);
    setError(null);
    try {
      const { error } = await supabase.auth.signInWithOtp({
        email: correo,
        options: { shouldCreateUser: false },
      });
      if (error) throw error;
      setEnviado(true);
    } catch (error) {
      setError(mensajeErrorEnEspanol(error));
    } finally {
      setEnviando(false);
    }
  }

  async function confirmarCodigo(codigo: string) {
    setError(null);
    const { error } = await supabase.auth.verifyOtp({
      email: correo,
      token: codigo.trim(),
      type: "email",
    });
    if (error) {
      const mensaje = "Código inválido o vencido -- pedí uno nuevo.";
      setError(mensaje);
      throw new Error(mensaje);
    }
  }

  function reiniciar() {
    setEnviado(false);
    setError(null);
  }

  return { enviado, enviando, error, pedirConfirmacion, confirmarCodigo, reiniciar };
}
