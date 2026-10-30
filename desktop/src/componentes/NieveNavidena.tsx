import { useEffect, useRef } from "react";
import {
  cantidadCopos,
  crearCopos,
  esFondoOscuro,
  posicionCopo,
} from "./NieveNavidena.logica";

// Navidad prendida o apagada: nieve en la app, y con ella el icono y el
// logo navideños. No se toca a mano: lo cambia `scripts/icono_navideno.py`
// (`poner` en el parche de fines de noviembre, `quitar` en el de
// principios de enero), sin fechas en el código (pedido del usuario
// 2026-10-29: un parche que se pone y se quita, nada más).
export const NAVIDAD_ACTIVA = false;

// Nieve muy sutil que cae DETRÁS de la barra lateral y la grilla (pedido
// del usuario 2026-10-29): un `canvas` fijo con `z-index: -1`, encima del
// fondo del `body` y debajo de todo lo demás, así se ve sólo en los
// espacios libres entre las tarjetas, que son opacas. No recibe clics.
//
// Con "reducir movimiento" del sistema no se dibuja. `requestAnimationFrame`
// se detiene solo con la ventana minimizada.
export default function NieveNavidena() {
  const lienzo = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = lienzo.current;
    if (!NAVIDAD_ACTIVA || !canvas) return;
    if (window.matchMedia?.("(prefers-reduced-motion: reduce)").matches) return;
    const contexto = canvas.getContext("2d");
    if (!contexto) return;

    let copos = crearCopos(
      cantidadCopos(window.innerWidth, window.innerHeight),
    );
    let oscuro = true;
    let cuadro = 0;
    const inicio = performance.now();

    const ajustarTamano = () => {
      const escala = window.devicePixelRatio || 1;
      canvas.width = Math.round(window.innerWidth * escala);
      canvas.height = Math.round(window.innerHeight * escala);
      contexto.setTransform(escala, 0, 0, escala, 0, 0);
      copos = crearCopos(cantidadCopos(window.innerWidth, window.innerHeight));
    };
    ajustarTamano();
    window.addEventListener("resize", ajustarTamano);

    let pedido = 0;
    const dibujar = (ahora: number) => {
      // El tema se revisa una vez por segundo, no en cada cuadro: leer el
      // estilo calculado fuerza a recalcular estilos.
      if (cuadro++ % 60 === 0)
        oscuro = esFondoOscuro(getComputedStyle(document.body).backgroundColor);
      const ancho = window.innerWidth;
      const alto = window.innerHeight;
      const segundos = (ahora - inicio) / 1000;
      contexto.clearRect(0, 0, ancho, alto);
      contexto.fillStyle = oscuro ? "#ffffff" : "#8fa3bf";
      const alfaTema = oscuro ? 0.5 : 0.6;
      for (const copo of copos) {
        const { x, y } = posicionCopo(copo, segundos, ancho, alto);
        contexto.globalAlpha = copo.alfa * alfaTema;
        contexto.beginPath();
        contexto.arc(x, y, copo.radio, 0, 2 * Math.PI);
        contexto.fill();
      }
      pedido = requestAnimationFrame(dibujar);
    };
    pedido = requestAnimationFrame(dibujar);

    return () => {
      cancelAnimationFrame(pedido);
      window.removeEventListener("resize", ajustarTamano);
    };
  }, []);

  if (!NAVIDAD_ACTIVA) return null;
  return (
    <canvas
      ref={lienzo}
      aria-hidden="true"
      style={{
        position: "fixed",
        inset: 0,
        width: "100%",
        height: "100%",
        zIndex: -1,
        pointerEvents: "none",
      }}
    />
  );
}
