import { formatoNumero } from "./graficos/utiles";

/** Comparación de una tarjeta KPI contra otro período. */
export interface Comparacion {
  /** Valor del período anterior. */
  anterior: number;
  /** Contra qué se compara, empezando por "que": "que las 2 semanas anteriores", "que el jueves pasado a esta hora". */
  contra: string;
  /** Lo que se compara, si no es el valor que muestra la tarjeta (por ejemplo, sin el día de hoy a medias). */
  actual?: number;
  /** "diferencia": "5 menos que…", en cantidades; para números chicos dice más que un porcentaje. */
  modo?: "porcentaje" | "diferencia";
}

const PORCENTAJE = new Intl.NumberFormat("es-CR", { style: "percent", maximumFractionDigits: 0 });

/**
 * El texto de la comparación, siempre en palabras y sin signos: "12% más que …
 * (40)", "5 menos que … (12)" o "Igual que … (12)". Sin base (anterior 0) no hay
 * porcentaje: se dice la diferencia en cantidades.
 */
export function textoComparacion(actual: number, c: Comparacion): string {
  const d = actual - c.anterior;
  const sentido = d > 0 ? "más" : "menos";
  let cuanto: string;
  if (d === 0) cuanto = "Igual";
  else if (c.modo === "diferencia" || c.anterior === 0) cuanto = `${formatoNumero(Math.abs(d))} ${sentido}`;
  else cuanto = `${PORCENTAJE.format(Math.abs(d) / c.anterior)} ${sentido}`;
  return `${cuanto} ${c.contra} (${formatoNumero(c.anterior)})`;
}
