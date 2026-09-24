import { useState } from "react";
import type { ChangeEvent } from "react";
import type { CustomDateProps } from "ag-grid-react";
import { interpretarFechaDDMMAAAA, mascaraFechaDDMMAAAA, textoFechaDDMMAAAA } from "../tiempo";

function mismoDia(a: Date | null, b: Date | null): boolean {
  if (a === null || b === null) return a === b;
  return a.getTime() === b.getTime();
}

/**
 * Caja del filtro de fecha de las grillas (`dateComponent` de AG Grid, ver
 * `FILTRO_FECHA` en `Tabla.tsx`), siempre en DD/MM/AAAA -- pedido del
 * usuario 2026-09-23. El selector que trae AG Grid es el `type="date"`
 * nativo, que muestra el formato del idioma de Windows (podía salir
 * MM/DD/AAAA). Recién avisa a la grilla con una fecha completa y válida;
 * mientras se escribe (o si se borra) le pasa `null`, que AG Grid toma como
 * "sin filtro".
 */
export default function FiltroFechaTabla({ date, onDateChange }: CustomDateProps) {
  const [texto, setTexto] = useState(() => (date ? textoFechaDDMMAAAA(date) : ""));
  const [fechaAnterior, setFechaAnterior] = useState<Date | null>(date);

  // La grilla también cambia `date` por su cuenta (ej. al borrar el filtro
  // desde el menú) -- se ajusta el texto durante el render, patrón que
  // recomienda React en vez de un `useEffect` que pise el estado.
  if (!mismoDia(date, fechaAnterior)) {
    setFechaAnterior(date);
    if (!mismoDia(date, interpretarFechaDDMMAAAA(texto))) {
      setTexto(date ? textoFechaDDMMAAAA(date) : "");
    }
  }

  function alCambiar(evento: ChangeEvent<HTMLInputElement>) {
    const siguiente = mascaraFechaDDMMAAAA(evento.target.value);
    setTexto(siguiente);
    const fecha = interpretarFechaDDMMAAAA(siguiente);
    setFechaAnterior(fecha);
    onDateChange(fecha);
  }

  return (
    <input
      className="ag-input-field-input ag-text-field-input"
      value={texto}
      onChange={alCambiar}
      placeholder="DD/MM/AAAA"
      inputMode="numeric"
      maxLength={10}
      style={{ width: "100%" }}
    />
  );
}
