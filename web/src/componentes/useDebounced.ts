import { useEffect, useState } from "react";

/**
 * Devuelve `valor`, pero actualizado recién `esperaMs` después de la
 * última vez que cambió -- para no recalcular algo caro (ej. el
 * `quickFilterText` de AG Grid sobre un dataset grande, ver `Tabla.tsx`)
 * en cada tecla mientras la persona todavía está escribiendo.
 */
export function useDebounced<T>(valor: T, esperaMs: number): T {
  const [debounced, setDebounced] = useState(valor);

  useEffect(() => {
    const temporizador = setTimeout(() => setDebounced(valor), esperaMs);
    return () => clearTimeout(temporizador);
  }, [valor, esperaMs]);

  return debounced;
}
