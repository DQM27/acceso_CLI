import "jsr:@supabase/functions-js@2/edge-runtime.d.ts";
import { decodeProtectedHeader } from "npm:jose@5";
import { clienteServicio } from "../_shared/admin.ts";
import { enSegundoPlano, ipDelCliente, json, leerCuerpo, preflight } from "../_shared/http.ts";
import {
  COLUMNAS_NOMBRES,
  TTL_DESAFIO_SEGUNDOS,
  asercionValida,
  emitirDesafio,
  emitirTokenDispositivo,
  metadataSaneada,
  nombresDe,
  registrarEvento,
  type ClavePublicaP256,
  type MetadatosDispositivo,
} from "../_shared/dispositivos.ts";

// Autentica un dispositivo ya vinculado (ver device-vincular) y le emite su
// token de sesión. Dos pasos (ver
// docs/features-futuras/propuesta-registro-dispositivos.md):
//
// 1. `{ "desafio": true }` → `{ desafio, expires_in }`.
// 2. `{ "asercion": "<JWS>", "metadata"? }` → token. El equipo firma el
//    desafío con su clave privada (ES256, `kid` = huella RFC 7638 de su clave
//    pública, `aud` = "device-auth").
//
// No existe otro camino: no hay secretos compartidos.
//
// Pública a propósito (`verify_jwt = false`): es la puerta de entrada.

// Secret opcional: sin él, el chequeo de versión mínima queda desactivado.
// `supabase secrets set VERSION_MINIMA_ACEPTADA=1.5.0`. Ver
// docs/auditorias/plan-qa-buenas-practicas-2026-09-17.md, punto 9.
const VERSION_MINIMA_ACEPTADA = Deno.env.get("VERSION_MINIMA_ACEPTADA") ?? null;

/** `true` si `version` ("major.minor.patch") es menor que `minima`. */
function versionPorDebajoDe(version: string, minima: string): boolean {
  const a = version.split(".").map((parte) => Number.parseInt(parte, 10) || 0);
  const b = minima.split(".").map((parte) => Number.parseInt(parte, 10) || 0);
  for (let i = 0; i < Math.max(a.length, b.length); i++) {
    const x = a[i] ?? 0;
    const y = b[i] ?? 0;
    if (x !== y) return x < y;
  }
  return false;
}

interface FilaDispositivo {
  id: string;
  sitio_id: string;
  tipo: string;
  clave_huella: string | null;
  clave_publica_jwk: ClavePublicaP256 | null;
  identificador_hardware: string | null;
  etiqueta: string | null;
  sitios: { nombre: string } | null;
}

const COLUMNAS = `id, sitio_id, tipo, clave_huella, clave_publica_jwk, identificador_hardware, ${COLUMNAS_NOMBRES}`;

const credencialesInvalidas = () => json({ error: "invalid_credentials" }, 401);

Deno.serve(async (req: Request) => {
  const respuestaPreflight = preflight(req);
  if (respuestaPreflight) return respuestaPreflight;
  if (req.method !== "POST") return json({ error: "method_not_allowed" }, 405);

  const cuerpo = await leerCuerpo(req);
  if (!cuerpo) return json({ error: "bad_request" }, 400);

  if (cuerpo.desafio === true) {
    return json({ desafio: await emitirDesafio(), expires_in: TTL_DESAFIO_SEGUNDOS });
  }

  const supabase = clienteServicio();
  const ip = ipDelCliente(req);
  const metadata = metadataSaneada(cuerpo.metadata);

  if (typeof cuerpo.asercion !== "string") return json({ error: "bad_request" }, 400);
  const dispositivo = await autenticarConAsercion(supabase, cuerpo.asercion, ip);
  if (!dispositivo) return credencialesInvalidas();

  // Sin versión mínima configurada, o sin que el cliente la mande, no se
  // bloquea: "no sé" nunca es motivo para rechazar. 426 Upgrade Required.
  if (
    VERSION_MINIMA_ACEPTADA &&
    metadata.app_version &&
    versionPorDebajoDe(metadata.app_version, VERSION_MINIMA_ACEPTADA)
  ) {
    return json({ error: "version_desactualizada", version_minima: VERSION_MINIMA_ACEPTADA }, 426);
  }

  enSegundoPlano(actualizarRastro(supabase, dispositivo, metadata, ip));

  return json(await emitirTokenDispositivo({ ...dispositivo, ...nombresDe(dispositivo) }));
});

async function autenticarConAsercion(
  supabase: ReturnType<typeof clienteServicio>,
  asercion: string,
  ip: string | null,
): Promise<FilaDispositivo | null> {
  let huella: string | undefined;
  try {
    huella = decodeProtectedHeader(asercion).kid;
  } catch {
    return null;
  }
  if (!huella) return null;

  const { data: dispositivo } = await supabase
    .from("dispositivos")
    .select(COLUMNAS)
    .eq("clave_huella", huella)
    .is("revoked_at", null)
    .maybeSingle<FilaDispositivo>();
  if (!dispositivo?.clave_publica_jwk) return null;

  if (!(await asercionValida(asercion, dispositivo.clave_publica_jwk))) {
    await registrarEvento(supabase, { tipo: "firma_invalida", dispositivo_id: dispositivo.id, ip });
    return null;
  }
  return dispositivo;
}

/**
 * Rastro informativo (`last_seen_at`, IP, metadata). Nunca pisa el
 * identificador de hardware ya registrado: si llega uno distinto, queda como
 * evento de seguridad para el panel en vez de borrar la evidencia.
 */
async function actualizarRastro(
  supabase: ReturnType<typeof clienteServicio>,
  dispositivo: FilaDispositivo,
  metadata: MetadatosDispositivo,
  ip: string | null,
): Promise<void> {
  const actualizacion: Record<string, string> = { last_seen_at: new Date().toISOString() };
  if (ip) actualizacion.last_ip = ip;
  for (const [campo, valor] of Object.entries(metadata)) {
    if (valor && campo !== "identificador_hardware") actualizacion[campo] = valor;
  }

  const hardware = metadata.identificador_hardware;
  if (hardware && !dispositivo.identificador_hardware) {
    actualizacion.identificador_hardware = hardware;
  } else if (hardware && hardware !== dispositivo.identificador_hardware) {
    await registrarEvento(supabase, {
      tipo: "hardware_distinto",
      dispositivo_id: dispositivo.id,
      ip,
      detalle: { registrado: dispositivo.identificador_hardware, recibido: hardware, metadata },
    });
  }

  const { error } = await supabase.from("dispositivos").update(actualizacion).eq("id", dispositivo.id);
  if (error) console.error("no se pudo actualizar el rastro del dispositivo:", error.message);
}
