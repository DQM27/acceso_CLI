import { ErrorRpc } from "./errores";
import type { Dependencias, Env, Permisos } from "./tipos";

/** Llama una RPC con el JWT de quien hizo el pedido: Postgres decide. */
export async function rpc<T>(
  env: Env,
  deps: Dependencias,
  token: string,
  nombre: string,
  argumentos: Record<string, unknown> = {},
): Promise<T> {
  const respuesta = await deps.fetch(`${env.SUPABASE_URL}/rest/v1/rpc/${nombre}`, {
    method: "POST",
    headers: {
      apikey: env.SUPABASE_PUBLISHABLE_KEY,
      Authorization: `Bearer ${token}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify(argumentos),
  });
  if (!respuesta.ok) {
    let cuerpo: { code?: string; message?: string } = {};
    try {
      cuerpo = await respuesta.json();
    } catch {
      // Sin cuerpo JSON: queda el estado HTTP.
    }
    throw new ErrorRpc(respuesta.status, cuerpo.code, cuerpo.message ?? "Error de la nube.");
  }
  return (await respuesta.json()) as T;
}

// Caché corta de permisos para ver: una imagen por pedido no debe costar un
// viaje a Supabase cada vez. Clave: hash del token (nunca el token).
const VIGENCIA_MS = 60_000;
const MAXIMO = 500;
const cache = new Map<string, { permisos: Permisos; hasta: number }>();

export async function hashHex(datos: string | ArrayBuffer | Uint8Array): Promise<string> {
  const bytes = typeof datos === "string" ? new TextEncoder().encode(datos) : datos;
  const resumen = await crypto.subtle.digest("SHA-256", bytes);
  return [...new Uint8Array(resumen)].map((b) => b.toString(16).padStart(2, "0")).join("");
}

export async function permisos(
  env: Env,
  deps: Dependencias,
  token: string,
  { usarCache }: { usarCache: boolean },
): Promise<Permisos> {
  const clave = await hashHex(token);
  const ahora = Date.now();
  if (usarCache) {
    const guardado = cache.get(clave);
    if (guardado && guardado.hasta > ahora) return guardado.permisos;
  }
  const resultado = await rpc<Permisos>(env, deps, token, "praind_permisos");
  if (cache.size >= MAXIMO) {
    for (const [k, v] of cache) if (v.hasta <= ahora) cache.delete(k);
    if (cache.size >= MAXIMO) cache.clear();
  }
  cache.set(clave, { permisos: resultado, hasta: ahora + VIGENCIA_MS });
  return resultado;
}

/** Sólo para pruebas. */
export function vaciarCachePermisos(): void {
  cache.clear();
}
