import { useEffect, useRef, useState } from "react";
import { LogOut } from "lucide-react";
import type { UsuarioSesion } from "../api";
import { ListaFlotante, useListaFlotante } from "./ListaFlotante";

/**
 * Nombre del usuario en la esquina inferior derecha de la barra de estado
 * — mismo lenguaje que los ítems de la status bar de VSC (texto normal
 * hasta que se pasa el mouse, ahí aparece el fondo de botón, ver
 * `.barra-estado-boton`). Al hacer click abre un popover con nombre/rol y
 * "Cerrar sesión" — mismo mecanismo que "Columnas ▾" en `Tabla.tsx`
 * (`ListaFlotante`/`useListaFlotante`), pero `direccion="arriba"` porque
 * el disparador vive pegado al borde inferior de la ventana, sin espacio
 * para abrir hacia abajo. Antes vivía fijo en el sidebar (`.shell-usuario`)
 * — se movió acá para dejar el sidebar sólo con navegación.
 */
export default function MenuUsuario({
  sesion,
  onCerrarSesion,
}: {
  sesion: UsuarioSesion;
  onCerrarSesion: () => void;
}) {
  const [abierto, setAbierto] = useState(false);
  const { campoRef, posicion } = useListaFlotante(abierto);
  const popoverRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!abierto) return;
    function alHacerClicAfuera(evento: MouseEvent) {
      const objetivo = evento.target as Node;
      if (campoRef.current?.contains(objetivo) || popoverRef.current?.contains(objetivo)) return;
      setAbierto(false);
    }
    document.addEventListener("mousedown", alHacerClicAfuera);
    return () => document.removeEventListener("mousedown", alHacerClicAfuera);
  }, [abierto, campoRef]);

  return (
    <div ref={campoRef}>
      <button type="button" className="barra-estado-boton" onClick={() => setAbierto((a) => !a)}>
        {sesion.nombre}
      </button>

      {/* Separación del popover hacia la pared derecha (2px) y hacia el
          trigger que tiene debajo (8px, `bottom` ya trae +4 de
          `useListaFlotante` — mismo margen que usa "Columnas ▾" al abrir
          hacia abajo — más 4 acá). */}
      {abierto && posicion && (
        <ListaFlotante
          posicion={{ ...posicion, right: posicion.right + 2, bottom: posicion.bottom + 4 }}
          ancho={220}
          alinear="derecha"
          direccion="arriba"
        >
          <div
            ref={popoverRef}
            style={{ padding: "0.9rem", display: "flex", flexDirection: "column", gap: "0.75rem" }}
          >
            <div style={{ minWidth: 0 }}>
              <p style={{ margin: 0, fontSize: "0.9rem", fontWeight: 600, color: "var(--texto)" }}>
                {sesion.nombre}
              </p>
              <span className="chip">{sesion.rol}</span>
            </div>
            <button
              type="button"
              className="boton boton-icono boton-salir"
              onClick={onCerrarSesion}
            >
              <LogOut size={17} strokeWidth={2} aria-hidden="true" />
              Cerrar sesión
            </button>
          </div>
        </ListaFlotante>
      )}
    </div>
  );
}
