import { useState } from "react";
import { supabase } from "../lib/supabase";

/**
 * Step-up authentication por correo antes de una acción sensible (agregar
 * o quitar un administrador) -- reusa el magic link nativo de Supabase
 * (`signInWithOtp`) en vez de armar envío de correo a mano. Sin código para
 * escribir a propósito: mostrar el código de 6 dígitos en el correo
 * requiere editar la plantilla de "Magic Link", y eso exige conectar SMTP
 * propio (bloqueado en el plan gratis de Supabase) -- ver conversación.
 * Mientras tanto, la persona hace clic en el link del correo y listo: al
 * volver a abrir el panel, `App.tsx` retoma la acción guardada (ver
 * `accionesPendientes.ts`).
 *
 * A propósito NO usa un cliente aparte: `signInWithOtp` sólo manda el
 * correo, no cambia la sesión activa (a diferencia de `verifyOtp`, que acá
 * ni se llama -- la confirmación pasa por el redirect del link, no por
 * código escrito en esta pestaña).
 */
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
      setError(String(error instanceof Error ? error.message : error));
    } finally {
      setEnviando(false);
    }
  }

  function reiniciar() {
    setEnviado(false);
    setError(null);
  }

  return { enviado, enviando, error, pedirConfirmacion, reiniciar };
}
