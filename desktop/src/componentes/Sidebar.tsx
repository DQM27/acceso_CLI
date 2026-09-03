import type { LucideIcon } from "lucide-react";
import { LogOut } from "lucide-react";
import type { Seccion } from "../App";
import type { UsuarioSesion } from "../api";

/**
 * Sidebar izquierdo de `Shell` (`App.tsx`) — navegación de secciones,
 * usuario y cerrar sesión. La marca/logo NO vive acá — se movió a la
 * `.barra-superior` de `Shell`, de lado a lado por encima de sidebar +
 * contenido (mismo lugar que el logo de la app en VSC, a la izquierda de su
 * barra de menú, no adentro de la barra de actividad). Extraído de `Shell` a
 * propósito: esa función ya hace enrutamiento (`seccion`), orquesta los
 * modales globales de ingreso/salida y registra atajos de teclado — el
 * markup del sidebar no le agregaba nada a compartir ese scope, sólo hacía
 * más larga la función que sí necesita seguir creciendo con enrutamiento y
 * modales. Sin estado propio más allá de lo puramente visual: `seccion`
 * actual, colapsado y sesión siguen viviendo en `Shell`, acá sólo llegan
 * por props.
 */
export default function Sidebar({
  secciones,
  seccionActual,
  onCambiarSeccion,
  colapsado,
  onToggleColapsado,
  sesion,
  onCerrarSesion,
}: {
  secciones: { id: Seccion; etiqueta: string; Icono: LucideIcon }[];
  seccionActual: Seccion;
  onCambiarSeccion: (id: Seccion) => void;
  colapsado: boolean;
  onToggleColapsado: () => void;
  sesion: UsuarioSesion;
  onCerrarSesion: () => void;
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

      <div style={{ flex: 1 }} title="Doble click para colapsar/expandir" onDoubleClick={onToggleColapsado} />

      <div className="shell-usuario">
        {!colapsado && (
          <div style={{ minWidth: 0 }}>
            <p
              style={{
                margin: 0,
                fontSize: "0.85rem",
                fontWeight: 600,
                color: "var(--texto)",
                whiteSpace: "nowrap",
                overflow: "hidden",
                textOverflow: "ellipsis",
              }}
            >
              {sesion.nombre}
            </p>
            <span className="chip">{sesion.rol}</span>
          </div>
        )}
        <button
          className="boton boton-icono boton-salir"
          title={colapsado ? "Cerrar sesión" : undefined}
          onClick={onCerrarSesion}
        >
          <LogOut size={17} strokeWidth={2} aria-hidden="true" />
          {!colapsado && "Cerrar sesión"}
        </button>
      </div>
    </nav>
  );
}
