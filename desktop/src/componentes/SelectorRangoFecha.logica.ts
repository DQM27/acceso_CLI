import { fechaHaceMeses, fechaYMD, textoFechaDDMMYYYY } from "../tiempo";

/**
 * Botón "Período: ..." que abre un popover con accesos rápidos (Hoy, Esta
 * semana, etc.) + los dos campos de fecha para un rango custom — mismo
 * patrón que ya probado en otro proyecto del usuario (Svelte). Reusa el
 * mecanismo de posicionamiento/portal de `ListaFlotante` en vez de
 * reinventarlo; el click-afuera para cerrar es lo único nuevo acá.
 *
 * Un acceso rápido aplica y cierra en el acto. El rango a mano (campos
 * Desde/Hasta) queda en un borrador local (`desdeBorrador`/`hastaBorrador`)
 * hasta "Aplicar" — tipear en los campos no dispara `onAplicar` todavía,
 * así el usuario puede ajustar las dos fechas antes de confirmar (o
 * "Cancelar" y no cambiar nada).
 */

/** Mismo texto que muestra el botón "Período: ..." — se exporta para que
 * quien necesite describir el filtro activo en otro lado (ej. el
 * encabezado del PDF exportado) no reimplemente este formateo. `desde`/
 * `hasta` vacíos son extremos abiertos (ver `rango_utc` en el backend); se
 * describe cada combinación en vez de asumir que "falta uno" significa
 * "sin filtro" (el caso normal es `desde` fijo, `hasta` abierto hasta hoy). */
export function textoRangoFecha(desde: string, hasta: string): string {
  if (desde && hasta) return `${textoFechaDDMMYYYY(desde)} – ${textoFechaDDMMYYYY(hasta)}`;
  if (desde) return `Desde ${textoFechaDDMMYYYY(desde)}`;
  if (hasta) return `Hasta ${textoFechaDDMMYYYY(hasta)}`;
  return "Todo el historial";
}

export interface Preset {
  /** Nombre completo: el título al pasar el mouse sobre el acceso rápido. */
  etiqueta: string;
  /** Nombre corto: lo que muestran el acceso rápido del panel y el botón
   * "Período" (pedido del usuario 2026-09-23: mucho texto). */
  corta: string;
  calcular: (hoy: Date) => { desde: string; hasta: string };
}

/** Lunes de la semana que contiene `d` — la semana arranca en lunes acá
 * (convención de semana laboral), no domingo. */
export function inicioSemana(d: Date): Date {
  const dia = d.getDay(); // 0 = domingo … 6 = sábado
  const offset = dia === 0 ? 6 : dia - 1;
  return new Date(d.getFullYear(), d.getMonth(), d.getDate() - offset);
}

export const PRESETS: Preset[] = [
  {
    etiqueta: "Hoy",
    corta: "Hoy",
    calcular: (hoy) => ({ desde: fechaYMD(hoy), hasta: fechaYMD(hoy) }),
  },
  {
    etiqueta: "Ayer",
    corta: "Ayer",
    calcular: (hoy) => {
      const ayer = new Date(hoy.getFullYear(), hoy.getMonth(), hoy.getDate() - 1);
      return { desde: fechaYMD(ayer), hasta: fechaYMD(ayer) };
    },
  },
  {
    etiqueta: "Esta semana",
    corta: "Semana",
    calcular: (hoy) => ({ desde: fechaYMD(inicioSemana(hoy)), hasta: fechaYMD(hoy) }),
  },
  {
    etiqueta: "Semana pasada",
    corta: "Sem. pasada",
    calcular: (hoy) => {
      const inicioActual = inicioSemana(hoy);
      const inicioPasada = new Date(
        inicioActual.getFullYear(),
        inicioActual.getMonth(),
        inicioActual.getDate() - 7,
      );
      const finPasada = new Date(
        inicioActual.getFullYear(),
        inicioActual.getMonth(),
        inicioActual.getDate() - 1,
      );
      return { desde: fechaYMD(inicioPasada), hasta: fechaYMD(finPasada) };
    },
  },
  {
    etiqueta: "Este mes",
    corta: "Mes",
    calcular: (hoy) => ({
      desde: fechaYMD(new Date(hoy.getFullYear(), hoy.getMonth(), 1)),
      hasta: fechaYMD(hoy),
    }),
  },
  {
    etiqueta: "Mes pasado",
    corta: "Mes pasado",
    calcular: (hoy) => ({
      desde: fechaYMD(new Date(hoy.getFullYear(), hoy.getMonth() - 1, 1)),
      // Día 0 del mes actual == último día del mes anterior.
      hasta: fechaYMD(new Date(hoy.getFullYear(), hoy.getMonth(), 0)),
    }),
  },
  {
    etiqueta: "Últimos 7 días",
    corta: "7 días",
    calcular: (hoy) => ({
      desde: fechaYMD(new Date(hoy.getFullYear(), hoy.getMonth(), hoy.getDate() - 6)),
      hasta: fechaYMD(hoy),
    }),
  },
  {
    etiqueta: "Últimos 30 días",
    corta: "30 días",
    calcular: (hoy) => ({
      desde: fechaYMD(new Date(hoy.getFullYear(), hoy.getMonth(), hoy.getDate() - 29)),
      hasta: fechaYMD(hoy),
    }),
  },
  // Los dos de abajo son las formas de "anular" el filtro (pedido del
  // usuario 2026-09-23: no había cómo volver atrás). "Últimos 6 meses" es
  // exactamente el período con que abre Historial (`hasta` abierto, ver
  // `fechaHaceMeses` en Historial.tsx), así que el botón lo muestra por su
  // nombre desde el arranque. "Todo el historial" deja los dos extremos
  // abiertos -- sin filtro.
  {
    etiqueta: "Últimos 6 meses",
    corta: "6 meses",
    calcular: (hoy) => ({ desde: fechaHaceMeses(6, hoy), hasta: "" }),
  },
  {
    etiqueta: "Todo el historial",
    corta: "Todo",
    calcular: () => ({ desde: "", hasta: "" }),
  },
];

/** "2026-09-23" → "23/09/26" (año corto, para el botón). */
export function fechaCorta(ymd: string): string {
  const [anio, mes, dia] = ymd.split("-");
  return `${dia}/${mes}/${anio.slice(2)}`;
}

/** Texto compacto del botón (pedido del usuario 2026-09-23: más chico y
 * más estético): el nombre corto del acceso rápido si el rango coincide con
 * uno ("Hoy", "Mes", "7 días"...), si no las fechas con año corto. El texto completo
 * sigue en `textoRangoFecha` (título del botón y encabezado del PDF). */
export function etiquetaCortaRango(desde: string, hasta: string, hoy: Date = new Date()): string {
  const preset = PRESETS.find((p) => {
    const rango = p.calcular(hoy);
    return rango.desde === desde && rango.hasta === hasta;
  });
  if (preset) return preset.corta;
  if (desde && hasta) return `${fechaCorta(desde)} – ${fechaCorta(hasta)}`;
  if (desde) return `Desde ${fechaCorta(desde)}`;
  if (hasta) return `Hasta ${fechaCorta(hasta)}`;
  return "Todo";
}
