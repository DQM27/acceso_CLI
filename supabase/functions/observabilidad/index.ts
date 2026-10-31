import "jsr:@supabase/functions-js@2/edge-runtime.d.ts";
import { json, preflight } from "../_shared/http.ts";
import { descomprimir, leerConTope, leerLote, traducirRespuestaRpc } from "./logica.ts";

// Receptora de la telemetría de los equipos (fase 1b de
// docs/features-futuras/plan-observabilidad-en-produccion.md).
//
// `POST .../observabilidad/telemetria` con el lote columnar comprimido
// (`Content-Encoding: br`; `gzip` y sin comprimir también se aceptan) y el
// token del equipo en `Authorization`. Existe porque la puerta de entrada de
// Supabase no descomprime el cuerpo de las peticiones: probado, con `br` o
// `gzip` PostgREST responde "Empty or invalid json".
//
// No tiene privilegios: NO usa `service_role`. Descomprime y llama a
// `registrar_telemetria` reenviando el MISMO token del equipo, así que quién
// es, si está vigente, los límites y las validaciones los decide la base
// (`observabilidad.registrar_telemetria`). Una petición con un token falso
// llega a la base y la base la rechaza.
//
// `verify_jwt = false` (supabase/config.toml), como `device-auth`: el token
// lo verifica la base al recibir la llamada. Para que un cuerpo basura no
// gaste CPU, antes de descomprimir se exige un `Authorization: Bearer` y el
// cuerpo se lee con tope (256 KB comprimido, 1 MB descomprimido).
//
// Respuestas (las entiende `telemetria::envio` en el núcleo): 200 con lo que
// devolvió la base; 400/413 lote sin arreglo (el equipo lo descarta); 401/403
// renovar el token; 415 codificación desconocida; 429/502/503 reintentar.

const SUPABASE_URL = Deno.env.get("SUPABASE_URL") ?? "";
const CLAVE_PUBLICA = Deno.env.get("SUPABASE_ANON_KEY") ?? "";

Deno.serve(async (req: Request) => {
  const respuestaPreflight = preflight(req);
  if (respuestaPreflight) return respuestaPreflight;

  const ruta = new URL(req.url).pathname;
  if (req.method !== "POST" || !/\/telemetria\/?$/.test(ruta)) {
    return json({ error: "no_encontrado" }, 404);
  }

  const autorizacion = req.headers.get("authorization") ?? "";
  if (!autorizacion.startsWith("Bearer ") || autorizacion.length < 20) {
    return json({ error: "sin_token" }, 401);
  }

  const leido = await leerConTope(req.body);
  if (!leido.ok) return json({ error: leido.error }, leido.estado);

  const descomprimido = await descomprimir(leido.valor, req.headers.get("content-encoding"));
  if (!descomprimido.ok) return json({ error: descomprimido.error }, descomprimido.estado);

  const lote = leerLote(descomprimido.valor);
  if (!lote.ok) return json({ error: lote.error }, lote.estado);

  let respuesta: Response;
  try {
    respuesta = await fetch(`${SUPABASE_URL}/rest/v1/rpc/registrar_telemetria`, {
      method: "POST",
      headers: {
        apikey: req.headers.get("apikey") ?? CLAVE_PUBLICA,
        authorization: autorizacion,
        "content-type": "application/json",
      },
      body: JSON.stringify({ p_lote: lote.valor }),
    });
  } catch (error) {
    console.error("observabilidad: no se pudo llamar a la base:", error instanceof Error ? error.message : error);
    return json({ error: "base_no_disponible" }, 502);
  }

  const cuerpoRpc: unknown = await respuesta.json().catch(() => null);
  const { estado, cuerpo } = traducirRespuestaRpc(respuesta.status, cuerpoRpc);
  if (estado >= 500) console.error("observabilidad: la base respondió", respuesta.status, cuerpo);
  return json(cuerpo, estado);
});
