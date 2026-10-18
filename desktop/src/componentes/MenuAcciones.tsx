import { useEffect, useRef, useState } from "react";
import { ChevronDown } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { ListaFlotante } from "./ListaFlotante";
import { useListaFlotante } from "./ListaFlotante.logica";

export interface AccionMenu {
  /** Texto de la fila: dice qué se va a registrar, no sólo "nuevo". */
  texto: string;
  Icono: LucideIcon;
  /** Atajo de teclado, si lo tiene (se ve a la derecha y en el `title`). */
  atajo?: string;
  onElegir: () => void;
}

/**
 * Un botón con texto que abre una lista corta de acciones -- para cuando una
 * misma intención ("Nuevo ingreso") tiene varios caminos y el botón no debe
 * adivinar cuál: se ven todos, con su nombre. Mismo mecanismo flotante que
 * el selector de columnas de `Tabla.tsx` (`ListaFlotante`).
 */
export default function MenuAcciones({
  texto,
  Icono,
  acciones,
}: {
  texto: string;
  Icono: LucideIcon;
  acciones: AccionMenu[];
}) {
  const [abierto, setAbierto] = useState(false);
  const { campoRef, posicion } = useListaFlotante(abierto);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!abierto) return;
    function alHacerClicAfuera(evento: MouseEvent) {
      const objetivo = evento.target as Node;
      if (campoRef.current?.contains(objetivo) || menuRef.current?.contains(objetivo)) return;
      setAbierto(false);
    }
    function alTeclear(evento: KeyboardEvent) {
      if (evento.key === "Escape") setAbierto(false);
    }
    document.addEventListener("mousedown", alHacerClicAfuera);
    document.addEventListener("keydown", alTeclear);
    return () => {
      document.removeEventListener("mousedown", alHacerClicAfuera);
      document.removeEventListener("keydown", alTeclear);
    };
  }, [abierto, campoRef]);

  return (
    <div ref={campoRef}>
      <button
        type="button"
        className="boton boton-icono"
        aria-haspopup="menu"
        aria-expanded={abierto}
        onClick={() => setAbierto((actual) => !actual)}
      >
        <Icono size={16} aria-hidden="true" />
        {texto}
        <ChevronDown size={14} aria-hidden="true" />
      </button>
      {abierto && posicion && (
        <ListaFlotante posicion={posicion} ancho={220}>
          <div ref={menuRef} role="menu" style={{ display: "flex", flexDirection: "column", gap: "0.25rem" }}>
            {acciones.map(({ texto: textoAccion, Icono: IconoAccion, atajo, onElegir }) => (
              <button
                key={textoAccion}
                type="button"
                role="menuitem"
                className="boton boton-icono boton-salir"
                style={{ justifyContent: "flex-start" }}
                title={atajo ? `Atajo: ${atajo}` : undefined}
                onClick={() => {
                  setAbierto(false);
                  onElegir();
                }}
              >
                <IconoAccion size={16} aria-hidden="true" />
                <span style={{ flex: 1, textAlign: "left" }}>{textoAccion}</span>
                {atajo && <span style={{ opacity: 0.6, fontSize: "0.75rem" }}>{atajo}</span>}
              </button>
            ))}
          </div>
        </ListaFlotante>
      )}
    </div>
  );
}
