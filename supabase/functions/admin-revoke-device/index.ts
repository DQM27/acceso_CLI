import "jsr:@supabase/functions-js@2/edge-runtime.d.ts";
import { clienteServicio, correoAdminAutorizado } from "../_shared/admin.ts";
import { json, leerCuerpo, preflight, textoOpcional } from "../_shared/http.ts";

// Baja permanente de un dispositivo. Corta el acceso en el acto: la política
// restrictiva `solo dispositivos vigentes` rechaza su token aunque no haya
// vencido, y el trigger `dispositivos_avisar_expulsion` avisa al equipo por
// Realtime. También anula cualquier código de vinculación pendiente.

Deno.serve(async (req: Request) => {
  const respuestaPreflight = preflight(req);
  if (respuestaPreflight) return respuestaPreflight;
  if (req.method !== "POST") return json({ error: "method_not_allowed" }, 405);

  const supabase = clienteServicio();
  if (!(await correoAdminAutorizado(req, supabase))) return json({ error: "unauthorized" }, 401);

  const cuerpo = await leerCuerpo(req);
  const dispositivoId = textoOpcional(cuerpo?.dispositivo_id);
  if (!dispositivoId) return json({ error: "bad_request" }, 400);

  const ahora = new Date().toISOString();
  const { error } = await supabase.from("dispositivos").update({ revoked_at: ahora }).eq("id", dispositivoId);
  if (error) return json({ error: "revoke_error", detail: error.message }, 500);

  const { error: codigosError } = await supabase
    .from("codigos_vinculacion")
    .update({ anulado_en: ahora })
    .eq("dispositivo_id", dispositivoId)
    .is("usado_en", null)
    .is("anulado_en", null);
  if (codigosError) return json({ error: "revoke_error", detail: codigosError.message }, 500);

  return json({ ok: true });
});
