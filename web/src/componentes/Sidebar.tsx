import type { LucideIcon } from "lucide-react";
import type { Seccion } from "../App";

/**
 * Sidebar izquierdo de `Shell` (`App.tsx`) — sólo navegación de secciones.
 * Usuario y "Cerrar sesión" viven en `MenuUsuario`, en la barra de estado
 * (ver `Shell`) — no acá. Copiado de `desktop/src/componentes/Sidebar.tsx`.
 * Sin estado propio más allá de lo puramente visual: `seccion` actual y
 * colapsado siguen viviendo en `Shell`, acá sólo llegan por props.
 */
export default function Sidebar({
  secciones,
  seccionActual,
  onCambiarSeccion,
  colapsado,
  onToggleColapsado,
}: {
  secciones: { id: Seccion; etiqueta: string; Icono: LucideIcon }[];
  seccionActual: Seccion;
  onCambiarSeccion: (id: Seccion) => void;
  colapsado: boolean;
  onToggleColapsado: () => void;
}) {
  return (
    <nav className={`shell-sidebar ${colapsado ? "shell-sidebar-colapsada" : ""}`}>
      <div className="shell-nav">
        {secciones.map(({ id, etiqueta, Icono }) => (
          <button
            key={id}
            onClick={() => onCambiarSeccion(id)}
            title={colapsado ? etiqueta : undefined}
            className={`nav-item ${seccionActual === id ? "nav-item-activo" : ""}`}
          >
            <Icono size={18} strokeWidth={2} aria-hidden="true" />
            {!colapsado && etiqueta}
          </button>
        ))}
      </div>

      <div
        style={{ flex: 1 }}
        title="Doble click para colapsar/expandir"
        onDoubleClick={onToggleColapsado}
      />
    </nav>
  );
}
