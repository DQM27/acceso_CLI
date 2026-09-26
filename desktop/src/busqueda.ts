/** Utilidades de los buscadores de los modales de registro (KOF,
 * proveedores): filtrar un catálogo ya cargado en memoria sin distinguir
 * tildes ni mayúsculas, y validar el número de gafete. */

/** Minúsculas y sin tildes, para que "jose" encuentre a "JOSÉ". */
export function plegar(texto: string): string {
  return texto
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase();
}

/** `true` si `pajar` contiene todas las palabras de `texto` (plegadas).
 * Texto vacío nunca coincide: un buscador vacío no muestra resultados. */
export function coincideBusqueda(texto: string, pajar: string): boolean {
  const partes = plegar(texto).split(/\s+/).filter(Boolean);
  if (partes.length === 0) return false;
  const pajarPlegado = plegar(pajar);
  return partes.every((parte) => pajarPlegado.includes(parte));
}

/** Número de gafete obligatorio, entero y mayor a cero. */
export function validarNumeroGafete(
  texto: string,
): { valido: true; numero: number } | { valido: false; mensaje: string } {
  const recortado = texto.trim();
  if (!recortado) return { valido: false, mensaje: "El número de gafete es obligatorio" };
  const numero = Number.parseInt(recortado, 10);
  if (Number.isNaN(numero) || numero <= 0) {
    return { valido: false, mensaje: "Ingrese un número de gafete válido" };
  }
  return { valido: true, numero };
}

/** "#07" -- número de gafete con dos dígitos, como en las grillas. */
export function textoGafete(numero: number): string {
  return `#${String(numero).padStart(2, "0")}`;
}
