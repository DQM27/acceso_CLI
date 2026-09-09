import { useEffect, useRef, useState } from "react";
import { ListaFlotante, useListaFlotante } from "./ListaFlotante";
import type { UnidadOperativa } from "../api/historial";

/**
 * Botón "N de M unidades ▾" que abre un popover de checkboxes -- misma
 * arquitectura que "Columnas ▾" de `Tabla.tsx` (popover posicionado con
 * `useListaFlotante`, cierra al clickear afuera, cada checkbox aplica al
 * toque, sin paso de "Aplicar" como `SelectorRangoFecha`) en vez de un
 * `<select multiple>` nativo. `excluidas` sigue el mismo criterio que
 * `ocultas` de `Tabla.tsx`: el Set guarda lo DESMARCADO, así que ningún
 * sitio nuevo que aparezca después queda afuera del filtro por accidente
 * (por defecto, nada está excluido).
 */

/** Mismo texto que muestra el botón — se exporta para no reimplementar este
 * formateo en otro lado, mismo criterio que `textoRangoFecha`. */
export function textoUnidadesOperativas(total: number, excluidas: number): string {
  if (total === 0) return "Unidades operativas";
  if (excluidas === 0) return "Todas las unidades";
  const incluidas = total - excluidas;
  if (incluidas === 0) return "Ninguna unidad";
  return `${incluidas} de ${total} unidades`;
}

export default function SelectorUnidadesOperativas({
  unidades,
  excluidas,
  onCambiar,
}: {
  unidades: UnidadOperativa[];
  excluidas: Set<string>;
  onCambiar: (excluidas: Set<string>) => void;
}) {
  const [abierto, setAbierto] = useState(false);
  const { campoRef, posicion } = useListaFlotante(abierto);
  const popoverRef = useRef<HTMLDivElement>(null);

  // Mismo mecanismo que `SelectorRangoFecha`/"Columnas ▾": cierra al
  // clickear afuera del botón y del popover (portal a `document.body`).
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

  function alternar(id: string) {
    const siguiente = new Set(excluidas);
    if (siguiente.has(id)) siguiente.delete(id);
    else siguiente.add(id);
    onCambiar(siguiente);
  }

  return (
    <>
      <div ref={campoRef}>
        <button
          type="button"
          className="boton"
          onClick={() => setAbierto((a) => !a)}
          disabled={unidades.length === 0}
        >
          {textoUnidadesOperativas(unidades.length, excluidas.size)} ▾
        </button>
      </div>
      {abierto && posicion && (
        <ListaFlotante posicion={posicion} ancho={220} alinear="izquierda">
          <div
            ref={popoverRef}
            style={{
              padding: "0.75rem 1rem",
              display: "flex",
              flexDirection: "column",
              gap: "0.4rem",
            }}
          >
            {unidades.map((unidad) => (
              <label key={unidad.id} style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
                <input
                  type="checkbox"
                  checked={!excluidas.has(unidad.id)}
                  onChange={() => alternar(unidad.id)}
                />
                {unidad.nombre}
              </label>
            ))}
          </div>
        </ListaFlotante>
      )}
    </>
  );
}
