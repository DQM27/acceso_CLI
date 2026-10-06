/**
 * Todo nombre de persona o empresa va en mayúscula: regla del núcleo
 * (`reglas/src/nombre.rs`), que la aplica al guardar. El escritorio sólo
 * llega al núcleo por comandos asíncronos, así que mientras se escribe
 * repite la misma conversión acá, en un solo lugar, para que lo que se ve
 * sea lo que va a quedar. Si alguna vez difieren, manda el núcleo.
 */
export function nombreMientrasSeEscribe(texto: string): string {
  return texto.toUpperCase();
}

/**
 * Para campos no controlados (react-hook-form): convierte el valor del
 * campo conservando el cursor (reasignar `value` lo mandaría al final si se
 * corrige a mitad del texto). `limpiar` corre antes, p. ej. sólo letras.
 */
export function escribirNombreEnCampo(campo: HTMLInputElement, limpiar: (texto: string) => string = (t) => t): void {
  const { selectionStart, selectionEnd } = campo;
  const antes = campo.value;
  const despues = nombreMientrasSeEscribe(limpiar(antes));
  if (despues === antes) return;
  // Si `limpiar` quitó caracteres, el cursor retrocede lo mismo.
  const corrimiento = antes.length - despues.length;
  campo.value = despues;
  if (selectionStart !== null && selectionEnd !== null) {
    campo.setSelectionRange(Math.max(0, selectionStart - corrimiento), Math.max(0, selectionEnd - corrimiento));
  }
}
