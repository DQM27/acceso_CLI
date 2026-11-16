/** Palabras de un nombre, sin tildes ni mayúsculas, para comparar. */
function palabras(nombre: string): string[] {
  return nombre
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toUpperCase()
    .replace(/[^A-Z\s]/g, " ")
    .split(/\s+/)
    .filter(Boolean);
}

/**
 * Si el nombre de la página del QR corresponde al contratista: todas las
 * palabras del nombre más corto están en el otro (puede sobrar un segundo
 * nombre o apellido), con al menos dos en común. Mismo criterio que el
 * escritorio (`DetalleContratista.logica.ts`).
 */
export function nombresCoinciden(enOrigen: string | null | undefined, contratista: string): boolean {
  if (!enOrigen) return false;
  const a = palabras(enOrigen);
  const b = palabras(contratista);
  const [corto, largo] = a.length <= b.length ? [a, b] : [b, a];
  const enLargo = new Set(largo);
  const comunes = corto.filter((p) => enLargo.has(p)).length;
  return comunes === corto.length && comunes >= 2;
}

export const MAXIMO_ARCHIVO = 10 * 1024 * 1024;

/** Valida el archivo elegido antes de mandarlo; `null` si está bien. */
export function problemaArchivo(archivo: { size: number; type: string }): string | null {
  if (archivo.size > MAXIMO_ARCHIVO) return "La imagen pasa de 10 MB.";
  if (archivo.type && !["image/png", "image/jpeg"].includes(archivo.type))
    return "El archivo debe ser una imagen PNG o JPEG.";
  return null;
}
