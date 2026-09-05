import { useState } from "react";
import { crearClienteVerificacion } from "../lib/supabase";

export type PasoVerificacion = "inicial" | "codigo_enviado";

/**
 * Step-up authentication por correo antes de una acción sensible (hoy: dar
 * de alta un administrador) -- reusa el OTP nativo de Supabase
 * (`signInWithOtp`/`verifyOtp`) en vez de armar envío de correo a mano, pero
 * en un cliente descartable (`crearClienteVerificacion`) para no tocar la
 * sesión real de Google mientras tanto. Mismo criterio que la verificación
 * en dos pasos al activar un dispositivo (ver
 * docs/plan-panel-administrativo-web.md): el código sólo prueba que quien
 * está frente a la pantalla ahora mismo controla ese correo, un gate humano
 * en el momento de la acción, no en el login.
 */
export function useVerificacionPorCorreo(correo: string) {
  const [paso, setPaso] = useState<PasoVerificacion>("inicial");
  const [enviando, setEnviando] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function pedirCodigo() {
    setEnviando(true);
    setError(null);
    try {
      const cliente = crearClienteVerificacion();
      const { error } = await cliente.auth.signInWithOtp({
        email: correo,
        options: { shouldCreateUser: false },
      });
      if (error) throw error;
      setPaso("codigo_enviado");
    } catch (error) {
      setError(String(error instanceof Error ? error.message : error));
    } finally {
      setEnviando(false);
    }
  }

  async function verificarCodigo(codigo: string): Promise<boolean> {
    setEnviando(true);
    setError(null);
    try {
      const cliente = crearClienteVerificacion();
      const { error } = await cliente.auth.verifyOtp({ email: correo, token: codigo, type: "email" });
      if (error) throw error;
      return true;
    } catch (error) {
      setError(String(error instanceof Error ? error.message : error));
      return false;
    } finally {
      setEnviando(false);
    }
  }

  function reiniciar() {
    setPaso("inicial");
    setError(null);
  }

  return { paso, enviando, error, pedirCodigo, verificarCodigo, reiniciar };
}
