// Utilidades HTTP comunes a todas las Edge Functions de este proyecto.
//
// CORS abierto a propósito: el panel web y la web de visitas llaman desde el
// navegador, y ninguna función depende de credenciales ambiente (cookies):
// toda autorización viaja explícita en el header `Authorization` o en el
// body, así que un origen abierto no expone nada.

export const CORS_HEADERS = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Headers": "authorization, x-client-info, apikey, content-type",
  "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
};

export function json(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json", ...CORS_HEADERS },
  });
}

/** Respuesta al preflight de CORS, o `null` si no es un preflight. */
export function preflight(req: Request): Response | null {
  return req.method === "OPTIONS" ? new Response(null, { status: 204, headers: CORS_HEADERS }) : null;
}

/** El body como objeto JSON, o `null` si no es JSON o no es un objeto. */
export async function leerCuerpo(req: Request): Promise<Record<string, unknown> | null> {
  try {
    const cuerpo = await req.json();
    return cuerpo && typeof cuerpo === "object" && !Array.isArray(cuerpo) ? cuerpo : null;
  } catch {
    return null;
  }
}

/**
 * IP del cliente según el proxy de Supabase. La observa el servidor, no la
 * declara el cliente: es el dato más confiable para auditoría.
 */
export function ipDelCliente(req: Request): string | null {
  return req.headers.get("x-forwarded-for")?.split(",")[0]?.trim() || null;
}

/** Texto recortado, o `null` si no es un texto con contenido. */
export function textoOpcional(valor: unknown, largoMaximo = 200): string | null {
  if (typeof valor !== "string") return null;
  const limpio = valor.trim();
  return limpio ? limpio.slice(0, largoMaximo) : null;
}

/**
 * Ejecuta `tarea` después de responder, sin que el runtime la corte al
 * terminar la petición (a diferencia de una promesa suelta).
 */
export function enSegundoPlano(tarea: Promise<unknown>): void {
  const runtime = (globalThis as { EdgeRuntime?: { waitUntil(p: Promise<unknown>): void } }).EdgeRuntime;
  const protegida = tarea.catch((error) => console.error("tarea en segundo plano falló:", error));
  if (runtime) runtime.waitUntil(protegida);
}
