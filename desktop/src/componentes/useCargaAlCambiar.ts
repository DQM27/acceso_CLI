import { useEffect } from "react";
import { toast } from "sonner";

/**
 * Corre `recargar()` cada vez que cambia su identidad (típicamente porque
 * cambió alguna dependencia interna, ej. un filtro de búsqueda) y avisa por
 * toast si falla — con el guard de "¿la pantalla sigue montada?" para no
 * disparar un toast sobre un componente que ya se desmontó. Estaba
 * duplicado igual en Contratistas/Empresas/Usuarios; `recargar` sigue
 * siendo dueño de cada pantalla (su propio `useCallback`), acá sólo vive el
 * envoltorio que era idéntico en las tres.
 */
export function useCargaAlCambiar(recargar: () => Promise<unknown>) {
  useEffect(() => {
    let vigente = true;
    recargar().catch((error) => vigente && toast.error(String(error)));
    return () => {
      vigente = false;
    };
  }, [recargar]);
}
