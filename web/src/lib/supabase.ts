import { createClient } from "@supabase/supabase-js";

/**
 * Cliente de Supabase para el navegador. La `publishable key` es pública a
 * propósito (reemplazo nuevo de la `anon key` de siempre, Supabase la
 * diseña para vivir en el cliente) — la seguridad real la dan las
 * políticas RLS de cada tabla, no esconder esta clave. Ver
 * `administradores_panel` (migración `crea_administradores_panel`): decide
 * quién puede entrar, no esta clave.
 *
 * Vienen de `.env` (versionado, valores de producción por defecto) --
 * para apuntar el build local a staging sin tocar ese archivo, crear un
 * `.env.local` (gitignored) con las mismas dos variables y los valores de
 * `docs/recuperacion-sitio-staging.md`.
 */
const SUPABASE_URL = import.meta.env.VITE_SUPABASE_URL;
const SUPABASE_PUBLISHABLE_KEY = import.meta.env.VITE_SUPABASE_PUBLISHABLE_KEY;

export const supabase = createClient(SUPABASE_URL, SUPABASE_PUBLISHABLE_KEY);
