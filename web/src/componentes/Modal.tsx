import { useCallback, useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";

/** Lo que dura la animación de salida (`.modal-velo-saliendo` en
 * index.css): `onCerrar` llega cuando termina. */
export const DURACION_SALIDA_MODAL_MS = 150;

/**
 * Modal genérico — backdrop + tarjeta centrada. No sabe nada de formularios
 * ni de ningún dominio en particular: cualquier pantalla lo usa para
 * cualquier contenido. Copiado de `desktop/src/componentes/Modal.tsx`.
 *
 * El backdrop NO cierra al hacer click a propósito — un click fuera de
 * lugar (frecuente al operar rápido, buscando o llenando un formulario) no
 * debe descartar en silencio lo que ya se escribió. Cerrar es explícito:
 * la X o Esc.
 *
 * Entrada y salida con zoom (pedido del usuario, 2026-11-13, igual que en el
 * escritorio): al abrir, la tarjeta crece desde un poco más chica con un
 * fundido; al cerrar con la X o Esc, se encoge y se desvanece, y recién
 * entonces avisa `onCerrar`. Si la pantalla no lo desmonta (varias no dejan
 * cerrar mientras guardan: `onCerrar={() => !enviando && ...}`), el modal
 * vuelve a verse. Lo que cierra la pantalla por su cuenta (ej. al guardar)
 * lo desmonta sin animación de salida.
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
  // `onCerrar` puede cambiar en cada render de la pantalla: se lee el último
  // al terminar la salida.
  const onCerrarRef = useRef(onCerrar);
  useEffect(() => {
    onCerrarRef.current = onCerrar;
  }, [onCerrar]);

  const [saliendo, setSaliendo] = useState(false);
  const saliendoRef = useRef(false);
  const esperaSalida = useRef<ReturnType<typeof setTimeout> | null>(null);

  const cerrar = useCallback(() => {
    // Una sola salida: Esc repetido o X + Esc no avisan dos veces.
    if (saliendoRef.current) return;
    saliendoRef.current = true;
    setSaliendo(true);
    esperaSalida.current = setTimeout(() => {
      onCerrarRef.current();
      // Si la pantalla no lo desmontó, vuelve a verse en vez de quedar
      // invisible y sin recibir clics.
      saliendoRef.current = false;
      setSaliendo(false);
    }, DURACION_SALIDA_MODAL_MS);
  }, []);

  useEffect(
    () => () => {
      if (esperaSalida.current) clearTimeout(esperaSalida.current);
    },
    [],
  );

  useEffect(() => {
    function alTeclear(evento: KeyboardEvent) {
      if (evento.key === "Escape") cerrar();
    }
    window.addEventListener("keydown", alTeclear);
    return () => window.removeEventListener("keydown", alTeclear);
  }, [cerrar]);

  return (
    <div
      className={saliendo ? "modal-velo modal-velo-saliendo" : "modal-velo"}
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
        className="tarjeta modal-tarjeta"
        role="dialog"
        aria-modal="true"
        aria-label={titulo}
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
          <button type="button" className="boton" onClick={cerrar} aria-label="Cerrar" title="Cerrar">
            ✕
          </button>
        </div>
        {children}
      </div>
    </div>
  );
}
