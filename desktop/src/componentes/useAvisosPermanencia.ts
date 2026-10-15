import { useEffect, useState } from "react";
import { avisosPermanencia } from "../api";
import type { AvisosPermanencia } from "../api";

/** Cada cuánto se vuelve a preguntar al núcleo: alguien puede cruzar un
 * umbral con el tiempo sin que ocurra nada más en la app. */
const CADA_MS = 60_000;

/**
 * Cuánta gente lleva demasiado adentro, según el núcleo
 * (`domain::permanencia`): lo pregunta al abrir, cada vez que cambia
 * `recargar` (un ingreso o una salida, una sincronización) y cada minuto. La
 * interfaz no calcula nada: guarda lo que el núcleo contesta. Un error al
 * preguntar no se muestra (es un dato informativo): se queda lo último.
 */
export function useAvisosPermanencia(recargar: unknown): AvisosPermanencia | null {
  const [avisos, setAvisos] = useState<AvisosPermanencia | null>(null);

  useEffect(() => {
    let vigente = true;
    const preguntar = () => {
      avisosPermanencia()
        .then((respuesta) => {
          if (vigente) setAvisos(respuesta);
        })
        .catch(() => {});
    };
    preguntar();
    const reloj = setInterval(preguntar, CADA_MS);
    return () => {
      vigente = false;
      clearInterval(reloj);
    };
  }, [recargar]);

  return avisos;
}
