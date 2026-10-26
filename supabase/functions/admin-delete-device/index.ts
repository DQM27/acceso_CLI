import "jsr:@supabase/functions-js@2/edge-runtime.d.ts";
import { clienteServicio, correoAdminAutorizado } from "../_shared/admin.ts";
import { json, leerCuerpo, preflight, textoOpcional } from "../_shared/http.ts";

// "Eliminar" en el panel RETIRA el equipo (si no lo estaba) y lo OCULTA de
// la lista. Nunca borra la fila.
//
// Retirar primero: eliminar le corta el paso para siempre, como retirar. Al
// retirarlo, la base cierra sus sesiones en la bitácora
// (`cerrar_sesiones_al_retirar_equipo`) y le avisa la expulsión en vivo,
// igual que `admin-revoke-device`.
//
// Nunca borrar (desde el incidente del 2026-10-26): la fila retirada es la
// PRUEBA con la que `device-auth` le responde a ese equipo "fuiste retirado"
// (403). Sin la fila, la nube sólo podría decir "no te conozco", y eso nunca
// autoriza a un equipo a mostrarse como retirado (ver
// `device-auth/autenticacion.ts`). Antes, un equipo sin historial se borraba
// del todo y uno con historial quedaba oculto: ahora todos quedan ocultos,
// con su historial intacto.
Deno.serve(async (req: Request) => {
  const respuestaPreflight = preflight(req);
  if (respuestaPreflight) return respuestaPreflight;
  if (req.method !== "POST") return json({ error: "method_not_allowed" }, 405);

  const supabase = clienteServicio();
  if (!(await correoAdminAutorizado(req, supabase))) return json({ error: "unauthorized" }, 401);

  const cuerpo = await leerCuerpo(req);
  const dispositivoId = textoOpcional(cuerpo?.dispositivo_id);
  if (!dispositivoId) return json({ error: "bad_request" }, 400);

  // Sólo si no estaba retirado: un retiro anterior conserva su fecha.
  const { error: revokeError } = await supabase
    .from("dispositivos")
    .update({ revoked_at: new Date().toISOString() })
    .eq("id", dispositivoId)
    .is("revoked_at", null);
  if (revokeError) return json({ error: "revoke_error", detail: revokeError.message }, 500);

  const { error: hideError } = await supabase
    .from("dispositivos")
    .update({ oculto_en_panel: true })
    .eq("id", dispositivoId);
  if (hideError) return json({ error: "hide_error", detail: hideError.message }, 500);

  return json({ ok: true });
});
