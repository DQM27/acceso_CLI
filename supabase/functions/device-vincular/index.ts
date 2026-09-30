import "jsr:@supabase/functions-js/edge-runtime.d.ts";
import { clienteServicio } from "../_shared/admin.ts";
import { ipDelCliente, json, leerCuerpo, preflight, textoOpcional } from "../_shared/http.ts";
import {
  LARGO_CODIGO,
  clavePublicaValida,
  emitirTokenDispositivo,
  huellaClave,
  metadataSaneada,
  normalizarCodigo,
  registrarEvento,
  sha256Hex,
  type TipoEventoSeguridad,
} from "../_shared/dispositivos.ts";

// Canje del código de vinculación que emitió el panel. El equipo manda:
//
//   { "codigo": "K7QM-R4XT-2P", "clave_publica_jwk": {kty,crv,x,y},
//     "dispositivo_esperado"?: uuid, "metadata"? }
//
// y, si el código es vigente y no se usó, queda atado a esa clave pública
// (canje atómico en `canjear_codigo_vinculacion`) y recibe su primer token,
// con la misma forma que responde `device-auth`. La clave privada nunca viaja.
//
// `dispositivo_esperado` lo manda un equipo que se re-vincula con datos
// locales ya cargados: un código de otro dispositivo se rechaza sin gastarse.
//
// Cualquier rechazo responde lo mismo (`codigo_invalido`) para no ayudar a
// adivinar, pero el motivo real queda en `eventos_seguridad_dispositivos`
// con la IP, visible en el panel.
//
// Pública a propósito (`verify_jwt = false`): el equipo todavía no tiene
// ninguna credencial.

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

Deno.serve(async (req: Request) => {
  const respuestaPreflight = preflight(req);
  if (respuestaPreflight) return respuestaPreflight;
  if (req.method !== "POST") return json({ error: "method_not_allowed" }, 405);

  const cuerpo = await leerCuerpo(req);
  if (!cuerpo || typeof cuerpo.codigo !== "string") return json({ error: "bad_request" }, 400);

  const clave = await clavePublicaValida(cuerpo.clave_publica_jwk);
  if (!clave) return json({ error: "clave_publica_invalida" }, 400);

  const supabase = clienteServicio();
  const ip = ipDelCliente(req);
  const metadata = metadataSaneada(cuerpo.metadata);
  const codigo = normalizarCodigo(cuerpo.codigo);
  const dispositivoEsperado = textoOpcional(cuerpo.dispositivo_esperado);
  if (dispositivoEsperado && !UUID.test(dispositivoEsperado)) return json({ error: "bad_request" }, 400);

  if (codigo.length !== LARGO_CODIGO) {
    await registrarEvento(supabase, { tipo: "codigo_inexistente", ip, detalle: { metadata } });
    return json({ error: "codigo_invalido" }, 401);
  }

  const codigoHash = await sha256Hex(codigo);
  const huella = await huellaClave(clave);

  const { data, error } = await supabase.rpc("canjear_codigo_vinculacion", {
    p_codigo_hash: codigoHash,
    p_clave_publica_jwk: clave,
    p_clave_huella: huella,
    p_metadata: metadata,
    p_ip: ip,
    p_dispositivo_esperado: dispositivoEsperado,
  });

  // Índice único de `clave_huella`: esa clave ya pertenece a otro
  // dispositivo. Todo se revirtió, incluido el uso del código.
  if (error?.code === "23505") return json({ error: "clave_en_uso" }, 409);
  if (error) return json({ error: "vinculacion_error", detail: error.message }, 500);

  const vinculado = (data as { dispositivo_id: string; sitio_id: string; tipo: string }[] | null)?.[0];
  if (!vinculado) {
    await registrarRechazo(supabase, codigoHash, dispositivoEsperado, ip, metadata);
    return json({ error: "codigo_invalido" }, 401);
  }

  const token = await emitirTokenDispositivo({
    id: vinculado.dispositivo_id,
    sitio_id: vinculado.sitio_id,
    tipo: vinculado.tipo,
    clave_huella: huella,
  });
  return json(token);
});

/** Averigua por qué no se pudo canjear y lo deja registrado. */
async function registrarRechazo(
  supabase: ReturnType<typeof clienteServicio>,
  codigoHash: string,
  dispositivoEsperado: string | null,
  ip: string | null,
  metadata: unknown,
): Promise<void> {
  const { data: fila } = await supabase
    .from("codigos_vinculacion")
    .select("dispositivo_id, usado_en, anulado_en, expira_en")
    .eq("codigo_hash", codigoHash)
    .maybeSingle();

  let tipo: TipoEventoSeguridad = "codigo_inexistente";
  if (fila?.usado_en) tipo = "codigo_usado";
  else if (fila?.anulado_en) tipo = "codigo_anulado";
  else if (fila && new Date(fila.expira_en) <= new Date()) tipo = "codigo_vencido";
  else if (fila && dispositivoEsperado && fila.dispositivo_id !== dispositivoEsperado) {
    tipo = "codigo_de_otro_dispositivo";
  }

  await registrarEvento(supabase, {
    tipo,
    dispositivo_id: fila?.dispositivo_id ?? null,
    ip,
    detalle: { metadata, dispositivo_esperado: dispositivoEsperado },
  });
}
