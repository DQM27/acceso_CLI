import { useCallback, useState } from "react";

/** Cuántas filas muestra la grilla ahora mismo y si hay algún filtro puesto
 * (por columna o el buscador de arriba). Lo informa `Tabla` por
 * `onConteoVisible` cada vez que cambia lo que se ve. */
export interface ConteoVisible {
  visibles: number;
  filtrando: boolean;
}

/** Texto del contador de la barra de estado: "304 movimiento(s)" sin
 * filtros, "1 de 304 movimiento(s)" con alguno. Antes cada pantalla contaba
 * `filas.length` -- lo cargado, no lo que quedaba tras filtrar -- y al
 * filtrar a una persona o un gafete seguía diciendo el total (hallazgo del
 * usuario, 2026-11-13). "X de Y" en vez de sólo X: deja a la vista que hay
 * un filtro puesto y que no se perdió nada. `total` admite texto (ej.
 * "500+" en un rango truncado). */
export function textoConteo(
  conteo: ConteoVisible | null,
  total: number | string,
  unidad: string,
): string {
  if (conteo?.filtrando) return `${conteo.visibles} de ${total} ${unidad}`;
  return `${total} ${unidad}`;
}

/** Estado del conteo de una grilla: el valor, y el callback estable para
 * `onConteoVisible`. Si llega el mismo conteo (ej. sólo cambió el orden)
 * conserva el objeto anterior y la pantalla no se vuelve a dibujar. */
export function useConteoVisible(): [ConteoVisible | null, (conteo: ConteoVisible) => void] {
  const [conteo, setConteo] = useState<ConteoVisible | null>(null);
  const reportar = useCallback((nuevo: ConteoVisible) => {
    setConteo((anterior) =>
      anterior &&
      anterior.visibles === nuevo.visibles &&
      anterior.filtrando === nuevo.filtrando
        ? anterior
        : nuevo,
    );
  }, []);
  return [conteo, reportar];
}
