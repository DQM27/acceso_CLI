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
 * cierra con click afuera o Escape.
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
    // en otra parte de la app dispare su propio handler -- mismo criterio
    // que el cierre por blur en `SalidaRutaModal`/`ListaFlotante`.
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
      style={{
        position: "fixed",
        top: posicion.y,
        left: posicion.x,
        zIndex: 200,
        minWidth: "13rem",
        background: "var(--elevado)",
        border: "1px solid var(--borde)",
        borderRadius: "var(--radio-chico)",
        boxShadow: "var(--sombra-panel)",
        padding: "0.3rem",
      }}
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
            style={{
              display: "flex",
              alignItems: "center",
              gap: "0.5rem",
              width: "100%",
              padding: "0.35rem 0.6rem",
              background: "none",
              border: "none",
              borderRadius: "var(--radio-chico)",
              color: "var(--texto)",
              fontSize: "0.85rem",
              textAlign: "left",
              cursor: bloqueado ? "not-allowed" : "pointer",
              opacity: bloqueado ? 0.5 : 1,
            }}
            onMouseEnter={(evento) => {
              if (!bloqueado) evento.currentTarget.style.background = "var(--campo-fondo)";
            }}
            onMouseLeave={(evento) => {
              evento.currentTarget.style.background = "none";
            }}
          >
            <span style={{ width: "1rem", display: "inline-flex", justifyContent: "center" }}>
              {visible && <Check size={14} aria-hidden="true" />}
            </span>
            {seccion.etiqueta}
          </button>
        );
      })}

      <div style={{ borderTop: "1px solid var(--borde)", margin: "0.3rem 0" }} />

      <button
        type="button"
        onClick={() => {
          onRestablecer();
          onCerrar();
        }}
        style={{
          display: "block",
          width: "100%",
          padding: "0.35rem 0.6rem",
          background: "none",
          border: "none",
          borderRadius: "var(--radio-chico)",
          color: "var(--muted)",
          fontSize: "0.85rem",
          textAlign: "left",
          cursor: "pointer",
        }}
        onMouseEnter={(evento) => (evento.currentTarget.style.background = "var(--campo-fondo)")}
        onMouseLeave={(evento) => (evento.currentTarget.style.background = "none")}
      >
        Restablecer orden y visibilidad
      </button>
    </div>
  );
}
