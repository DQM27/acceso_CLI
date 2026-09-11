import { supabase } from "../lib/supabase";

/** `data as T` (sin validar) hacía que un cambio de contrato del lado de
 * la Edge Function (backend y frontend viven en el mismo repo, pero se
 * despliegan por separado -- un deploy de función sin el del panel es
 * perfectamente posible) fallara silenciosamente más abajo, en cualquier
 * `.algo` sobre `undefined`, lejos de esta función y sin ningún mensaje
 * claro. `esperado` es un chequeo mínimo de forma (no un schema completo
 * tipo zod -- no vale la pena esa dependencia nueva para esto), sólo lo
 * suficiente para fallar acá, con un mensaje que diga qué función y qué
 * se esperaba. */
export async function invocar<T>(
  nombre: string,
  esperado: (data: unknown) => data is T,
  body?: Record<string, unknown>,
): Promise<T> {
  const { data, error } = await supabase.functions.invoke<T>(nombre, { body });
  if (error) {
    let detalle: string | undefined;
    const contexto = (error as { context?: Response }).context;
    if (contexto instanceof Response) {
      try {
        const cuerpo = await contexto.clone().json();
        detalle = cuerpo?.detail ?? cuerpo?.error;
      } catch {
        // Sin cuerpo JSON legible -- se usa error.message más abajo.
      }
    }
    throw new Error(detalle ?? error.message);
  }
  if (!esperado(data)) {
    throw new Error(`${nombre} devolvió una respuesta con forma inesperada.`);
  }
  return data;
}

export function esObjeto(valor: unknown): valor is Record<string, unknown> {
  return typeof valor === "object" && valor !== null;
}

/** Las Edge Functions de acción que no devuelven body -- cualquier
 * respuesta sin error ya vale como "anduvo". */
export function loQueSea(_valor: unknown): _valor is void {
  return true;
}
