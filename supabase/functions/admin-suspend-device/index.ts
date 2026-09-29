import "jsr:@supabase/functions-js/edge-runtime.d.ts";
import { clienteServicio, correoAdminAutorizado } from "../_shared/admin.ts";
import { json, leerCuerpo, preflight, textoOpcional } from "../_shared/http.ts";

// Suspensión temporal (reversible), distinta de revocar. Igual que revocar,
// corta el acceso en el acto y avisa al equipo por Realtime.
//
//   { "dispositivo_id": uuid, "suspendido": boolean }

Deno.serve(async (req: Request) => {
  const respuestaPreflight = preflight(req);
  if (respuestaPreflight) return respuestaPreflight;
  if (req.method !== "POST") return json({ error: "method_not_allowed" }, 405);

  const supabase = clienteServicio();
  if (!(await correoAdminAutorizado(req, supabase))) return json({ error: "unauthorized" }, 401);

  const cuerpo = await leerCuerpo(req);
  const dispositivoId = textoOpcional(cuerpo?.dispositivo_id);
  if (!dispositivoId || typeof cuerpo?.suspendido !== "boolean") return json({ error: "bad_request" }, 400);

  const { error } = await supabase
    .from("dispositivos")
    .update({ suspended_at: cuerpo.suspendido ? new Date().toISOString() : null })
    .eq("id", dispositivoId);
  if (error) return json({ error: "suspend_error", detail: error.message }, 500);

  return json({ ok: true });
});
