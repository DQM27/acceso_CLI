// Lo que corre adentro del Web Worker. `trabajar` se copia tal cual (toString)
// a un Worker armado en memoria junto con `crearAyudas` (ver runner.js): no puede
// usar nada de afuera de su propio cuerpo.
//
// El código del asistente es el cuerpo de una función que recibe
// (datos, columnas, h, ancho, tema) y devuelve el dibujo como texto SVG o HTML.

export function trabajar(crearAyudas) {
  // Defensa extra (la política de seguridad ya prohíbe la red): nada para salir.
  for (const nombre of ["fetch", "XMLHttpRequest", "WebSocket", "EventSource", "importScripts", "indexedDB", "caches"]) {
    try {
      Object.defineProperty(self, nombre, { value: undefined, configurable: false, writable: false });
    } catch {
      // Si no se puede redefinir, igual la bloquea la política de seguridad.
    }
  }
  const LARGO_MAXIMO = 2_000_000;

  self.onmessage = (evento) => {
    const { codigo, datos, columnas, ancho, tema } = evento.data ?? {};
    try {
      const dibujar = new Function("datos", "columnas", "h", "ancho", "tema", `"use strict";\n${codigo}`);
      const resultado = dibujar(datos, columnas, crearAyudas(tema ?? {}), ancho, tema ?? {});
      if (typeof resultado !== "string" || !resultado.trim()) {
        throw new Error("El código no devolvió un dibujo (tiene que terminar con return y un texto SVG o HTML).");
      }
      if (resultado.length > LARGO_MAXIMO) throw new Error("El dibujo es demasiado grande.");
      self.postMessage({ ok: true, html: resultado });
    } catch (error) {
      self.postMessage({ ok: false, mensaje: error instanceof Error ? error.message : String(error) });
    }
  };
}

/** El código fuente del Worker: las ayudas y `trabajar`, en un solo texto. */
export function fuenteDelTrabajador(crearAyudas) {
  return `"use strict";\nconst crearAyudas = ${crearAyudas.toString()};\n(${trabajar.toString()})(crearAyudas);\n`;
}
