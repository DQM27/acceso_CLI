import type { ResumenDiario } from "../../api/analisis";

/** Orden de la semana en la tabla dinámica (si no, la ordena alfabéticamente). */
export const DIAS_SEMANA = ["Lunes", "Martes", "Miércoles", "Jueves", "Viernes", "Sábado", "Domingo"] as const;

/** Una fila del resumen diario como la lee la tabla dinámica. */
export type FilaDinamica = {
  /** "2026-03-02T00:00:00": sin zona, así Syncfusion lo toma como ese día a
   * medianoche local al agrupar por año, mes y día (con "Z" o sólo la fecha
   * lo interpretaría en UTC y en Costa Rica caería el día anterior). */
  dia: string;
  dia_semana: string;
  unidad: string;
  tipo_persona: string;
  tipo_ingreso: string;
  medio: string;
  ingresos: number;
  con_salida: number;
  minutos_adentro: number;
  horas_adentro: number;
  /** Relleno: el valor real lo calcula `permanenciaDeCelda` por celda. */
  permanencia: number;
};

export function filaDinamica(fila: ResumenDiario, textoTipoPersona: string): FilaDinamica {
  const [anio, mes, dia] = fila.dia.split("-").map(Number);
  // getUTCDay: 0 = domingo; DIAS_SEMANA empieza en lunes.
  const diaSemana = (new Date(Date.UTC(anio, mes - 1, dia)).getUTCDay() + 6) % 7;
  return {
    dia: `${fila.dia}T00:00:00`,
    dia_semana: DIAS_SEMANA[diaSemana],
    unidad: fila.unidad,
    tipo_persona: textoTipoPersona,
    tipo_ingreso: fila.tipo_ingreso,
    medio: fila.medio,
    ingresos: fila.ingresos,
    con_salida: fila.con_salida,
    minutos_adentro: fila.minutos_adentro,
    horas_adentro: Math.round(fila.minutos_adentro / 60),
    permanencia: 0,
  };
}

/** Permanencia promedio (minutos) de las filas de una celda, ponderada por
 * cuántos salieron: promediar los promedios de cada día sesgaría hacia los
 * días con pocas salidas. `undefined` (celda vacía) si nadie salió. */
export function permanenciaDeCelda(filas: Pick<FilaDinamica, "minutos_adentro" | "con_salida">[]): number | undefined {
  let minutos = 0;
  let salidas = 0;
  for (const fila of filas) {
    minutos += fila.minutos_adentro;
    salidas += fila.con_salida;
  }
  return salidas > 0 ? Math.round(minutos / salidas) : undefined;
}

/** El tooltip de los gráficos admite marcado (`<b>`, `<br/>`); los nombres de
 * empresa los escriben los operadores, así que se escapan antes de mezclarlos
 * con él. */
export function escaparHtml(texto: string): string {
  return texto.replace(/[&<>"']/g, (c) => `&#${c.charCodeAt(0)};`);
}
