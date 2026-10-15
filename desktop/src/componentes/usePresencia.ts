import { useEffect, useState } from "react";

/**
 * Mantiene montado un elemento el rato justo que dura su animación de salida:
 * con `activo` en `false` sigue montado (`saliendo`) `duracionMs` y luego se
 * desmonta. Quien lo usa pinta el elemento mientras `montado` sea `true` y le
 * pone la clase de salida cuando `saliendo` lo sea.
 */
export function usePresencia(
  activo: boolean,
  duracionMs: number,
): { montado: boolean; saliendo: boolean } {
  const [montado, setMontado] = useState(activo);
  const [activoPrevio, setActivoPrevio] = useState(activo);

  // Estado derivado de una prop: se ajusta durante el render (patrón de React),
  // no en un efecto, para que el elemento aparezca en el mismo cuadro.
  if (activo !== activoPrevio) {
    setActivoPrevio(activo);
    if (activo) setMontado(true);
  }

  useEffect(() => {
    if (activo) return;
    const espera = setTimeout(() => setMontado(false), duracionMs);
    return () => clearTimeout(espera);
  }, [activo, duracionMs]);

  return { montado, saliendo: montado && !activo };
}
