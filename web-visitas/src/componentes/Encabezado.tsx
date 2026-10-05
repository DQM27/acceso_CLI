import { useEffect, useRef, useState } from "react";
import { Link } from "react-router-dom";
import { ChevronLeft, LogOut, Moon, Sun } from "lucide-react";
import { useAuth } from "../contexto/AuthContexto";
import { useTema } from "../tema";
import marca from "../assets/marca.png";

function iniciales(nombre: string) {
  return nombre
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((p) => p[0]?.toUpperCase())
    .join("");
}

/** Barra superior de 56 px: marca (o "volver") a la izquierda, la cuenta a
 * la derecha. Misma barra blanca con borde fino que el panel. */
export default function Encabezado({ titulo, volver }: { titulo?: string; volver?: string }) {
  const { anfitrion, cerrarSesion } = useAuth();
  const { tema, alternar } = useTema();
  const [abierto, setAbierto] = useState(false);
  const [saliendo, setSaliendo] = useState(false);
  const menu = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!abierto) return;
    const alClic = (evento: MouseEvent) => {
      if (!menu.current?.contains(evento.target as Node)) setAbierto(false);
    };
    const alTecla = (evento: KeyboardEvent) => {
      if (evento.key === "Escape") setAbierto(false);
    };
    document.addEventListener("mousedown", alClic);
    document.addEventListener("keydown", alTecla);
    return () => {
      document.removeEventListener("mousedown", alClic);
      document.removeEventListener("keydown", alTecla);
    };
  }, [abierto]);

  return (
    <header className="sticky top-0 z-20 border-b border-borde bg-panel">
      <div className="mx-auto flex h-14 max-w-[720px] items-center justify-between gap-3 px-4">
        <div className="flex min-w-0 items-center gap-2">
          {volver ? (
            <Link to={volver} className="boton boton-discreto -ml-2" aria-label="Volver">
              <ChevronLeft aria-hidden="true" />
            </Link>
          ) : (
            <Link to="/visitas" className="flex items-center gap-2 no-underline text-texto">
              <img src={marca} alt="" className="h-7 w-7 rounded-chico" />
              <span className="font-semibold">Visitas</span>
            </Link>
          )}
          {titulo && <span className="truncate text-[15px] font-semibold">{titulo}</span>}
        </div>

        {anfitrion && (
          <div className="relative" ref={menu}>
            <button
              type="button"
              className="flex h-9 w-9 items-center justify-center rounded-full border border-borde bg-acento-suave text-[13px] font-semibold text-acento"
              aria-haspopup="menu"
              aria-expanded={abierto}
              aria-label={`Cuenta de ${anfitrion.nombre}`}
              onClick={() => setAbierto((a) => !a)}
            >
              {iniciales(anfitrion.nombre)}
            </button>
            {abierto && (
              <div
                role="menu"
                className="tarjeta absolute right-0 top-11 z-30 flex w-64 flex-col gap-1 p-2 shadow-[var(--sombra-panel)]"
              >
                <div className="px-2 py-1">
                  <div className="font-semibold">{anfitrion.nombre}</div>
                  <div className="truncate text-[13px] text-muted">{anfitrion.correo}</div>
                </div>
                <button type="button" role="menuitem" className="boton boton-discreto justify-start" onClick={alternar}>
                  {tema === "light" ? <Moon aria-hidden="true" /> : <Sun aria-hidden="true" />}
                  {tema === "light" ? "Tema oscuro" : "Tema claro"}
                </button>
                <button
                  type="button"
                  role="menuitem"
                  className="boton boton-discreto justify-start"
                  disabled={saliendo}
                  onClick={async () => {
                    setSaliendo(true);
                    try {
                      await cerrarSesion();
                    } finally {
                      setSaliendo(false);
                    }
                  }}
                >
                  <LogOut aria-hidden="true" />
                  Cerrar sesión
                </button>
              </div>
            )}
          </div>
        )}
      </div>
    </header>
  );
}
