import { useEffect, useEffectEvent, useRef, useState } from "react";
import { avisosPermanencia } from "../api";
import type { AvisosPermanencia } from "../api";

/** Cada cuánto se vuelve a preguntar al núcleo: alguien puede cruzar un
 * umbral con el tiempo sin que ocurra nada más en la app. */
const CADA_MS = 60_000;

/** ¿Cambió quién está en aviso o crítico? Compara los números y los textos
 * (no el objeto: cada respuesta es uno nuevo aunque diga lo mismo). */
export function cambiaronNiveles(antes: AvisosPermanencia, ahora: AvisosPermanencia): boolean {
  return (
    antes.en_aviso !== ahora.en_aviso ||
    antes.criticos !== ahora.criticos ||
    antes.texto_aviso !== ahora.texto_aviso ||
    antes.texto_critico !== ahora.texto_critico
  );
}

/**
 * Cuánta gente lleva demasiado adentro, según el núcleo
 * (`domain::permanencia`): lo pregunta al abrir, cada vez que cambia
 * `recargar` (un ingreso o una salida, una sincronización) y cada minuto. La
 * interfaz no calcula nada: guarda lo que el núcleo contesta. Un error al
 * preguntar no se muestra (es un dato informativo): se queda lo último.
 *
 * `alCambiarNiveles` se llama cuando la respuesta difiere de la anterior --
 * típicamente porque alguien cruzó un umbral con el reloj. El color de las
 * filas de Activos sale de los datos de la grilla, que sólo cambian al
 * recargar: sin este aviso, el contador decía "1 crítico" y la fila seguía
 * sin resaltar hasta la siguiente sincronización (auditoría 2026-11-13).
 */
export function useAvisosPermanencia(
  recargar: unknown,
  alCambiarNiveles?: () => void,
): AvisosPermanencia | null {
  const [avisos, setAvisos] = useState<AvisosPermanencia | null>(null);
  // Última respuesta, para comparar sin depender del estado: una respuesta
  // igual a la anterior no re-renderiza (antes, App se redibujaba cada
  // minuto aunque nada cambiara).
  const ultima = useRef<AvisosPermanencia | null>(null);
  const avisarCambio = useEffectEvent(() => alCambiarNiveles?.());

  useEffect(() => {
    let vigente = true;
    // `porReloj`: sólo un cambio que encontró la pregunta de cada minuto
    // avisa. Si cambió por una recarga (una salida, una sincronización), los
    // datos de la grilla ya son nuevos: avisar recargaría dos veces.
    const preguntar = (porReloj: boolean) => {
      avisosPermanencia()
        .then((respuesta) => {
          if (!vigente) return;
          const anterior = ultima.current;
          if (anterior !== null && !cambiaronNiveles(anterior, respuesta)) return;
          ultima.current = respuesta;
          setAvisos(respuesta);
          if (porReloj && anterior !== null) avisarCambio();
        })
        .catch(() => {});
    };
    preguntar(false);
    const reloj = setInterval(() => preguntar(true), CADA_MS);
    return () => {
      vigente = false;
      clearInterval(reloj);
    };
  }, [recargar]);

  return avisos;
}
