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
