import "jsr:@supabase/functions-js@2/edge-runtime.d.ts";
import { createClient } from "jsr:@supabase/supabase-js@2";
import {
  calcularHuella,
  claveDeLaPeticion,
  claveValida,
  extraerEvento,
  leerEnvelope,
  MAX_BYTES_COMPRIMIDOS,
  MAX_BYTES_ENVELOPE,
  MAX_ITEMS,
} from "./logica.ts";

// Receptor de errores propio: el "Sentry propio". SÓLO staging.
//
// Las apps siguen usando el SDK de Sentry; sólo cambia el DSN, que apunta
// acá. Con el DSN
//
//   https://<clave>@<proyecto>.supabase.co/functions/v1/receptor-errores/1
//
// el SDK manda los envelopes a `.../receptor-errores/api/1/envelope/`. Cada
// evento se limpia de datos personales (ver logica.ts), se agrupa por huella
// y se guarda con `registrar_error_app`
// (supabase/scripts/errores_app_staging.sql). Todo lo que no es un evento
// (sesiones, adjuntos, transacciones) se ignora.
//
// Pública a propósito (`verify_jwt = false`): los SDK de Sentry no saben
// mandar el JWT de Supabase. Se autentica con la clave del DSN, que viaja
// dentro de las apps (no es un secreto) y sólo identifica al emisor: el
// control real es el límite de tamaño y el de ritmo (120 errores por minuto,
// en `registrar_error_app`). Nunca se registra el contenido de una petición.
//
// Secreto de la función: RECEPTOR_ERRORES_CLAVE (la clave del DSN).

const CLAVE = Deno.env.get("RECEPTOR_ERRORES_CLAVE") ?? "";
const supabase = createClient(
  Deno.env.get("SUPABASE_URL") ?? "",
  Deno.env.get("SUPABASE_SERVICE_ROLE_KEY") ?? "",
  { auth: { persistSession: false } },
);

function responder(estado: number, cuerpo: unknown = {}, extra: HeadersInit = {}): Response {
  return new Response(JSON.stringify(cuerpo), {
    status: estado,
    headers: { "Content-Type": "application/json", ...extra },
  });
}

/** El contenido sin comprimir, o `null` si pasa del límite o no es gzip válido. */
async function descomprimir(bytes: Uint8Array, limite: number): Promise<Uint8Array | null> {
  try {
    const flujo = new Blob([bytes]).stream().pipeThrough(new DecompressionStream("gzip"));
    const lector = flujo.getReader();
    const partes: Uint8Array[] = [];
    let total = 0;
    while (true) {
      const { done, value } = await lector.read();
      if (done) break;
      total += value.length;
      if (total > limite) {
        await lector.cancel();
        return null;
      }
      partes.push(value);
    }
    const salida = new Uint8Array(total);
    let posicion = 0;
    for (const parte of partes) {
      salida.set(parte, posicion);
      posicion += parte.length;
    }
    return salida;
  } catch {
    return null;
  }
}

Deno.serve(async (req: Request) => {
  const url = new URL(req.url);
  if (req.method !== "POST" || !/\/api\/[^/]+\/envelope\/?$/.test(url.pathname)) {
    return responder(404, { error: "no_encontrado" });
  }
  if (!claveValida(claveDeLaPeticion(req.headers, url), CLAVE)) {
    return responder(401, { error: "clave_invalida" });
  }

  const declarado = Number(req.headers.get("content-length") ?? 0);
  if (declarado > MAX_BYTES_COMPRIMIDOS) return responder(413, { error: "demasiado_grande" });
  let cuerpo = new Uint8Array(await req.arrayBuffer());
  if (cuerpo.length > MAX_BYTES_COMPRIMIDOS) return responder(413, { error: "demasiado_grande" });

  if ((req.headers.get("content-encoding") ?? "").toLowerCase().includes("gzip")) {
    const plano = await descomprimir(cuerpo, MAX_BYTES_ENVELOPE);
    if (!plano) return responder(413, { error: "demasiado_grande" });
    cuerpo = plano;
  } else if (cuerpo.length > MAX_BYTES_ENVELOPE) {
    return responder(413, { error: "demasiado_grande" });
  }

  const envelope = leerEnvelope(cuerpo);
  if (!envelope) return responder(400, { error: "envelope_invalido" });

  for (const item of envelope.items.slice(0, MAX_ITEMS)) {
    if (item.cabecera.type !== "event") continue;
    const evento = extraerEvento(item.payload);
    if (!evento) continue;
    const huella = await calcularHuella(evento);
    const { data, error } = await supabase.rpc("registrar_error_app", { p: { ...evento, huella } });
    if (error) {
      // Sin el contenido: sólo el código, para no volcar datos al registro.
      console.error("registrar_error_app falló:", error.code ?? "sin_codigo");
      return responder(503, { error: "no_disponible" }, { "Retry-After": "60" });
    }
    if (data === "limite") return responder(429, { error: "limite" }, { "Retry-After": "60" });
  }

  // Los SDK esperan el id del evento en la respuesta.
  return responder(200, { id: typeof envelope.encabezado.event_id === "string" ? envelope.encabezado.event_id : "" });
});
