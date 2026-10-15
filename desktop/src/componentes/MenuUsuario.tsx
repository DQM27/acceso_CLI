import { useEffect, useRef, useState } from "react";
import { KeyRound, LogOut, Moon, Sun } from "lucide-react";
import type { UsuarioSesion } from "../api";
import CambiarPasswordModal from "./CambiarPasswordModal";
import { ListaFlotante } from "./ListaFlotante";
import { useListaFlotante } from "./ListaFlotante.logica";
import { NOMBRE_TEMA } from "./SelectorTema.logica";
import type { Tema } from "./SelectorTema.logica";

/** La inicial del nombre, para el círculo (vacío si no hay nombre). */
function inicialDe(nombre: string): string {
  return nombre.trim().charAt(0).toLocaleUpperCase("es");
}

/**
 * La cuenta de quien tiene la sesión, al fondo del menú lateral: un círculo
 * con su inicial y, con el menú abierto, su nombre. Es el lugar donde se
 * busca "mi cuenta", así que ahí viven todas las cosas de la persona: el tema
 * (que se guarda por usuario), cambiar la contraseña y cerrar sesión. Con el
 * menú colapsado queda sólo el círculo.
 *
 * Al hacer click abre un popover con el nombre y el rol y las tres acciones
 * -- mismo mecanismo que "Columnas ▾" en `Tabla.tsx`
 * (`ListaFlotante`/`useListaFlotante`), con `direccion="arriba"` porque el
 * disparador vive pegado al borde inferior de la ventana.
 */
export default function MenuUsuario({
  sesion,
  colapsado,
  tema,
  onAlternarTema,
  onCerrarSesion,
}: {
  sesion: UsuarioSesion;
  colapsado: boolean;
  tema: Tema;
  onAlternarTema: () => void;
  onCerrarSesion: () => void;
}) {
  const [abierto, setAbierto] = useState(false);
  const [cambiandoPassword, setCambiandoPassword] = useState(false);
  const { campoRef, posicion } = useListaFlotante(abierto);
  const popoverRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!abierto) return;
    function alHacerClicAfuera(evento: MouseEvent) {
      const objetivo = evento.target as Node;
      if (campoRef.current?.contains(objetivo) || popoverRef.current?.contains(objetivo)) return;
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

  const IconoTema = tema === "tokyo-night" ? Moon : Sun;

  return (
    <div ref={campoRef} className="usuario">
      <button
        type="button"
        className="usuario-tarjeta"
        onClick={() => setAbierto((a) => !a)}
        aria-expanded={abierto}
        aria-haspopup="menu"
        title={`${sesion.nombre} — tema, contraseña y cerrar sesión`}
        aria-label={`Cuenta de ${sesion.nombre}`}
      >
        <span className="usuario-inicial" aria-hidden="true">
          {inicialDe(sesion.nombre)}
        </span>
        {!colapsado && <span className="usuario-nombre">{sesion.nombre}</span>}
      </button>

      {abierto && posicion && (
        <ListaFlotante
          posicion={{ ...posicion, bottom: posicion.bottom + 4 }}
          ancho={240}
          direccion="arriba"
        >
          <div ref={popoverRef} className="usuario-menu" role="menu">
            <div style={{ minWidth: 0 }}>
              <p className="usuario-menu-nombre">{sesion.nombre}</p>
              <span className="chip">{sesion.rol}</span>
            </div>
            <button
              type="button"
              role="menuitem"
              className="boton boton-icono boton-salir"
              onClick={onAlternarTema}
              title="Cambiar el tema de la app"
            >
              <IconoTema size={17} strokeWidth={2} aria-hidden="true" />
              Tema: {NOMBRE_TEMA[tema]}
            </button>
            <button
              type="button"
              role="menuitem"
              className="boton boton-icono boton-salir"
              onClick={() => {
                setAbierto(false);
                setCambiandoPassword(true);
              }}
            >
              <KeyRound size={17} strokeWidth={2} aria-hidden="true" />
              Cambiar contraseña
            </button>
            <button
              type="button"
              role="menuitem"
              className="boton boton-icono boton-salir"
              onClick={onCerrarSesion}
              title="Atajo: Ctrl+Q"
            >
              <LogOut size={17} strokeWidth={2} aria-hidden="true" />
              Cerrar sesión
            </button>
          </div>
        </ListaFlotante>
      )}

      {cambiandoPassword && <CambiarPasswordModal onCerrar={() => setCambiandoPassword(false)} />}
    </div>
  );
}
