import { assert, assertEquals } from "jsr:@std/assert@1";
import { Buffer } from "node:buffer";
import { brotliCompressSync, gzipSync } from "node:zlib";
import {
  descomprimir,
  leerConTope,
  leerLote,
  MAX_COMPRIMIDO,
  MAX_DESCOMPRIMIDO,
  traducirRespuestaRpc,
} from "./logica.ts";

const LOTE = {
  lote_id: "eeeeeeee-0000-0000-0000-000000000001",
  esquema: 3,
  base_ms: 1793470000000,
  sesion: { id: "dddddddd-0000-0000-0000-000000000001" },
  bloques: [{ tipo: "reloj", cols: ["confiable"], filas: [[0, true], [1500, false]] }],
};
const TEXTO = new TextEncoder().encode(JSON.stringify(LOTE));

function flujoDe(trozos: Uint8Array[]): ReadableStream<Uint8Array> {
  return new ReadableStream({
    start(controlador) {
      for (const trozo of trozos) controlador.enqueue(trozo);
      controlador.close();
    },
  });
}

Deno.test("descomprime brotli, gzip y sin codificar al mismo lote", async () => {
  for (const [codificacion, cuerpo] of [
    ["br", new Uint8Array(brotliCompressSync(TEXTO))],
    ["gzip", new Uint8Array(gzipSync(TEXTO))],
    [null, TEXTO],
    ["identity", TEXTO],
  ] as const) {
    const resultado = await descomprimir(cuerpo, codificacion);
    assert(resultado.ok, `falló ${codificacion}`);
    const lote = leerLote(resultado.valor);
    assert(lote.ok);
    assertEquals(lote.valor, LOTE);
  }
});

Deno.test("una bomba de compresión se corta en el tope", async () => {
  // 8 MB de ceros comprimen a unos pocos KB.
  const bomba = new Uint8Array(brotliCompressSync(Buffer.alloc(8 * 1024 * 1024)));
  assert(bomba.byteLength < MAX_COMPRIMIDO);
  const resultado = await descomprimir(bomba, "br");
  assertEquals(resultado, { ok: false, estado: 413, error: "demasiado_grande" });
});

Deno.test("justo en el tope pasa; un byte más no", async () => {
  const justo = new Uint8Array(brotliCompressSync(Buffer.alloc(MAX_DESCOMPRIMIDO, 0x20)));
  assert((await descomprimir(justo, "br")).ok);
  const pasado = new Uint8Array(brotliCompressSync(Buffer.alloc(MAX_DESCOMPRIMIDO + 1, 0x20)));
  assertEquals((await descomprimir(pasado, "br")).ok, false);
});

Deno.test("brotli dañado y codificación desconocida", async () => {
  assertEquals(await descomprimir(new TextEncoder().encode("esto no es brotli"), "br"), {
    ok: false,
    estado: 400,
    error: "compresion_invalida",
  });
  assertEquals(await descomprimir(TEXTO, "zstd"), { ok: false, estado: 415, error: "codificacion_no_soportada" });
});

Deno.test("el cuerpo se lee con tope aunque no traiga content-length", async () => {
  const trozo = new Uint8Array(64 * 1024);
  const dentro = await leerConTope(flujoDe([trozo, trozo]), MAX_COMPRIMIDO);
  assert(dentro.ok);
  assertEquals(dentro.valor.byteLength, 128 * 1024);
  const fuera = await leerConTope(flujoDe([trozo, trozo, trozo, trozo, trozo]), MAX_COMPRIMIDO);
  assertEquals(fuera, { ok: false, estado: 413, error: "demasiado_grande" });
  const vacio = await leerConTope(null);
  assert(vacio.ok);
  assertEquals(vacio.valor.byteLength, 0);
});

Deno.test("sólo un objeto JSON es un lote", () => {
  const texto = (valor: string) => new TextEncoder().encode(valor);
  assert(leerLote(texto("{}")).ok);
  for (const malo of ["[]", "1", '"lote"', "null", "{roto"]) {
    assertEquals(leerLote(texto(malo)), { ok: false, estado: 400, error: "json_invalido" }, malo);
  }
});

Deno.test("traduce la respuesta de la base a lo que entiende el equipo", () => {
  const recibido = { estado: "recibido", lote_id: LOTE.lote_id, filas: 2, descartadas_tipo: 0, descartadas_otras: 0 };
  assertEquals(traducirRespuestaRpc(200, recibido), { estado: 200, cuerpo: recibido });
  assertEquals(traducirRespuestaRpc(200, { estado: "duplicado" }).estado, 200);

  // Sin arreglo: el equipo lo descarta.
  for (const codigo of ["22023", "23514", "23502", "22P02"]) {
    assertEquals(traducirRespuestaRpc(400, { code: codigo, message: "x" }).estado, 400, codigo);
  }
  // Renovar el token o equipo no vigente.
  assertEquals(traducirRespuestaRpc(403, { code: "42501" }).estado, 403);
  assertEquals(traducirRespuestaRpc(401, { code: "PGRST301" }).estado, 401);
  assertEquals(traducirRespuestaRpc(401, null).estado, 401);
  // Reintentar más tarde.
  assertEquals(traducirRespuestaRpc(503, { code: "53400" }).estado, 429);
  assertEquals(traducirRespuestaRpc(404, { code: "PGRST202" }).estado, 503);
  assertEquals(traducirRespuestaRpc(500, { code: "XX000" }).estado, 502);
  assertEquals(traducirRespuestaRpc(500, null).estado, 502);
});

Deno.test("el detalle de un rechazo no crece sin límite", () => {
  const { cuerpo } = traducirRespuestaRpc(400, { code: "22023", message: "x".repeat(5000) });
  assertEquals(String(cuerpo.detalle).length, 200);
});
