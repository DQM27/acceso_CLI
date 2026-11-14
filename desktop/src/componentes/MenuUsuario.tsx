import { useRef, useState } from "react";
import { useCerrarAlClicAfuera } from "./useCerrarAlClicAfuera";
import { KeyRound, LogOut, Moon, PanelLeft, RefreshCw, Sun } from "lucide-react";
import type { UsuarioSesion } from "../api";
import CambiarPasswordModal from "./CambiarPasswordModal";
import Engranaje from "./Engranaje";
import { ListaFlotante } from "./ListaFlotante";
import { useAperturaAnimada, useListaFlotante } from "./ListaFlotante.logica";
import { NOMBRE_TEMA } from "./SelectorTema.logica";
import { useVersionApp } from "./useVersionApp";
import type { Tema } from "./SelectorTema.logica";

/** La inicial del nombre, para el círculo (vacío si no hay nombre). */
function inicialDe(nombre: string): string {
  return nombre.trim().charAt(0).toLocaleUpperCase("es");
}

/**
 * La cuenta de quien tiene la sesión, al fondo del menú lateral: un círculo
 * con su inicial dentro de un engranaje (que dice "ajustes de mi cuenta") y,
 * con el menú abierto, su nombre. Es el lugar donde se
 * busca "mi cuenta", así que ahí viven todas las cosas de la persona: el tema
 * (que se guarda por usuario), cambiar la contraseña y cerrar sesión. Con el
 * menú colapsado queda sólo el círculo.
 *
 * Al hacer click abre un popover con el nombre y el rol y las tres acciones
 * -- mismo mecanismo que "Columnas ▾" en `Tabla.tsx`
 * (`ListaFlotante`/`useListaFlotante`), pero hacia la derecha del disparador
 * (el disparador vive al fondo del menú lateral, y el contenido está al
 * costado), entrando y saliendo deslizado desde él.
 */
export default function MenuUsuario({
  sesion,
  colapsado,
  tema,
  onAlternarTema,
  onCerrarSesion,
  onPersonalizarMenu,
  onBuscarActualizacion,
  buscandoActualizacion,
}: {
  sesion: UsuarioSesion;
  colapsado: boolean;
  tema: Tema;
  onAlternarTema: () => void;
  onCerrarSesion: () => void;
  /** Abre el menú para mostrar/ocultar secciones junto a la cuenta. Antes
   * sólo se llegaba con clic derecho en el menú lateral, y casi nadie lo
   * descubría (auditoría 2026-11-14, U10). */
  onPersonalizarMenu: (posicion: { x: number; y: number }) => void;
  /** Antes era la versión del pie del menú, sin nada que lo indicara (U11). */
  onBuscarActualizacion: () => void;
  buscandoActualizacion: boolean;
}) {
  // `abierto` es lo que quiere quien opera; `visible` lo mantiene en pantalla
  // el rato que dura la animación de salida.
  const { abierto, visible, abrir, cerrar } = useAperturaAnimada();
  const [cambiandoPassword, setCambiandoPassword] = useState(false);
  const { campoRef, posicion } = useListaFlotante(visible);
  const popoverRef = useRef<HTMLDivElement>(null);

  useCerrarAlClicAfuera(abierto, [campoRef, popoverRef], cerrar);

  const IconoTema = tema === "tokyo-night" ? Moon : Sun;
  const version = useVersionApp();

  return (
    <div ref={campoRef} className="usuario">
      <button
        type="button"
        className="usuario-tarjeta"
        onClick={abierto ? cerrar : abrir}
        aria-expanded={abierto}
        aria-haspopup="menu"
        title={`${sesion.nombre} — tema, menú, actualizaciones, contraseña y cerrar sesión`}
        aria-label={`Cuenta de ${sesion.nombre}`}
      >
        <Engranaje>{inicialDe(sesion.nombre)}</Engranaje>
        {!colapsado && <span className="usuario-nombre">{sesion.nombre}</span>}
      </button>

      {visible && posicion && (
        <ListaFlotante
          posicion={posicion}
          ancho={240}
          direccion="derecha"
          animacion="lateral"
          saliendo={!abierto}
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
              title="Mostrar, ocultar o restablecer las secciones del menú lateral"
              onClick={() => {
                // Al costado de la cuenta, a su altura (el menú de secciones
                // mide ~300 px).
                const caja = campoRef.current?.getBoundingClientRect();
                cerrar();
                onPersonalizarMenu({
                  x: (caja?.right ?? 0) + 8,
                  y: Math.max(8, (caja?.bottom ?? 0) - 300),
                });
              }}
            >
              <PanelLeft size={17} strokeWidth={2} aria-hidden="true" />
              Personalizar menú
            </button>
            <button
              type="button"
              role="menuitem"
              className="boton boton-icono boton-salir"
              onClick={onBuscarActualizacion}
              disabled={buscandoActualizacion}
              title={version ? `Versión instalada: ${version}` : undefined}
            >
              <RefreshCw
                size={17}
                strokeWidth={2}
                aria-hidden="true"
                className={buscandoActualizacion ? "girando" : undefined}
              />
              {buscandoActualizacion ? "Buscando…" : "Buscar actualización"}
            </button>
            <button
              type="button"
              role="menuitem"
              className="boton boton-icono boton-salir"
              onClick={() => {
                cerrar();
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
