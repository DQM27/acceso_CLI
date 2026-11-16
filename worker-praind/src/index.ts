// Worker de carnés PRAIND. Ver docs/features-futuras/plan-fotos-praind.md.
//
// | Ruta                          | Quién                 | Qué hace                                  |
// |-------------------------------|-----------------------|-------------------------------------------|
// | GET    /foto/<clave>          | puede_ver             | Entrega el original desde R2              |
// | POST   /cargar-desde-qr       | puede_cargar          | Baja el carné del enlace del QR y lo guarda |
// | PUT    /cargar/<contratista>  | puede_cargar          | Recibe la imagen directamente             |
// | DELETE /foto/<contratista>    | administrador (RPC)   | Quita el carné                            |
//
// Todas piden `Authorization: Bearer <JWT de Supabase>` (equipo o panel).
// Los registros nunca llevan tokens, nombres ni cédulas.

import { ErrorHttp, ErrorRpc } from "./errores";
import { extension, leerConLimite, MAXIMO_IMAGEN, tipoPorFirma } from "./imagen";
import { descargarCarne, validarEnlace } from "./origen";
import { hashHex, permisos, rpc } from "./supabase";
import type { Dependencias, Env, TipoImagen } from "./tipos";

const UUID = "[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}";
const RE_CLAVE = new RegExp(`^contratistas/${UUID}/[0-9a-f]{64}\\.(png|jpg)$`);
const RE_UUID = new RegExp(`^${UUID}$`);
const MAXIMO_JSON = 4 * 1024;
const MAXIMO_TEXTO = 300;

export default {
  fetch(request: Request, env: Env): Promise<Response> {
    return manejar(request, env, { fetch: (entrada, init) => fetch(entrada, init) });
  },
} satisfies ExportedHandler<Env>;

export async function manejar(request: Request, env: Env, deps: Dependencias): Promise<Response> {
  const origen = request.headers.get("Origin");
  const origenPermitido = origen && origenes(env).includes(origen) ? origen : null;

  if (request.method === "OPTIONS") {
    if (!origenPermitido) return new Response(null, { status: 403 });
    return conCors(new Response(null, { status: 204 }), origenPermitido, true);
  }

  let respuesta: Response;
  try {
    respuesta = await enrutar(request, env, deps);
  } catch (error) {
    respuesta = respuestaDeError(error);
  }
  const url = new URL(request.url);
  console.log(`${request.method} ${url.pathname.split("/")[1] ?? ""} -> ${respuesta.status}`);
  return origenPermitido ? conCors(respuesta, origenPermitido, false) : respuesta;
}

function origenes(env: Env): string[] {
  return env.ORIGENES_PERMITIDOS.split(",").map((o) => o.trim()).filter(Boolean);
}

function conCors(respuesta: Response, origen: string, preflight: boolean): Response {
  const conCabeceras = new Response(respuesta.body, respuesta);
  conCabeceras.headers.set("Access-Control-Allow-Origin", origen);
  conCabeceras.headers.append("Vary", "Origin");
  conCabeceras.headers.set("Access-Control-Expose-Headers", "ETag");
  if (preflight) {
    conCabeceras.headers.set("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE");
    conCabeceras.headers.set("Access-Control-Allow-Headers", "Authorization, Content-Type, If-None-Match");
    conCabeceras.headers.set("Access-Control-Max-Age", "600");
  }
  return conCabeceras;
}

function json(cuerpo: unknown, estado = 200): Response {
  return new Response(JSON.stringify(cuerpo), {
    status: estado,
    headers: { "Content-Type": "application/json; charset=utf-8", "Cache-Control": "no-store" },
  });
}

function respuestaDeError(error: unknown): Response {
  const http =
    error instanceof ErrorHttp
      ? error
      : error instanceof ErrorRpc
        ? error.aHttp()
        : null;
  if (http) return json({ error: http.codigo, detail: http.mensaje }, http.estado);
  console.error("error inesperado:", error instanceof Error ? error.name : typeof error);
  return json({ error: "error_interno", detail: "Error inesperado. Intentá de nuevo." }, 500);
}

function tokenDe(request: Request): string {
  const token = (request.headers.get("Authorization") ?? "").replace(/^Bearer\s+/i, "").trim();
  if (!token) throw new ErrorHttp(401, "sesion_invalida", "Falta la sesión.");
  return token;
}

const noEncontrado = () => new ErrorHttp(404, "no_existe", "No existe.");

async function enrutar(request: Request, env: Env, deps: Dependencias): Promise<Response> {
  const { pathname, searchParams } = new URL(request.url);
  const partes = pathname.split("/").filter(Boolean);
  const metodo = request.method;

  if (partes[0] === "foto" && metodo === "GET") {
    const clave = partes.slice(1).join("/");
    if (!RE_CLAVE.test(clave)) throw noEncontrado();
    return servirFoto(request, env, deps, clave);
  }
  if (partes[0] === "foto" && metodo === "DELETE" && partes.length === 2 && RE_UUID.test(partes[1])) {
    return quitar(env, deps, tokenDe(request), partes[1]);
  }
  if (pathname === "/cargar-desde-qr" && metodo === "POST") {
    return cargarDesdeQr(request, env, deps);
  }
  if (partes[0] === "cargar" && metodo === "PUT" && partes.length === 2 && RE_UUID.test(partes[1])) {
    return cargarArchivo(request, env, deps, partes[1], searchParams);
  }
  throw noEncontrado();
}

async function servirFoto(request: Request, env: Env, deps: Dependencias, clave: string): Promise<Response> {
  const token = tokenDe(request);
  if (!(await permisos(env, deps, token, { usarCache: true })).puede_ver) {
    throw new ErrorHttp(403, "sin_permiso", "No tiene permiso para ver gafetes PRAIND.");
  }
  const sha = clave.split("/")[2].split(".")[0];
  const etag = `"${sha}"`;
  const cabeceras = {
    "Cache-Control": "private, max-age=31536000, immutable",
    ETag: etag,
    "X-Content-Type-Options": "nosniff",
  };
  if (request.headers.get("If-None-Match") === etag) {
    return new Response(null, { status: 304, headers: cabeceras });
  }
  const objeto = await env.FOTOS.get(clave);
  if (!objeto) throw noEncontrado();
  return new Response(objeto.body, {
    headers: {
      ...cabeceras,
      "Content-Type": clave.endsWith(".png") ? "image/png" : "image/jpeg",
      "Content-Length": String(objeto.size),
    },
  });
}

async function exigirCarga(env: Env, deps: Dependencias, token: string): Promise<void> {
  if (!(await permisos(env, deps, token, { usarCache: false })).puede_cargar) {
    throw new ErrorHttp(403, "sin_permiso", "No tiene permiso para cargar gafetes PRAIND.");
  }
}

function textoOpcional(valor: unknown): string | null {
  if (typeof valor !== "string") return null;
  const limpio = valor.trim();
  return limpio ? limpio.slice(0, MAXIMO_TEXTO) : null;
}

async function cargarDesdeQr(request: Request, env: Env, deps: Dependencias): Promise<Response> {
  const token = tokenDe(request);
  const texto = new TextDecoder().decode(
    await leerConLimite(request.body, MAXIMO_JSON, new ErrorHttp(413, "pedido_grande", "Pedido demasiado grande.")),
  );
  let cuerpo: { contratista_id?: unknown; enlace?: unknown; solo_previsualizar?: unknown };
  try {
    cuerpo = JSON.parse(texto);
  } catch {
    throw new ErrorHttp(400, "dato_invalido", "El pedido no es JSON.");
  }
  const contratistaId = typeof cuerpo.contratista_id === "string" ? cuerpo.contratista_id.toLowerCase() : "";
  if (!RE_UUID.test(contratistaId)) throw new ErrorHttp(400, "dato_invalido", "Falta el contratista.");
  const enlace = validarEnlace(cuerpo.enlace);

  await exigirCarga(env, deps, token);
  const carne = await descargarCarne(deps, enlace);
  const resumen = { nombre_en_origen: carne.nombre, bytes: carne.bytes.byteLength, tipo_mime: carne.tipo };
  if (cuerpo.solo_previsualizar === true) return json(resumen);

  const clave = await guardar(env, deps, token, contratistaId, carne.bytes, carne.tipo, enlace.toString(), carne.nombre);
  return json({ clave, ...resumen });
}

async function cargarArchivo(
  request: Request,
  env: Env,
  deps: Dependencias,
  contratistaId: string,
  parametros: URLSearchParams,
): Promise<Response> {
  const token = tokenDe(request);
  const largo = Number(request.headers.get("Content-Length") ?? "0");
  const demasiadoGrande = new ErrorHttp(413, "imagen_grande", "La imagen del gafete PRAIND pasa de 10 MB.");
  if (largo > MAXIMO_IMAGEN) throw demasiadoGrande;

  // Antes de leer el cuerpo: sin permiso no se recibe el archivo.
  await exigirCarga(env, deps, token);
  const bytes = await leerConLimite(request.body, MAXIMO_IMAGEN, demasiadoGrande);
  const tipo = tipoPorFirma(bytes);
  if (!tipo) throw new ErrorHttp(415, "no_es_imagen", "El archivo debe ser una imagen PNG o JPEG.");

  const clave = await guardar(
    env,
    deps,
    token,
    contratistaId,
    bytes,
    tipo,
    textoOpcional(parametros.get("enlace_origen")),
    textoOpcional(parametros.get("nombre_en_origen")),
  );
  return json({ clave, bytes: bytes.byteLength, tipo_mime: tipo });
}

/**
 * Escribe en R2 y registra en Supabase. Si el registro falla, el objeto
 * recién escrito se borra para no dejar huérfanos. Si había un carné
 * anterior, se borra después de registrar el nuevo.
 */
async function guardar(
  env: Env,
  deps: Dependencias,
  token: string,
  contratistaId: string,
  bytes: Uint8Array,
  tipo: TipoImagen,
  enlace: string | null,
  nombre: string | null,
): Promise<string> {
  const sha = await hashHex(bytes);
  const clave = `contratistas/${contratistaId}/${sha}.${extension(tipo)}`;
  const existia = (await env.FOTOS.head(clave)) !== null;
  if (!existia) {
    await env.FOTOS.put(clave, bytes, { httpMetadata: { contentType: tipo }, sha256: sha });
  }

  let anterior: string | null;
  try {
    anterior = await rpc<string | null>(env, deps, token, "praind_registrar", {
      p_contratista_id: contratistaId,
      p_clave: clave,
      p_sha256: sha,
      p_bytes: bytes.byteLength,
      p_tipo_mime: tipo,
      p_enlace_origen: enlace,
      p_nombre_en_origen: nombre,
    });
  } catch (error) {
    if (!existia) await env.FOTOS.delete(clave);
    throw error;
  }

  if (anterior && anterior !== clave && RE_CLAVE.test(anterior)) {
    await env.FOTOS.delete(anterior);
  }
  return clave;
}

async function quitar(env: Env, deps: Dependencias, token: string, contratistaId: string): Promise<Response> {
  const quitada = await rpc<string | null>(env, deps, token, "praind_quitar", { p_contratista_id: contratistaId });
  if (quitada && RE_CLAVE.test(quitada)) await env.FOTOS.delete(quitada);
  return json({ quitada: quitada !== null });
}
