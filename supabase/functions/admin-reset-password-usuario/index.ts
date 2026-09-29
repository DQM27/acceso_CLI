import "jsr:@supabase/functions-js/edge-runtime.d.ts";
import { clienteServicio, correoAdminAutorizado } from "../_shared/admin.ts";
import { json, preflight } from "../_shared/http.ts";

// Mismo alfabeto/longitud que admin-create-usuario -- ver ese archivo para
// el razonamiento de entropía/legibilidad.
const ALFABETO_PASSWORD_TEMPORAL = "23456789ABCDEFGHJKLMNPQRSTUVWXYZ";

function generarPasswordTemporal(longitud = 10): string {
  const bytes = new Uint8Array(longitud);
  crypto.getRandomValues(bytes);
  return Array.from(bytes, (b) => ALFABETO_PASSWORD_TEMPORAL[b % ALFABETO_PASSWORD_TEMPORAL.length]).join("");
}

/**
 * Genera una contraseña temporal nueva para un usuario ya existente --
 * "olvidó la contraseña" o backfill de una cédula creada antes de este
 * cambio (auth_user_id todavía NULL). Mismo actor privilegiado que
 * admin-create-usuario, ver docs/planes-implementados/plan-autenticacion-supabase-auth.md.
 */
Deno.serve(async (req: Request) => {
  const respuestaPreflight = preflight(req);
  if (respuestaPreflight) return respuestaPreflight;
  if (req.method !== "POST") return json({ error: "method_not_allowed" }, 405);

  const supabase = clienteServicio();

  if (!(await correoAdminAutorizado(req, supabase))) {
    return json({ error: "unauthorized" }, 401);
  }

  let body: { usuario_id?: string };
  try {
    body = await req.json();
  } catch {
    return json({ error: "bad_request" }, 400);
  }

  const usuarioId = body.usuario_id?.trim();
  if (!usuarioId) return json({ error: "bad_request" }, 400);

  const { data: usuario, error: usuarioError } = await supabase
    .from("usuarios")
    .select("id, cedula, auth_user_id")
    .eq("id", usuarioId)
    .maybeSingle();

  if (usuarioError || !usuario) {
    return json({ error: "usuario_no_encontrado" }, 404);
  }

  const passwordTemporal = generarPasswordTemporal();

  if (usuario.auth_user_id) {
    // Caso normal: ya tenía cuenta de Auth, se le pisa la contraseña.
    const { error: updateError } = await supabase.auth.admin.updateUserById(usuario.auth_user_id, {
      password: passwordTemporal,
      user_metadata: { cedula: usuario.cedula, debe_cambiar_password: true },
    });
    if (updateError) return json({ error: "auth_error", detail: updateError.message }, 500);
  } else {
    // Backfill: usuario creado antes de este cambio, todavía sin cuenta de
    // Auth -- se crea ahora y se enlaza.
    const { data: authUser, error: createError } = await supabase.auth.admin.createUser({
      email: `${usuario.cedula}@brisas.local`,
      password: passwordTemporal,
      email_confirm: true,
      user_metadata: { cedula: usuario.cedula, debe_cambiar_password: true },
    });
    if (createError || !authUser?.user) {
      return json({ error: "auth_error", detail: createError?.message }, 500);
    }
    const { error: linkError } = await supabase
      .from("usuarios")
      .update({ auth_user_id: authUser.user.id })
      .eq("id", usuarioId);
    if (linkError) {
      await supabase.auth.admin.deleteUser(authUser.user.id);
      return json({ error: "usuario_error", detail: linkError.message }, 500);
    }
  }

  return json({ usuario_id: usuarioId, password_temporal: passwordTemporal });
});
