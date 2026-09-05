import { createClient } from "@supabase/supabase-js";

/**
 * Cliente de Supabase para el navegador. La `anon key` es pública a
 * propósito (Supabase la diseña para vivir en el cliente) — la seguridad
 * real la dan las políticas RLS de cada tabla, no esconder esta clave. Ver
 * `administradores_panel` (migración `crea_administradores_panel`): decide
 * quién puede entrar, no esta clave.
 *
 * TODO: pegar la `anon key` real (Settings → API → Project API keys, en el
 * dashboard de Supabase) antes de correr esto contra el proyecto real.
 */
const SUPABASE_URL = "https://xidaepyaljzkpbsxrqsm.supabase.co";
const SUPABASE_ANON_KEY = "PEGAR_ANON_KEY_AQUI";

export const supabase = createClient(SUPABASE_URL, SUPABASE_ANON_KEY);
