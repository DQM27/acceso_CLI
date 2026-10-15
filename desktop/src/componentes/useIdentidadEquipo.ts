import { useEffect, useState } from "react";
import { obtenerIdentidadEquipo } from "../api/nube";
import type { IdentidadEquipo } from "../api/nube";

/** Unidad y etiqueta con que el panel registró esta PC. Se vuelve a leer cada
 * vez que cambia `recargar` (por ejemplo, al terminar una sincronización, que
 * es cuando puede llegar un token con datos nuevos). Un error al leer no se
 * muestra: es un dato informativo, nunca bloquea nada. */
export function useIdentidadEquipo(recargar?: unknown): IdentidadEquipo | null {
  const [identidad, setIdentidad] = useState<IdentidadEquipo | null>(null);
  useEffect(() => {
    let vigente = true;
    obtenerIdentidadEquipo()
      .then((leida) => {
        if (vigente) setIdentidad(leida);
      })
      .catch(() => {});
    return () => {
      vigente = false;
    };
  }, [recargar]);
  return identidad;
}
