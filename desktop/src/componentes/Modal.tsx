import { useEffect, useLayoutEffect, useRef, useState } from "react";
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
 *
 * Cambio de alto animado (reportado 2026-09-24): al elegir a alguien en un
 * buscador (Nuevo ingreso, Salida, KOF, proveedores) se despliega una ficha
 * debajo y el modal crece. Centrado, ese crecimiento era de golpe y se veía
 * a los saltos; ahora el cuerpo mide su contenido (`ResizeObserver`) y el
 * alto pasa de un valor al otro con una transición, así el modal se estira
 * suave sin dejar de estar centrado.
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

  const contenidoRef = useRef<HTMLDivElement>(null);
  // `null` hasta la primera medida: al abrir, el modal aparece con su alto
  // natural (sin animar desde 0); de ahí en más cada cambio se anima.
  const [alto, setAlto] = useState<number | null>(null);
  useLayoutEffect(() => {
    const contenido = contenidoRef.current;
    // Sin ResizeObserver (jsdom en los tests) el alto queda natural, sin
    // animación.
    if (!contenido || typeof ResizeObserver === "undefined") return;
    const observador = new ResizeObserver(() => setAlto(contenido.offsetHeight));
    observador.observe(contenido);
    return () => observador.disconnect();
  }, []);

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
        <div className="modal-cuerpo" style={{ height: alto ?? undefined }}>
          <div ref={contenidoRef}>{children}</div>
        </div>
      </div>
    </div>
  );
}
