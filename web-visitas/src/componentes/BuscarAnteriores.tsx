import { useEffect, useId, useState } from "react";
import type { KeyboardEvent } from "react";
import * as Popover from "@radix-ui/react-popover";
import { useQuery } from "@tanstack/react-query";
import { Search, UserPlus } from "lucide-react";
import { visitantesAnteriores } from "../api";
import type { VisitanteAnterior } from "../dominio";
import { fechaLegible } from "../fecha";

/** Buscador de personas que el anfitrión ya agendó (combobox): al enfocarlo
 * muestra las más recientes y filtra mientras se escribe. Flechas, Enter y
 * Escape funcionan como en cualquier lista desplegable. */
export default function BuscarAnteriores({ alElegir }: { alElegir: (persona: VisitanteAnterior) => void }) {
  const id = useId();
  const [texto, setTexto] = useState("");
  const [busqueda, setBusqueda] = useState("");
  const [abierto, setAbierto] = useState(false);
  const [activo, setActivo] = useState(0);
  useEffect(() => {
    const espera = setTimeout(() => setBusqueda(texto.trim()), 250);
    return () => clearTimeout(espera);
  }, [texto]);
  const resultados = useQuery({
    queryKey: ["visitantes-anteriores", busqueda],
    queryFn: ({ signal }) => visitantesAnteriores(busqueda, signal),
    staleTime: 60_000,
    enabled: abierto,
  });
  const lista = (resultados.data ?? []).slice(0, 8);
  const sinResultados = !!busqueda && resultados.isSuccess && lista.length === 0;
  const visible = abierto && (lista.length > 0 || sinResultados);
  const indice = Math.min(activo, Math.max(lista.length - 1, 0));

  function elegir(persona: VisitanteAnterior) {
    alElegir(persona);
    setTexto("");
    setBusqueda("");
    setAbierto(false);
  }

  function alTeclear(evento: KeyboardEvent<HTMLInputElement>) {
    if (evento.key === "ArrowDown" || evento.key === "ArrowUp") {
      evento.preventDefault();
      setAbierto(true);
      if (lista.length === 0) return;
      const paso = evento.key === "ArrowDown" ? 1 : -1;
      setActivo((indice + paso + lista.length) % lista.length);
    } else if (evento.key === "Enter" && visible && lista[indice]) {
      evento.preventDefault();
      elegir(lista[indice]);
    } else if (evento.key === "Escape" && visible) {
      evento.preventDefault();
      setAbierto(false);
    }
  }

  return (
    <Popover.Root open={visible} onOpenChange={setAbierto}>
      <div className="campo">
        <label htmlFor={`${id}-entrada`}>Buscar a alguien que ya vino</label>
        <Popover.Anchor asChild>
          <span className="relative">
            <Search
              size={16}
              className="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-muted"
              aria-hidden="true"
            />
            <input
              id={`${id}-entrada`}
              type="search"
              role="combobox"
              aria-autocomplete="list"
              aria-expanded={visible}
              aria-controls={`${id}-lista`}
              aria-activedescendant={visible && lista[indice] ? `${id}-opcion-${indice}` : undefined}
              className="pl-8"
              placeholder="Nombre o cédula"
              value={texto}
              autoComplete="off"
              onFocus={() => setAbierto(true)}
              onClick={() => setAbierto(true)}
              onChange={(e) => {
                setTexto(e.target.value);
                setActivo(0);
                setAbierto(true);
              }}
              onKeyDown={alTeclear}
            />
          </span>
        </Popover.Anchor>
      </div>
      <Popover.Portal>
        <Popover.Content
          className="flotante w-[var(--radix-popper-anchor-width)] p-1"
          align="start"
          sideOffset={4}
          collisionPadding={12}
          // El foco se queda en el campo para seguir escribiendo.
          onOpenAutoFocus={(e) => e.preventDefault()}
          onInteractOutside={(e) => {
            if ((e.target as HTMLElement | null)?.id === `${id}-entrada`) e.preventDefault();
          }}
        >
          {sinResultados ? (
            <p className="m-0 px-2.5 py-2 text-[13px] text-muted">Nadie con ese nombre o cédula. Agréguelo abajo.</p>
          ) : (
            <>
              <div className="px-2.5 pb-1 pt-1.5 text-[11px] font-semibold uppercase tracking-wide text-muted">
                {busqueda ? "Resultados" : "Recientes"}
              </div>
              <ul id={`${id}-lista`} role="listbox" aria-label="Personas que ya vinieron" className="m-0 list-none p-0">
                {lista.map((persona, i) => (
                  <li
                    key={persona.cedula}
                    id={`${id}-opcion-${i}`}
                    role="option"
                    aria-selected={i === indice}
                    className="opcion-lista"
                    onMouseDown={(e) => e.preventDefault()}
                    onMouseMove={() => setActivo(i)}
                    onClick={() => elegir(persona)}
                  >
                    <UserPlus size={16} className="shrink-0 text-acento" aria-hidden="true" />
                    <span className="min-w-0 flex-1">
                      <span className="block truncate font-semibold text-texto">{persona.nombre}</span>
                      <span className="block truncate text-[12px] text-muted">
                        {[persona.cedula, persona.empresa, `vino el ${fechaLegible(persona.ultima_vez)}`]
                          .filter(Boolean)
                          .join(" · ")}
                      </span>
                    </span>
                  </li>
                ))}
              </ul>
            </>
          )}
        </Popover.Content>
      </Popover.Portal>
    </Popover.Root>
  );
}
