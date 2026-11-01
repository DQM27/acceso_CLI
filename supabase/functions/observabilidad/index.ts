import "jsr:@supabase/functions-js@2/edge-runtime.d.ts";
import { json, preflight } from "../_shared/http.ts";
import { armarLote } from "./errores.ts";
import { descomprimir, leerConTope, leerLote, traducirRespuestaRpc } from "./logica.ts";

// Receptora de la telemetría y los errores de los equipos (fases 1a y 1b de
// docs/features-futuras/plan-observabilidad-en-produccion.md).
//
// - `POST .../observabilidad/telemetria`: el lote columnar, a
//   `registrar_telemetria`.
// - `POST .../observabilidad/errores`: `{"reportes": [...]}` (formato
//   `control_acceso::errores::Reporte`). Cada reporte se limpia otra vez de
//   datos personales y se le calcula la huella (errores.ts) antes de pasarlo
//   a `registrar_errores`.
//
// Las dos con el cuerpo comprimido (`Content-Encoding: br`; `gzip` y sin
// comprimir también se aceptan) y el token del equipo en `Authorization`.
// Existe porque la puerta de entrada de Supabase no descomprime el cuerpo de
// las peticiones: probado, con `br` o `gzip` PostgREST responde "Empty or
// invalid json".
//
// No tiene privilegios: NO usa `service_role`. Llama a la base reenviando el
// MISMO token del equipo, así que quién es, si está vigente, los límites y
// las validaciones los decide la base. Una petición con un token falso llega
// a la base y la base la rechaza.
//
// `verify_jwt = false` (supabase/config.toml), como `device-auth`: el token
// lo verifica la base al recibir la llamada. Para que un cuerpo basura no
// gaste CPU, antes de descomprimir se exige un `Authorization: Bearer` y el
// cuerpo se lee con tope (256 KB comprimido, 1 MB descomprimido). Nunca se
// registra el contenido de una petición.
//
// Respuestas (las entienden `telemetria::envio` y `errores::envio` en el
// núcleo): 200 con lo que devolvió la base; 400/413 sin arreglo (el equipo
// lo descarta); 401/403 renovar el token; 415 codificación desconocida;
// 429/502/503 reintentar.

const SUPABASE_URL = Deno.env.get("SUPABASE_URL") ?? "";
const CLAVE_PUBLICA = Deno.env.get("SUPABASE_ANON_KEY") ?? "";

async function llamarBase(
  req: Request,
  autorizacion: string,
  funcion: "registrar_telemetria" | "registrar_errores",
  argumentos: Record<string, unknown>,
): Promise<{ estado: number; cuerpo: Record<string, unknown> }> {
  let respuesta: Response;
  try {
    respuesta = await fetch(`${SUPABASE_URL}/rest/v1/rpc/${funcion}`, {
      method: "POST",
      headers: {
        apikey: req.headers.get("apikey") ?? CLAVE_PUBLICA,
        authorization: autorizacion,
        "content-type": "application/json",
      },
      body: JSON.stringify(argumentos),
    });
  } catch (error) {
    console.error("observabilidad: no se pudo llamar a la base:", error instanceof Error ? error.message : error);
    return { estado: 502, cuerpo: { error: "base_no_disponible" } };
  }
  const cuerpoRpc: unknown = await respuesta.json().catch(() => null);
  const traducida = traducirRespuestaRpc(respuesta.status, cuerpoRpc);
  if (traducida.estado >= 500) console.error("observabilidad:", funcion, "respondió", respuesta.status, traducida.cuerpo);
  return traducida;
}

Deno.serve(async (req: Request) => {
  const respuestaPreflight = preflight(req);
  if (respuestaPreflight) return respuestaPreflight;

  const ruta = new URL(req.url).pathname.match(/\/(telemetria|errores)\/?$/)?.[1];
  if (req.method !== "POST" || !ruta) {
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

  const cuerpo = leerLote(descomprimido.valor);
  if (!cuerpo.ok) return json({ error: cuerpo.error }, cuerpo.estado);

  if (ruta === "telemetria") {
    const { estado, cuerpo: respuesta } = await llamarBase(req, autorizacion, "registrar_telemetria", {
      p_lote: cuerpo.valor,
    });
    return json(respuesta, estado);
  }

  const lote = await armarLote(cuerpo.valor);
  if (!lote.ok) return json({ error: lote.error }, lote.estado);
  // Aunque no quede ningún reporte útil se llama a la base: es la que
  // verifica el token.
  const { estado, cuerpo: respuesta } = await llamarBase(req, autorizacion, "registrar_errores", {
    p_reportes: lote.errores,
  });
  return json(estado === 200 ? { ...respuesta, descartados: lote.descartados } : respuesta, estado);
});
