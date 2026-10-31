// Lógica pura de la receptora `observabilidad` (ver index.ts), separada para
// probarla sin red: leer el cuerpo con tope, descomprimirlo con tope, leer el
// lote y traducir la respuesta de la base a lo que espera el equipo.

import { Buffer } from "node:buffer";
import { createBrotliDecompress, createGunzip } from "node:zlib";

/** Cuerpo comprimido como máximo. Un lote real de 500 filas son ~8 KB. */
export const MAX_COMPRIMIDO = 256 * 1024;
/** Cuerpo descomprimido como máximo: lo que no entra se corta (bomba). */
export const MAX_DESCOMPRIMIDO = 1024 * 1024;

export type Resultado<T> =
  | { ok: true; valor: T }
  | { ok: false; estado: 400 | 413 | 415; error: string };

function falla(estado: 400 | 413 | 415, error: string): { ok: false; estado: 400 | 413 | 415; error: string } {
  return { ok: false, estado, error };
}

function unir(trozos: Uint8Array[], total: number): Uint8Array {
  const salida = new Uint8Array(total);
  let posicion = 0;
  for (const trozo of trozos) {
    salida.set(trozo, posicion);
    posicion += trozo.byteLength;
  }
  return salida;
}

/**
 * Lee el cuerpo sin pasar de `tope` bytes, aunque no traiga `content-length`
 * (cuerpo por partes): lo que se pase se corta sin terminar de leerlo.
 */
export async function leerConTope(
  cuerpo: ReadableStream<Uint8Array> | null,
  tope = MAX_COMPRIMIDO,
): Promise<Resultado<Uint8Array>> {
  if (!cuerpo) return { ok: true, valor: new Uint8Array() };
  const lector = cuerpo.getReader();
  const trozos: Uint8Array[] = [];
  let total = 0;
  let leido = await lector.read();
  while (!leido.done) {
    total += leido.value.byteLength;
    if (total > tope) {
      await lector.cancel();
      return falla(413, "demasiado_grande");
    }
    trozos.push(leido.value);
    leido = await lector.read();
  }
  return { ok: true, valor: unir(trozos, total) };
}

/**
 * Descomprime según `Content-Encoding` (`br`, `gzip` o sin codificar) sin
 * pasar de `tope` bytes: se lee por partes y se corta apenas se pasa, sin
 * reservar memoria para todo lo que el cuerpo prometa.
 */
export async function descomprimir(
  cuerpo: Uint8Array,
  codificacion: string | null,
  tope = MAX_DESCOMPRIMIDO,
): Promise<Resultado<Uint8Array>> {
  const forma = (codificacion ?? "").trim().toLowerCase();
  if (forma === "" || forma === "identity") {
    return cuerpo.byteLength > tope ? falla(413, "demasiado_grande") : { ok: true, valor: cuerpo };
  }
  const flujo = forma === "br" ? createBrotliDecompress() : forma === "gzip" ? createGunzip() : null;
  if (!flujo) return falla(415, "codificacion_no_soportada");

  const trozos: Uint8Array[] = [];
  let total = 0;
  flujo.end(Buffer.from(cuerpo));
  try {
    for await (const trozo of flujo as AsyncIterable<Uint8Array>) {
      total += trozo.byteLength;
      if (total > tope) {
        flujo.destroy();
        return falla(413, "demasiado_grande");
      }
      trozos.push(trozo);
    }
  } catch {
    return falla(400, "compresion_invalida");
  }
  return { ok: true, valor: unir(trozos, total) };
}

/** El lote como objeto JSON. Un arreglo, un número o un texto no son lotes. */
export function leerLote(bytes: Uint8Array): Resultado<Record<string, unknown>> {
  let valor: unknown;
  try {
    valor = JSON.parse(new TextDecoder().decode(bytes));
  } catch {
    return falla(400, "json_invalido");
  }
  if (!valor || typeof valor !== "object" || Array.isArray(valor)) return falla(400, "json_invalido");
  return { ok: true, valor: valor as Record<string, unknown> };
}

/** Códigos de Postgres que significan "este lote no tiene arreglo". */
const LOTE_MAL_FORMADO = new Set([
  "22023", // invalid_parameter_value: lo levanta registrar_telemetria
  "23514", // check_violation: versión que no es semver, operación no canónica…
  "23502", // not_null_violation: falta un campo obligatorio de la sesión
  "22P02", // invalid_text_representation: un uuid o un número que no lo es
  "22003", // numeric_value_out_of_range
  "22007", // invalid_datetime_format
  "22008", // datetime_field_overflow
]);

/**
 * Traduce lo que respondió `registrar_telemetria` (vía PostgREST) a lo que
 * entiende el equipo (`telemetria::envio::Respuesta`): 2xx llegó; 400 se
 * descarta; 401/403 renovar el token; 429 y 5xx reintentar más tarde.
 */
export function traducirRespuestaRpc(
  estadoHttp: number,
  cuerpo: unknown,
): { estado: number; cuerpo: Record<string, unknown> } {
  if (estadoHttp >= 200 && estadoHttp < 300 && cuerpo && typeof cuerpo === "object") {
    return { estado: 200, cuerpo: cuerpo as Record<string, unknown> };
  }
  const codigo = cuerpo && typeof cuerpo === "object" ? String((cuerpo as { code?: unknown }).code ?? "") : "";
  const mensaje = cuerpo && typeof cuerpo === "object" ? String((cuerpo as { message?: unknown }).message ?? "") : "";
  if (LOTE_MAL_FORMADO.has(codigo)) {
    return { estado: 400, cuerpo: { error: "lote_mal_formado", codigo, detalle: mensaje.slice(0, 200) } };
  }
  if (codigo === "42501") return { estado: 403, cuerpo: { error: "equipo_no_autorizado" } };
  if (codigo === "53400") return { estado: 429, cuerpo: { error: "demasiados_lotes" } };
  if (codigo.startsWith("PGRST3") || estadoHttp === 401) return { estado: 401, cuerpo: { error: "token_invalido" } };
  // La función todavía no existe en esta base (migración sin aplicar): que
  // el equipo guarde el lote y reintente, no que lo descarte.
  if (codigo === "42883" || codigo === "PGRST202" || estadoHttp === 404) {
    return { estado: 503, cuerpo: { error: "receptora_no_lista" } };
  }
  return { estado: 502, cuerpo: { error: "base_no_disponible", codigo } };
}
