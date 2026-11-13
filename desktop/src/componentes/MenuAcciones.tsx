import { useEffect, useRef } from "react";
import { ChevronDown } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { ListaFlotante } from "./ListaFlotante";
import { useAperturaAnimada, useListaFlotante } from "./ListaFlotante.logica";
import { ESTILO_BOTON_COMPACTO } from "./MenuAcciones.logica";

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
 * el selector de columnas de `Tabla.tsx` (`ListaFlotante`); entra y sale
 * cayendo desde el botón, como el menú de cuenta (`animacion="caida"`).
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
  const { abierto, visible, abrir, cerrar } = useAperturaAnimada();
  const { campoRef, posicion } = useListaFlotante(visible);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!abierto) return;
    function alHacerClicAfuera(evento: MouseEvent) {
      const objetivo = evento.target as Node;
      if (campoRef.current?.contains(objetivo) || menuRef.current?.contains(objetivo)) return;
      cerrar();
    }
    function alTeclear(evento: KeyboardEvent) {
      if (evento.key === "Escape") cerrar();
    }
    document.addEventListener("mousedown", alHacerClicAfuera);
    document.addEventListener("keydown", alTeclear);
    return () => {
      document.removeEventListener("mousedown", alHacerClicAfuera);
      document.removeEventListener("keydown", alTeclear);
    };
  }, [abierto, campoRef, cerrar]);

  return (
    <div ref={campoRef} style={{ flexShrink: 0 }}>
      <button
        type="button"
        className="boton boton-icono"
        style={ESTILO_BOTON_COMPACTO}
        aria-haspopup="menu"
        aria-expanded={abierto}
        onClick={abierto ? cerrar : abrir}
      >
        <Icono size={16} aria-hidden="true" />
        {texto}
        <ChevronDown size={14} aria-hidden="true" />
      </button>
      {visible && posicion && (
        <ListaFlotante posicion={posicion} ancho={220} animacion="caida" saliendo={!abierto}>
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
                  cerrar();
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
