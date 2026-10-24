import { useRef, useState, type ReactNode } from "react";
import { ChartColumn, ImageDown, Table2 } from "lucide-react";
import { descargarArchivo } from "../../exportacion";
import { nombreDeArchivo, svgAPng } from "../imagen";
import { formatoNumero } from "./utiles";

/**
 * Componentes que comparten los gráficos del lienzo: tooltip, leyenda y el
 * marco con el cambio a "ver como tabla". Las funciones y hooks están en
 * `utiles.ts`.
 *
 * Los nombres y valores vienen de los datos: se insertan siempre como texto
 * (React los escapa), nunca como HTML.
 */

export interface FilaTooltip {
  nombre: string;
  valor: string;
  color?: string;
  /** Cómo se dibuja la marca de la serie: barra (cuadro) o línea (trazo). */
  marca?: "barra" | "linea";
}

/** Tooltip: el valor primero y fuerte, el nombre después. */
export function Tooltip({
  x,
  y,
  titulo,
  filas,
  ancho,
  pista,
}: {
  x: number;
  y: number;
  titulo: string;
  filas: FilaTooltip[];
  ancho: number;
  /** Ayuda corta al pie ("Clic para filtrar por esto"). */
  pista?: string;
}) {
  // Del lado del puntero que tenga lugar.
  const izquierda = x > ancho * 0.6;
  return (
    <div
      className="lienzo-tooltip"
      role="status"
      style={{ left: x, top: y, transform: `translate(${izquierda ? "calc(-100% - 12px)" : "12px"}, -50%)` }}
    >
      <div className="lienzo-tooltip-titulo">{titulo}</div>
      {filas.map((f) => (
        <div key={f.nombre} className="lienzo-tooltip-fila">
          {f.color && <span className={`lienzo-clave lienzo-clave-${f.marca ?? "linea"}`} style={{ background: f.color }} />}
          <strong>{f.valor}</strong>
          <span>{f.nombre}</span>
        </div>
      ))}
      {pista && <div className="lienzo-tooltip-pista">{pista}</div>}
    </div>
  );
}

export function Leyenda({ elementos, marca }: { elementos: { nombre: string; color: string; detalle?: string }[]; marca: "barra" | "linea" }) {
  return (
    <ul className="lienzo-leyenda">
      {elementos.map((e) => (
        <li key={e.nombre}>
          <span className={`lienzo-clave lienzo-clave-${marca}`} style={{ background: e.color }} aria-hidden="true" />
          <span>{e.nombre}</span>
          {e.detalle && <span className="lienzo-leyenda-detalle">{e.detalle}</span>}
        </li>
      ))}
    </ul>
  );
}

export interface TablaDeDatos {
  columnas: string[];
  filas: (string | number)[][];
}

/**
 * Marco de un gráfico: título, botón para alternar entre el gráfico y sus
 * datos en tabla (la vista accesible y sin colores), y el contenido.
 */
export function MarcoGrafico({
  titulo,
  subtitulo,
  datos,
  children,
}: {
  titulo: string;
  subtitulo?: string;
  datos: TablaDeDatos;
  children: ReactNode;
}) {
  const [comoTabla, setComoTabla] = useState(false);
  const [exportando, setExportando] = useState(false);
  const marco = useRef<HTMLElement>(null);

  // PNG del gráfico tal como se ve (colores del tema), con su título y leyenda.
  const descargarImagen = async () => {
    const seccion = marco.current;
    const svg = seccion?.querySelector<SVGSVGElement>("svg");
    if (!seccion || !svg) return;
    setExportando(true);
    try {
      const estilo = getComputedStyle(seccion);
      const leyenda = Array.from(seccion.querySelectorAll<HTMLElement>(".lienzo-leyenda li")).map((li) => ({
        nombre: li.querySelector("span:not(.lienzo-clave)")?.textContent ?? "",
        color: getComputedStyle(li.querySelector(".lienzo-clave") ?? li).backgroundColor,
      }));
      const png = await svgAPng(svg, {
        titulo,
        subtitulo,
        leyenda,
        fondo: estilo.backgroundColor,
        texto: estilo.color,
        textoSuave: getComputedStyle(seccion.querySelector(".lienzo-subtitulo") ?? seccion).color,
      });
      descargarArchivo(nombreDeArchivo(titulo, "png"), png, "image/png");
    } finally {
      setExportando(false);
    }
  };

  return (
    <section ref={marco} className="tarjeta lienzo-bloque lienzo-grafico-marco">
      <header className="lienzo-bloque-cabecera">
        <h3>{titulo}</h3>
        {subtitulo && <span className="lienzo-subtitulo">{subtitulo}</span>}
        <button
          type="button"
          className="lienzo-alternar"
          onClick={() => void descargarImagen()}
          disabled={comoTabla || exportando}
          title="Descargar como imagen (PNG)"
          aria-label={`Descargar ${titulo} como imagen`}
        >
          <ImageDown size={16} aria-hidden="true" />
        </button>
        <button
          type="button"
          className="lienzo-alternar"
          onClick={() => setComoTabla((t) => !t)}
          aria-pressed={comoTabla}
          title={comoTabla ? "Ver como gráfico" : "Ver como tabla"}
          aria-label={comoTabla ? `Ver ${titulo} como gráfico` : `Ver ${titulo} como tabla`}
        >
          {comoTabla ? <ChartColumn size={16} aria-hidden="true" /> : <Table2 size={16} aria-hidden="true" />}
        </button>
      </header>
      {comoTabla ? (
        <div className="lienzo-tabla-caja lienzo-aparecer">
          <table className="lienzo-tabla">
            <thead>
              <tr>
                {datos.columnas.map((c) => (
                  <th key={c} scope="col">
                    {c}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {datos.filas.map((fila, i) => (
                <tr key={i}>
                  {fila.map((celda, j) => (
                    <td key={j} className={typeof celda === "number" ? "lienzo-numero" : undefined}>
                      {typeof celda === "number" ? formatoNumero(celda) : celda}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : (
        children
      )}
    </section>
  );
}
