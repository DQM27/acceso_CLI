import "jsr:@supabase/functions-js@2/edge-runtime.d.ts";
import { clienteServicio, correoAdminAutorizado } from "../_shared/admin.ts";
import { json, leerCuerpo, preflight, textoOpcional } from "../_shared/http.ts";

// Crea un sitio suelto, sin dispositivo -- para el desplegable de "Sitio" en
// el alta de dispositivos (ver web/src/pantallas/Dispositivos.tsx). Si ya
// existe uno con ese nombre, lo devuelve en vez de duplicarlo.
//
// `direccion` se elimino de `sitios` (2026-09-12): se pedia opcionalmente
// al crear pero nunca se mostraba ni editaba despues -- write-only, dato
// inaccesible una vez guardado.
Deno.serve(async (req: Request) => {
  const respuestaPreflight = preflight(req);
  if (respuestaPreflight) return respuestaPreflight;
  if (req.method !== "POST") return json({ error: "method_not_allowed" }, 405);

  const supabase = clienteServicio();
  if (!(await correoAdminAutorizado(req, supabase))) return json({ error: "unauthorized" }, 401);

  const cuerpo = await leerCuerpo(req);
  const nombre = textoOpcional(cuerpo?.nombre, 120);
  if (!nombre) return json({ error: "bad_request", detail: "falta el nombre" }, 400);

  const { data: sitio, error } = await supabase
    .from("sitios")
    .upsert({ nombre }, { onConflict: "nombre" })
    .select("id, nombre")
    .single();

  if (error || !sitio) return json({ error: "sitio_error", detail: error?.message }, 500);

  return json(sitio);
});
