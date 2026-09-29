// Autorización de las funciones `admin-*`: una sola implementación en vez
// de la copia que tenía cada función.
//
// Quien llama debe traer un JWT de sesión de Supabase Auth (Google OAuth
// desde el panel web) Y su correo debe estar en `administradores_panel`:
// mismo criterio que `private.es_admin_global()` en RLS, pero verificado a
// mano porque estas funciones usan el service role y saltan RLS.

import { createClient, type SupabaseClient } from "jsr:@supabase/supabase-js@2";

const SUPABASE_URL = Deno.env.get("SUPABASE_URL")!;
const SERVICE_ROLE_KEY = Deno.env.get("SUPABASE_SERVICE_ROLE_KEY")!;

/** Cliente con service role: salta RLS, sólo para usar tras autorizar. */
export function clienteServicio(): SupabaseClient {
  return createClient(SUPABASE_URL, SERVICE_ROLE_KEY);
}

/** Correo del administrador autorizado, o `null` si no lo es. */
export async function correoAdminAutorizado(req: Request, admin: SupabaseClient): Promise<string | null> {
  const token = (req.headers.get("authorization") ?? "").replace(/^Bearer\s+/i, "").trim();
  if (!token) return null;

  const { data: userData, error: userError } = await admin.auth.getUser(token);
  const correo = userData?.user?.email;
  if (userError || !correo) return null;

  const { data: fila } = await admin
    .from("administradores_panel")
    .select("correo")
    .eq("correo", correo)
    .maybeSingle();

  return fila ? correo : null;
}
