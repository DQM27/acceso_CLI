/**
 * FullCalendar (@fullcalendar/core) inyecta, entre su CSS, un `@font-face`
 * con la fuente de íconos (prev/next del calendario) como `data:` URI --
 * el navegador la evalúa apenas la regla se inserta en la hoja de estilos,
 * sin importar si algún elemento llega a usar esa fuente (a diferencia de
 * lo que uno esperaría de una carga "perezosa"). La CSP estricta de esta
 * app (`font-src 'self'`, sin `data:`) la bloquea igual, generando una
 * violación real aunque el ícono en sí se reemplace por texto plano vía
 * CSS (ver `.selector-fechas .fc-icon` en `index.css`).
 *
 * Se filtra específicamente esa regla ANTES de que `@fullcalendar/core`
 * corra su propia inyección -- por eso este módulo se importa primero (sin
 * nada más antes) en `componentes/SelectorFechas.tsx`: los imports ESM se
 * evalúan en orden, y este no tiene dependencias propias, así que el parche
 * ya está puesto cuando el import de FullCalendar empieza a evaluarse.
 */
const insertarOriginal = CSSStyleSheet.prototype.insertRule;
CSSStyleSheet.prototype.insertRule = function (
  this: CSSStyleSheet,
  regla: string,
  indice?: number,
): number {
  // Sustituye por una regla inocua en vez de omitirla -- el llamador
  // (`appendStylesTo` en @fullcalendar/core) calcula el índice de cada
  // inserción como `ruleCnt + i` asumiendo que TODAS las reglas anteriores
  // ya se insertaron; devolver sin insertar nada corre esa cuenta y la
  // siguiente llamada pide un índice mayor al tamaño real de la hoja
  // (`IndexSizeError`). Insertar un no-op mantiene el conteo alineado.
  if (regla.includes("@font-face") && regla.includes("fcicons")) {
    return insertarOriginal.call(this, ":root{}", indice);
  }
  return insertarOriginal.call(this, regla, indice);
};
