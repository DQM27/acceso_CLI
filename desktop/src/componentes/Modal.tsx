import { useEffect } from "react";
import type { ReactNode } from "react";

/**
 * Modal genérico — backdrop + tarjeta centrada. No sabe nada de formularios
 * ni de ningún dominio en particular: cualquier pantalla lo usa para
 * cualquier contenido (formulario de alta/edición hoy, confirmaciones u
 * otros diálogos después).
 *
 * El backdrop NO cierra al hacer click a propósito — un click fuera de
 * lugar (frecuente al operar rápido, buscando o llenando un formulario) no
 * debe descartar en silencio lo que ya se escribió. Cerrar es explícito:
 * la X o Esc.
 */
export default function Modal({
  titulo,
  onCerrar,
  anclarArriba = false,
  children,
}: {
  titulo: string;
  onCerrar: () => void;
  /** Fija el borde superior en vez de centrar -- para modales que crecen
   * al elegir algo (buscador + ficha que se despliega debajo). Centrado, al
   * crecer se movía para arriba y para abajo a la vez, y se veía a los
   * saltos (reportado 2026-09-24); anclado sólo crece hacia abajo. */
  anclarArriba?: boolean;
  children: ReactNode;
}) {
  useEffect(() => {
    function alTeclear(evento: KeyboardEvent) {
      if (evento.key === "Escape") onCerrar();
    }
    window.addEventListener("keydown", alTeclear);
    return () => window.removeEventListener("keydown", alTeclear);
  }, [onCerrar]);

  return (
    <div
      style={{
        position: "fixed",
        inset: 0,
        background: "var(--velo)",
        display: "flex",
        alignItems: anclarArriba ? "flex-start" : "center",
        justifyContent: "center",
        // Anclado a media altura menos ~13rem: vacío (sólo el buscador)
        // queda justo encima del centro, y con la ficha desplegada el
        // modal completo queda centrado. `max` para ventanas bajas.
        paddingTop: anclarArriba ? "max(4vh, calc(50vh - 13rem))" : undefined,
        zIndex: 100,
      }}
    >
      <div
        // `modal`: títulos, etiquetas y botones en mayúsculas en TODOS los
        // modales (ver `.modal` en index.css).
        className="tarjeta modal"
        style={{
          background: "var(--elevado)",
          boxShadow: "var(--sombra-panel)",
          width: "32rem",
          maxWidth: "calc(100% - 2rem)",
          maxHeight: "calc(100% - 2rem)",
          overflowY: "auto",
          padding: "1.5rem",
        }}
      >
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            marginBottom: "1rem",
          }}
        >
          <h2 style={{ margin: 0, fontSize: "1.1rem", color: "var(--acento)" }}>{titulo}</h2>
          <button type="button" className="boton" onClick={onCerrar}>
            ✕
          </button>
        </div>
        {children}
      </div>
    </div>
  );
}
