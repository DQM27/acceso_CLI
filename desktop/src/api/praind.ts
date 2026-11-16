import { invoke } from "./invocar";

// Espejo de comandos/praind.rs (carnés PRAIND de contratistas).

export interface EstadoCarne {
  /** Ruta del carné vigente; `null` si el contratista no tiene. */
  clave: string | null;
  /** Ya está guardado en este equipo: se ve sin internet. */
  guardado: boolean;
  /** El contratista ya llegó a la nube (hace falta para cargar). */
  sincronizado: boolean;
}

export interface PermisosCarne {
  puede_ver: boolean;
  puede_cargar: boolean;
  es_admin: boolean;
  sin_conexion: boolean;
}

export interface CargaCarne {
  clave: string | null;
  nombre_en_origen: string | null;
  bytes: number;
  tipo_mime: string;
}

export function estadoCarnePraind(contratistaId: number): Promise<EstadoCarne> {
  return invoke("estado_carne_praind", { contratistaId });
}

export function permisosCarnePraind(): Promise<PermisosCarne> {
  return invoke("permisos_carne_praind");
}

/** Los bytes del carné (de este equipo si ya se abrió; si no, de la nube). */
export async function verCarnePraind(contratistaId: number): Promise<Uint8Array<ArrayBuffer>> {
  return comoBytes(await invoke<unknown>("ver_carne_praind", { contratistaId }));
}

/** Por el canal binario de Tauri la respuesta llega como `ArrayBuffer`; si
 * el WebView cae al canal de mensajes, llega como lista de números. Un
 * `Blob` hecho con la lista guarda el TEXTO "137,80,78,…" y la imagen sale
 * rota: por eso se normaliza acá. */
export function comoBytes(respuesta: unknown): Uint8Array<ArrayBuffer> {
  if (respuesta instanceof ArrayBuffer) return new Uint8Array(respuesta);
  if (ArrayBuffer.isView(respuesta)) {
    // Copia: el buffer de una vista puede ser compartido o más grande.
    return Uint8Array.from(new Uint8Array(respuesta.buffer, respuesta.byteOffset, respuesta.byteLength));
  }
  if (Array.isArray(respuesta)) return Uint8Array.from(respuesta as number[]);
  throw new Error("El carné llegó en un formato inesperado.");
}

export function cargarCarnePraindDesdeQr(
  contratistaId: number,
  enlace: string,
  soloPrevisualizar: boolean,
): Promise<CargaCarne> {
  return invoke("cargar_carne_praind_desde_qr", { contratistaId, enlace, soloPrevisualizar });
}

/** Sube la imagen en binario; el contratista va en una cabecera. */
export function cargarCarnePraindArchivo(contratistaId: number, bytes: Uint8Array): Promise<CargaCarne> {
  return invoke("cargar_carne_praind_archivo", bytes, {
    headers: { "x-contratista-id": String(contratistaId) },
  });
}
