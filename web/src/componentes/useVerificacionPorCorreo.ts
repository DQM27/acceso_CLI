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
export function useVerificacionPorCorreo(correo: string) {
  const [enviado, setEnviado] = useState(false);
  const [enviando, setEnviando] = useState(false);
  const [error, setError] = useState<string | null>(null);

  /** Devuelve el mensaje de error (o `null` si salió bien) además de dejarlo
   * en `error` -- el valor de retorno es para quien necesita reaccionar en
   * el mismo tick (ver `BotonProbarOtp` en `App.tsx`); leer `error` del
   * estado justo después de este `await` no sirve ahí porque el closure de
   * un callback async queda con el valor de cuando se creó, no el que
   * `setError` acaba de fijar. */
  async function pedirConfirmacion(): Promise<string | null> {
    setEnviando(true);
    setError(null);
    try {
      const { error } = await supabase.auth.signInWithOtp({
        email: correo,
        options: { shouldCreateUser: false },
      });
      if (error) throw error;
      setEnviado(true);
      return null;
    } catch (error) {
      const mensaje = String(error instanceof Error ? error.message : error);
      setError(mensaje);
      return mensaje;
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
