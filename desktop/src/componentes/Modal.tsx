import type { ReactNode } from "react";

/**
 * Modal genérico — backdrop + tarjeta centrada. No sabe nada de formularios
 * ni de ningún dominio en particular: cualquier pantalla lo usa para
 * cualquier contenido (formulario de alta/edición hoy, confirmaciones u
 * otros diálogos después).
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
  return (
    <div
      style={{
        position: "fixed",
        inset: 0,
        background: "rgba(0, 0, 0, 0.55)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 100,
      }}
      onClick={onCerrar}
    >
      <div
        className="tarjeta"
        onClick={(evento) => evento.stopPropagation()}
        style={{
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
