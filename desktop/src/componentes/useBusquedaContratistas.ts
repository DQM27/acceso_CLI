import { useEffect, useEffectEvent, useState } from "react";
import { buscarContratistas } from "../api";
import type { ContratistaResumen } from "../api";

/** Espera desde la última tecla antes de consultar. */
export const ESPERA_BUSQUEDA_MS = 120;
/** Cuántos contratistas muestra la lista del buscador. */
export const MAX_RESULTADOS_BUSQUEDA = 4;

interface Busqueda {
  /** Contratistas encontrados. Mientras llega la respuesta de lo nuevo que
   * se tecleó, se quedan los anteriores (no parpadea a vacío). */
  resultados: ContratistaResumen[];
  /** Hay una búsqueda en curso para el texto actual. */
  buscando: boolean;
}

/**
 * Buscador de contratistas con espera (Nuevo ingreso, Gestión de gafete).
 *
 * Estaba copiado igual en los dos modales, con el mismo error: la espera se
 * cancelaba al teclear, pero una consulta ya enviada no -- si la respuesta
 * de "car" llegaba después de la de "carlos", la lista mostraba lo de "car"
 * (auditoría 2026-11-13). Acá cada respuesta trae el texto que la pidió y
 * sólo se muestra si sigue siendo el actual. `buscando` se deriva de eso, sin
 * setState síncrono dentro del efecto.
 *
 * `activa` en `false` (o texto vacío) vacía la lista sin consultar.
 * `alFallar` recibe el error para mostrarlo donde el modal ya lo muestra.
 */
export function useBusquedaContratistas(
  texto: string,
  activa: boolean,
  alFallar: (error: string) => void,
): Busqueda {
  const consulta = activa && texto.trim() ? texto : "";
  const [respuesta, setRespuesta] = useState<{ texto: string; filas: ContratistaResumen[] }>({
    texto: "",
    filas: [],
  });
  const avisarFallo = useEffectEvent(alFallar);

  useEffect(() => {
    if (!consulta) return;
    let vigente = true;
    const espera = window.setTimeout(() => {
      buscarContratistas({ texto: consulta })
        .then((pagina) => {
          if (vigente) setRespuesta({ texto: consulta, filas: pagina.items.slice(0, MAX_RESULTADOS_BUSQUEDA) });
        })
        .catch((error) => {
          if (!vigente) return;
          // Se da por respondida (sin filas) para que deje de "buscar".
          setRespuesta({ texto: consulta, filas: [] });
          avisarFallo(String(error));
        });
    }, ESPERA_BUSQUEDA_MS);
    return () => {
      vigente = false;
      window.clearTimeout(espera);
    };
  }, [consulta]);

  if (!consulta) return { resultados: [], buscando: false };
  return { resultados: respuesta.filas, buscando: respuesta.texto !== consulta };
}
