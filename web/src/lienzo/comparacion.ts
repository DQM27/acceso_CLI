import { formatoNumero } from "./graficos/utiles";

/** Comparación de una tarjeta KPI contra otro período. */
export interface Comparacion {
  /** Valor del período anterior. */
  anterior: number;
  /** Contra qué se compara: "vs. las 2 semanas anteriores" (o, en diferencia, "que el jueves pasado a esta hora"). */
  contra: string;
  /** Lo que se compara, si no es el valor que muestra la tarjeta (por ejemplo, sin el día de hoy a medias). */
  actual?: number;
  /** "diferencia": "5 menos que…", en cantidades; para números chicos dice más que un porcentaje. */
  modo?: "porcentaje" | "diferencia";
}

const PORCENTAJE = new Intl.NumberFormat("es-CR", { style: "percent", maximumFractionDigits: 0, signDisplay: "exceptZero" });

/** El texto de la comparación: porcentaje ("+12 % vs. … (40)") o diferencia ("5 menos que … (12)"). */
export function textoComparacion(actual: number, c: Comparacion): string {
  if (c.modo === "diferencia") {
    const d = actual - c.anterior;
    const cuanto = d === 0 ? "Igual" : `${formatoNumero(Math.abs(d))} ${d > 0 ? "más" : "menos"}`;
    return `${cuanto} ${c.contra} (${formatoNumero(c.anterior)})`;
  }
  if (c.anterior === 0) return `${formatoNumero(c.anterior)} ${c.contra}`;
  return `${PORCENTAJE.format((actual - c.anterior) / c.anterior)} ${c.contra} (${formatoNumero(c.anterior)})`;
}
