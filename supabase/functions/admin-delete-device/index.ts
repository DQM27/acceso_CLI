import "jsr:@supabase/functions-js@2/edge-runtime.d.ts";
import { clienteServicio, correoAdminAutorizado } from "../_shared/admin.ts";
import { json, leerCuerpo, preflight, textoOpcional } from "../_shared/http.ts";

// Primero RETIRA el equipo (si no lo estaba): eliminar le corta el paso
// para siempre, como retirar, aunque después no se pueda borrar. Antes, un
// equipo con historial que se "eliminaba" sin retirar sólo quedaba oculto en
// el panel y SEGUÍA pudiendo conectarse. Al retirarlo, la base cierra sus
// sesiones en la bitácora (`cerrar_sesiones_al_retirar_equipo`) y le avisa
// la expulsión en vivo, igual que `admin-revoke-device`.
//
// Después intenta un borrado definitivo. Las tablas que referencian
// dispositivo_origen_id / dispositivo_entrada_id / dispositivo_salida_id
// (contratistas, empresas, gafetes, usuarios, ingresos) NO tienen ON DELETE
// CASCADE ("NO ACTION") -- si el dispositivo ya genero historial real,
// Postgres rechaza el borrado con foreign_key_violation (23503). En ese
// caso, en vez de fallar, lo marcamos oculto_en_panel=true (ver migracion
// oculta_dispositivos_del_panel): sigue existiendo (con su historial
// intacto) pero deja de aparecer en la lista por defecto -- "al darle
// Eliminar, si no se puede borrar del todo, que al menos se oculte".
Deno.serve(async (req: Request) => {
  const respuestaPreflight = preflight(req);
  if (respuestaPreflight) return respuestaPreflight;
  if (req.method !== "POST") return json({ error: "method_not_allowed" }, 405);

  const supabase = clienteServicio();
  if (!(await correoAdminAutorizado(req, supabase))) return json({ error: "unauthorized" }, 401);

  const cuerpo = await leerCuerpo(req);
  const dispositivoId = textoOpcional(cuerpo?.dispositivo_id);
  if (!dispositivoId) return json({ error: "bad_request" }, 400);

  const { error: revokeError } = await supabase
    .from("dispositivos")
    .update({ revoked_at: new Date().toISOString() })
    .eq("id", dispositivoId)
    .is("revoked_at", null);
  if (revokeError) return json({ error: "revoke_error", detail: revokeError.message }, 500);

  const { error: deleteError } = await supabase.from("dispositivos").delete().eq("id", dispositivoId);
  if (!deleteError) return json({ ok: true, borrado: true });

  if (deleteError.code !== "23503") {
    return json({ error: "delete_error", detail: deleteError.message }, 500);
  }

  const { error: hideError } = await supabase
    .from("dispositivos")
    .update({ oculto_en_panel: true })
    .eq("id", dispositivoId);
  if (hideError) return json({ error: "hide_error", detail: hideError.message }, 500);

  return json({ ok: true, borrado: false });
});
