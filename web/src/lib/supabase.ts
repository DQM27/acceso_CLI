import { createClient } from "@supabase/supabase-js";

/**
 * Cliente de Supabase para el navegador. La `publishable key` es pública a
 * propósito (reemplazo nuevo de la `anon key` de siempre, Supabase la
 * diseña para vivir en el cliente) — la seguridad real la dan las
 * políticas RLS de cada tabla, no esconder esta clave. Ver
 * `administradores_panel` (migración `crea_administradores_panel`): decide
 * quién puede entrar, no esta clave.
 */
const SUPABASE_URL = "https://xidaepyaljzkpbsxrqsm.supabase.co";
const SUPABASE_PUBLISHABLE_KEY = "sb_publishable_Sr9DPGMD7MFirLQfG7ViWg_6pJeEpqU";

export const supabase = createClient(SUPABASE_URL, SUPABASE_PUBLISHABLE_KEY);

/**
 * Cliente aparte, SIN persistir sesión, sólo para el código de verificación
 * por correo antes de una acción sensible (ver `useVerificacionPorCorreo`).
 * A propósito no es el mismo `supabase` de arriba: `verifyOtp` reemplaza la
 * sesión activa del cliente que lo llama, y `onAuthStateChange` de
 * `AuthContexto` reacciona a eso -- usar el cliente principal acá tira la
 * sesión de Google de vuelta a "cargando" a mitad de una acción, perdiendo
 * el estado de la pantalla (paso del formulario, etc.). Este cliente
 * desechable nunca toca ese listener.
 */
export function crearClienteVerificacion() {
  return createClient(SUPABASE_URL, SUPABASE_PUBLISHABLE_KEY, {
    auth: { persistSession: false, autoRefreshToken: false },
  });
}
