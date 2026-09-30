import { useEffect, useRef } from "react";
import { Check } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import type { Seccion } from "../App";

interface FilaSeccion {
  id: Seccion;
  etiqueta: string;
  Icono: LucideIcon;
}

/**
 * Menú contextual del sidebar (click derecho, mismo comportamiento que la
 * barra de actividad de VS Code): checklist de TODAS las secciones
 * (incluidas las ocultas, para poder volver a mostrarlas) que no se cierra
 * al tocar un ítem -- permite alternar varias antes de cerrar. Sólo se
 * cierra con click afuera o Escape. Copiado de
 * `desktop/src/componentes/MenuContextualSecciones.tsx`.
 */
export default function MenuContextualSecciones({
  posicion,
  secciones,
  ocultas,
  onCambiarVisibilidad,
  onRestablecer,
  onCerrar,
}: {
  posicion: { x: number; y: number };
  /** Todas las secciones, en el orden actual -- incluye las ocultas. */
  secciones: FilaSeccion[];
  ocultas: Seccion[];
  onCambiarVisibilidad: (id: Seccion, visible: boolean) => void;
  onRestablecer: () => void;
  onCerrar: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function alClicFuera(evento: MouseEvent) {
      if (ref.current && !ref.current.contains(evento.target as Node)) onCerrar();
    }
    function alTeclear(evento: KeyboardEvent) {
      if (evento.key === "Escape") onCerrar();
    }
    // `mousedown` (no `click`) para cerrar antes de que un click posterior
    // en otra parte de la app dispare su propio handler.
    document.addEventListener("mousedown", alClicFuera);
    document.addEventListener("keydown", alTeclear);
    return () => {
      document.removeEventListener("mousedown", alClicFuera);
      document.removeEventListener("keydown", alTeclear);
    };
  }, [onCerrar]);

  const visiblesCount = secciones.length - ocultas.length;

  return (
    <div
      ref={ref}
      role="menu"
      className="menu-contextual"
      // Sólo la posición es dinámica (donde se hizo el click derecho).
      style={{ top: posicion.y, left: posicion.x }}
    >
      {secciones.map((seccion) => {
        const visible = !ocultas.includes(seccion.id);
        const bloqueado = visible && visiblesCount <= 1;
        return (
          <button
            key={seccion.id}
            type="button"
            role="menuitemcheckbox"
            aria-checked={visible}
            disabled={bloqueado}
            title={bloqueado ? "No se puede ocultar la última sección visible" : undefined}
            onClick={() => onCambiarVisibilidad(seccion.id, !visible)}
            className="menu-contextual-item"
          >
            <span className="inline-flex w-4 justify-center">
              {visible && <Check size={14} aria-hidden="true" />}
            </span>
            {seccion.etiqueta}
          </button>
        );
      })}

      <div className="menu-contextual-separador" />

      <button
        type="button"
        onClick={() => {
          onRestablecer();
          onCerrar();
        }}
        className="menu-contextual-item menu-contextual-item-tenue"
      >
        Restablecer orden y visibilidad
      </button>
    </div>
  );
}
