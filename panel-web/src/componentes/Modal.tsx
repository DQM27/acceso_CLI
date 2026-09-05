import { useEffect } from "react";
import type { ReactNode } from "react";

/**
 * Modal genérico — backdrop + tarjeta centrada. No sabe nada de formularios
 * ni de ningún dominio en particular: cualquier pantalla lo usa para
 * cualquier contenido. Copiado de `desktop/src/componentes/Modal.tsx`.
 *
 * El backdrop NO cierra al hacer click a propósito — un click fuera de
 * lugar (frecuente al operar rápido, buscando o llenando un formulario) no
 * debe descartar en silencio lo que ya se escribió. Cerrar es explícito:
 * la X o Esc.
 */
export default function Modal({
  titulo,
  onCerrar,
  children,
}: {
  titulo: string;
  onCerrar: () => void;
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
        alignItems: "center",
        justifyContent: "center",
        zIndex: 100,
      }}
    >
      <div
        className="tarjeta"
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
