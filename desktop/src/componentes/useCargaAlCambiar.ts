import { useEffect } from "react";
import { toast } from "sonner";

/**
 * Corre `recargar(estaVigente)` cada vez que cambia su identidad
 * (típicamente porque cambió alguna dependencia interna, ej. un filtro de
 * búsqueda) y avisa por toast si falla. `estaVigente()` le permite a
 * `recargar` descartar su propio resultado si para cuando llega ya se
 * disparó una recarga más nueva (o la pantalla se desmontó) — sin esto, una
 * respuesta vieja que tarda más que una nueva puede llegar después y pisar
 * datos ya actualizados con datos obsoletos. Estaba duplicado igual en
 * Contratistas/Empresas/Usuarios; `recargar` sigue siendo dueño de cada
 * pantalla (su propio `useCallback`), acá sólo vive el envoltorio que era
 * idéntico en las tres.
 */
export function useCargaAlCambiar(recargar: (estaVigente: () => boolean) => Promise<unknown>) {
  useEffect(() => {
    let vigente = true;
    recargar(() => vigente).catch((error) => vigente && toast.error(String(error)));
    return () => {
      vigente = false;
    };
  }, [recargar]);
}
