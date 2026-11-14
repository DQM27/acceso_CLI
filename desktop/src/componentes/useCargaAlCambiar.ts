import { useEffect, useState } from "react";
import { avisos } from "../avisos/avisos";
import { useSeccionActiva } from "../contexto/BarraEstadoContexto";
import { EVENTO_CAMBIO_EN_VIVO, EVENTO_CAMBIO_LOCAL_NUBE, EVENTO_NUBE_ACTUALIZADA } from "../eventosNube";
import { useRecargaVisible } from "./useRecargaVisible";

/**
 * Corre `recargar(estaVigente)` cada vez que cambia su identidad
 * (típicamente porque cambió alguna dependencia interna, ej. un filtro de
 * búsqueda) y avisa si falla. `estaVigente()` le permite a
 * `recargar` descartar su propio resultado si para cuando llega ya se
 * disparó una recarga más nueva (o la pantalla se desmontó) — sin esto, una
 * respuesta vieja que tarda más que una nueva puede llegar después y pisar
 * datos ya actualizados con datos obsoletos. Estaba duplicado igual en
 * Contratistas/Empresas/Usuarios; `recargar` sigue siendo dueño de cada
 * pantalla (su propio `useCallback`), acá sólo vive el envoltorio que era
 * idéntico en las tres.
 *
 * Con `escucharNube`, también recarga cuando llega una sincronización, un
 * aviso en vivo o un registro hecho en esta PC desde otra parte (ej. el
 * modal de ingreso con Ctrl+N) -- pero sólo con la pantalla a la vista;
 * oculta, queda pendiente y recarga una vez al volver (`useRecargaVisible`).
 */
export function useCargaAlCambiar(recargar: (estaVigente: () => boolean) => Promise<unknown>, escucharNube = false) {
  const activa = useSeccionActiva();
  const versionVisible = useRecargaVisible();
  // Cambios que llegan con la pantalla a la vista.
  const [cambiosVistos, setCambiosVistos] = useState(0);

  useEffect(() => {
    if (!escucharNube || !activa) return;
    const alCambiar = () => setCambiosVistos((n) => n + 1);
    const eventos = [EVENTO_NUBE_ACTUALIZADA, EVENTO_CAMBIO_EN_VIVO, EVENTO_CAMBIO_LOCAL_NUBE];
    for (const evento of eventos) window.addEventListener(evento, alCambiar);
    return () => {
      for (const evento of eventos) window.removeEventListener(evento, alCambiar);
    };
  }, [activa, escucharNube]);

  // Sin `escucharNube` la versión visible no participa: esa pantalla no
  // quiere enterarse de cambios externos.
  const versionExterna = escucharNube ? versionVisible : 0;

  useEffect(() => {
    let vigente = true;
    const estaVigente = () => vigente;
    recargar(estaVigente).catch((error) => estaVigente() && avisos.error(error));
    return () => {
      vigente = false;
    };
  }, [recargar, cambiosVistos, versionExterna]);
}
