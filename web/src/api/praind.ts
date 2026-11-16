import { z } from "../lib/validacion";
import { supabase } from "../lib/supabase";

/**
 * Carnés PRAIND de contratistas (docs/features-futuras/plan-fotos-praind.md,
 * fase 4). Las imágenes viven en R2 y las sirve el Worker `praind-brisas`;
 * el panel le habla con la sesión del administrador, y los permisos los
 * decide la nube (`praind_permisos`). La ruta del carné vigente está en
 * `contratistas.praind_foto`.
 */

const URL_STAGING = "pmrytjktlyiuikxuuxpr";

/** Dirección del Worker: `VITE_PRAIND_URL`, o la que corresponde al
 * proyecto de Supabase del build (staging o producción). */
export function urlWorker(
  explicita: string | undefined = import.meta.env.VITE_PRAIND_URL,
  supabaseUrl: string = import.meta.env.VITE_SUPABASE_URL ?? "",
): string {
  if (explicita) return explicita.replace(/\/+$/, "");
  return supabaseUrl.includes(URL_STAGING) ? "https://praind-staging.megabrisas.com" : "https://praind.megabrisas.com";
}

async function token(): Promise<string> {
  const { data } = await supabase.auth.getSession();
  const actual = data.session?.access_token;
  if (!actual) throw new Error("La sesión venció. Volvé a entrar.");
  return actual;
}

/** Pedido al Worker; un rechazo trae el texto para mostrar en `detail`. */
async function pedir(ruta: string, init: RequestInit = {}): Promise<Response> {
  let respuesta: Response;
  try {
    respuesta = await fetch(`${urlWorker()}${ruta}`, {
      ...init,
      headers: { ...init.headers, Authorization: `Bearer ${await token()}` },
    });
  } catch {
    throw new Error("No se pudo conectar con el servicio de carnés. Revisá la conexión.");
  }
  if (!respuesta.ok) {
    let detalle: string | undefined;
    try {
      detalle = ((await respuesta.json()) as { detail?: string }).detail;
    } catch {
      // Sin cuerpo JSON: queda el texto genérico.
    }
    throw new Error(detalle ?? `El servicio de carnés respondió ${respuesta.status}.`);
  }
  return respuesta;
}

/** Ruta del carné vigente del contratista, o `null` si no tiene. */
export async function claveCarne(contratistaId: string): Promise<string | null> {
  const { data, error } = await supabase
    .from("contratistas")
    .select("praind_foto")
    .eq("id", contratistaId)
    .maybeSingle();
  if (error) throw new Error(error.message);
  return z.object({ praind_foto: z.string().nullable() }).nullable().parse(data)?.praind_foto ?? null;
}

/** La imagen del carné, lista para un `<img>` (quien llama libera la URL). */
export async function urlImagenCarne(clave: string): Promise<string> {
  const respuesta = await pedir(`/foto/${clave}`);
  return URL.createObjectURL(await respuesta.blob());
}

const cargaEsquema = z.object({
  clave: z.string().nullable().optional(),
  nombre_en_origen: z.string().nullable().optional(),
  bytes: z.number(),
  tipo_mime: z.string(),
});
export type CargaCarne = z.infer<typeof cargaEsquema>;

/** Carga desde el enlace del QR. Con `soloPrevisualizar` no guarda nada:
 * devuelve el nombre de la página para compararlo antes. */
export async function cargarCarneDesdeQr(
  contratistaId: string,
  enlace: string,
  soloPrevisualizar: boolean,
): Promise<CargaCarne> {
  const respuesta = await pedir("/cargar-desde-qr", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      contratista_id: contratistaId,
      enlace: enlace.trim(),
      solo_previsualizar: soloPrevisualizar,
    }),
  });
  return cargaEsquema.parse(await respuesta.json());
}

/** Carga la imagen (PNG o JPEG) directamente. */
export async function cargarCarneArchivo(contratistaId: string, archivo: Blob): Promise<CargaCarne> {
  const respuesta = await pedir(`/cargar/${contratistaId}`, {
    method: "PUT",
    headers: { "Content-Type": "application/octet-stream" },
    body: archivo,
  });
  return cargaEsquema.parse(await respuesta.json());
}

export async function quitarCarne(contratistaId: string): Promise<void> {
  await pedir(`/foto/${contratistaId}`, { method: "DELETE" });
}

/** Si los equipos (escritorio y móvil) pueden cargar carnés además del panel. */
export async function cargaDesdeEquiposPermitida(): Promise<boolean> {
  const { data, error } = await supabase.from("ajustes_praind").select("carga_desde_equipos").maybeSingle();
  if (error) throw new Error(error.message);
  return z.object({ carga_desde_equipos: z.boolean() }).nullable().parse(data)?.carga_desde_equipos ?? false;
}

export async function cambiarCargaDesdeEquipos(permitir: boolean): Promise<void> {
  const { error } = await supabase.rpc("praind_cambiar_carga_desde_equipos", { p_permitir: permitir });
  if (error) throw new Error(error.message);
}
