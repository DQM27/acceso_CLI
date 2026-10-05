import { useState } from "react";
import * as DropdownMenu from "@radix-ui/react-dropdown-menu";
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
  const [saliendo, setSaliendo] = useState(false);

  async function salir() {
    setSaliendo(true);
    try {
      await cerrarSesion();
    } finally {
      setSaliendo(false);
    }
  }

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
          // No modal: sin bloqueo de scroll, que inyectaría un <style> y la CSP
          // lo rechaza.
          <DropdownMenu.Root modal={false}>
            <DropdownMenu.Trigger asChild>
              <button
                type="button"
                className="flex h-9 w-9 items-center justify-center rounded-full border border-borde bg-acento-suave text-[13px] font-semibold text-acento"
                aria-label={`Cuenta de ${anfitrion.nombre}`}
              >
                {iniciales(anfitrion.nombre)}
              </button>
            </DropdownMenu.Trigger>
            <DropdownMenu.Portal>
              <DropdownMenu.Content className="flotante w-64 p-1.5" align="end" sideOffset={6} collisionPadding={12}>
                <DropdownMenu.Label className="px-2.5 pb-2 pt-1.5">
                  <div className="font-semibold text-texto">{anfitrion.nombre}</div>
                  <div className="truncate text-[13px] text-muted">{anfitrion.correo}</div>
                </DropdownMenu.Label>
                <DropdownMenu.Separator className="my-1 h-px bg-borde" />
                <DropdownMenu.Item className="opcion-lista" onSelect={alternar}>
                  {tema === "light" ? <Moon size={16} aria-hidden="true" /> : <Sun size={16} aria-hidden="true" />}
                  {tema === "light" ? "Tema oscuro" : "Tema claro"}
                </DropdownMenu.Item>
                <DropdownMenu.Item className="opcion-lista" disabled={saliendo} onSelect={salir}>
                  <LogOut size={16} aria-hidden="true" />
                  Cerrar sesión
                </DropdownMenu.Item>
              </DropdownMenu.Content>
            </DropdownMenu.Portal>
          </DropdownMenu.Root>
        )}
      </div>
    </header>
  );
}
