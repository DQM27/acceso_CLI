import { ErrorHttp } from "./errores";
import type { TipoImagen } from "./tipos";

export const MAXIMO_IMAGEN = 10 * 1024 * 1024;

/** El tipo se decide por los primeros bytes, nunca por la cabecera. */
export function tipoPorFirma(bytes: Uint8Array): TipoImagen | null {
  const png = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
  if (bytes.length >= png.length && png.every((b, i) => bytes[i] === b)) return "image/png";
  if (bytes.length >= 3 && bytes[0] === 0xff && bytes[1] === 0xd8 && bytes[2] === 0xff) return "image/jpeg";
  return null;
}

export function extension(tipo: TipoImagen): "png" | "jpg" {
  return tipo === "image/png" ? "png" : "jpg";
}

/** Lee un cuerpo entero sin pasar de `limite` bytes. */
export async function leerConLimite(
  cuerpo: ReadableStream<Uint8Array> | null,
  limite: number,
  errorSiExcede: ErrorHttp,
): Promise<Uint8Array> {
  if (!cuerpo) return new Uint8Array();
  const partes: Uint8Array[] = [];
  let total = 0;
  const lector = cuerpo.getReader();
  for (;;) {
    const { done, value } = await lector.read();
    if (done) break;
    total += value.byteLength;
    if (total > limite) {
      await lector.cancel();
      throw errorSiExcede;
    }
    partes.push(value);
  }
  const todo = new Uint8Array(total);
  let posicion = 0;
  for (const parte of partes) {
    todo.set(parte, posicion);
    posicion += parte.byteLength;
  }
  return todo;
}
