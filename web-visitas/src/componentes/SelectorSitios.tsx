import { useEffect, useRef, useState } from "react";
import { ChevronDown, MapPin, Search } from "lucide-react";
import type { Sitio } from "../dominio";

const MARCAS_DIACRITICAS = new RegExp("[\\u0300-\\u036f]", "g");
function normalizar(texto: string): string {
  return texto.normalize("NFD").replace(MARCAS_DIACRITICAS, "").toLowerCase();
}

const MAX_TAGS_VISIBLES = 2;

/**
 * Combobox con buscador para elegir uno o varios sitios -- reemplaza los
 * chips de antes (`.selector-sitios` con `flex-wrap` sin límite), que con
 * muchos sitios (hasta `MAX_SITIOS = 100`, ver dominio.ts) apilaban varias
 * filas y empujaban el resto del formulario hacia abajo. Este control
 * queda siempre a la misma altura (una fila), sin importar cuántos sitios
 * haya -- el desplegable es lo único que scrollea, y sólo mientras está
 * abierto. Sin JS de Bootstrap (Popper) por la misma razón que el resto de
 * la migración: evita el riesgo de la CSP estricta con sus íconos
 * `data:image`.
 */
export default function SelectorSitios({
  sitios,
  seleccionados,
  onCambiar,
  invalido,
  describedBy,
}: {
  sitios: Sitio[];
  seleccionados: string[];
  onCambiar: (ids: string[]) => void;
  invalido?: boolean;
  describedBy?: string;
}) {
  const [abierto, setAbierto] = useState(false);
  const [busqueda, setBusqueda] = useState("");
  const raiz = useRef<HTMLDivElement>(null);
  const control = useRef<HTMLButtonElement>(null);
  const buscador = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!abierto) return;
    buscador.current?.focus();
    function alHacerClickAfuera(evento: MouseEvent) {
      if (!raiz.current?.contains(evento.target as Node)) setAbierto(false);
    }
    function alPresionarTecla(evento: KeyboardEvent) {
      if (evento.key === "Escape") {
        setAbierto(false);
        control.current?.focus();
      }
    }
    document.addEventListener("mousedown", alHacerClickAfuera);
    document.addEventListener("keydown", alPresionarTecla);
    return () => {
      document.removeEventListener("mousedown", alHacerClickAfuera);
      document.removeEventListener("keydown", alPresionarTecla);
    };
  }, [abierto]);

  const elegidos = sitios.filter((s) => seleccionados.includes(s.id));
  const filtrados = sitios.filter((s) =>
    normalizar(s.nombre).includes(normalizar(busqueda)),
  );

  function alternar(id: string) {
    onCambiar(
      seleccionados.includes(id)
        ? seleccionados.filter((sid) => sid !== id)
        : [...seleccionados, id],
    );
  }

  const resumenAccesible =
    elegidos.length === 0
      ? "ningún sitio seleccionado"
      : `${elegidos.length} sitio${elegidos.length === 1 ? "" : "s"} seleccionado${elegidos.length === 1 ? "" : "s"}: ${elegidos.map((s) => s.nombre).join(", ")}`;

  return (
    <div className="selector-sitios" ref={raiz}>
      <button
        type="button"
        ref={control}
        className={`combo-control${abierto ? " abierto" : ""}`}
        aria-haspopup="listbox"
        aria-expanded={abierto}
        aria-label={`Sitios de la visita. ${resumenAccesible}`}
        aria-invalid={invalido}
        aria-describedby={describedBy}
        onClick={() => setAbierto((v) => !v)}
      >
        <MapPin aria-hidden="true" className="combo-icono" />
        <span className="combo-tags" aria-hidden="true">
          {elegidos.length === 0 ? (
            <span className="marcador">Todos los sitios</span>
          ) : (
            <>
              {elegidos.slice(0, MAX_TAGS_VISIBLES).map((s) => (
                <span className="combo-tag" key={s.id}>
                  {s.nombre}
                </span>
              ))}
              {elegidos.length > MAX_TAGS_VISIBLES && (
                <span className="combo-mas">
                  +{elegidos.length - MAX_TAGS_VISIBLES} más
                </span>
              )}
            </>
          )}
        </span>
        <ChevronDown aria-hidden="true" className="combo-chevron" />
      </button>
      {abierto && (
        <div className="combo-panel">
          <div className="combo-buscar">
            <Search aria-hidden="true" />
            <label htmlFor="buscador-sitios" className="visually-hidden">
              Buscar sitio
            </label>
            <input
              type="search"
              id="buscador-sitios"
              ref={buscador}
              autoComplete="off"
              placeholder="Buscar sitio…"
              value={busqueda}
              onChange={(e) => setBusqueda(e.target.value)}
            />
          </div>
          <div className="combo-acciones">
            <button
              type="button"
              className="btn btn-link btn-sm p-0"
              onClick={() =>
                onCambiar([
                  ...new Set([...seleccionados, ...filtrados.map((s) => s.id)]),
                ])
              }
            >
              Seleccionar todo
            </button>
            <button
              type="button"
              className="btn btn-link btn-sm p-0"
              onClick={() => onCambiar([])}
            >
              Limpiar
            </button>
          </div>
          <div className="combo-lista">
            {filtrados.length === 0 ? (
              <p className="combo-vacio">
                Ningún sitio coincide con la búsqueda.
              </p>
            ) : (
              filtrados.map((sitio) => {
                const id = `combo-sitio-${sitio.id}`;
                return (
                  <div className="combo-fila" key={sitio.id}>
                    <input
                      type="checkbox"
                      id={id}
                      checked={seleccionados.includes(sitio.id)}
                      onChange={() => alternar(sitio.id)}
                    />
                    <MapPin aria-hidden="true" />
                    <label htmlFor={id}>{sitio.nombre}</label>
                  </div>
                );
              })
            )}
          </div>
        </div>
      )}
    </div>
  );
}
