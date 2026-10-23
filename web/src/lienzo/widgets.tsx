import { useId } from "react";
import { textoValor, type Barra, type TablaCalculada } from "./calculo";

/** Tono del valor de una tarjeta (colores del sistema "brisas"). */
export type TonoKpi = "neutro" | "error" | "advertencia";

export function TarjetaKpi({
  valor,
  titulo,
  detalle,
  tono = "neutro",
}: {
  valor: number | string | null;
  titulo: string;
  detalle?: string;
  tono?: TonoKpi;
}) {
  return (
    <div className={`tarjeta lienzo-kpi lienzo-kpi-${tono}`}>
      <div className="lienzo-kpi-valor">{valor === null ? "—" : typeof valor === "number" ? valor.toLocaleString("es-CR") : valor}</div>
      <div className="lienzo-kpi-titulo">{titulo}</div>
      {detalle && <small>{detalle}</small>}
    </div>
  );
}

const ALTO = 220;
const MARGEN = { arriba: 10, derecha: 6, abajo: 26, izquierda: 32 };

/** Techo "redondo" del eje: 80 para un máximo de 69, 5 para un 4... */
function techo(maximo: number): number {
  if (maximo <= 5) return 5;
  const paso = 10 ** Math.floor(Math.log10(maximo));
  return Math.ceil(maximo / paso) * paso;
}

/**
 * Barras en SVG propio, sin librería. `resaltarUltima` pinta más clara la
 * última barra (el día en curso, que aún no termina).
 */
export function GraficoBarras({
  barras,
  resaltarUltima = false,
  titulo,
}: {
  barras: Barra[];
  resaltarUltima?: boolean;
  titulo: string;
}) {
  const idTitulo = useId();
  if (barras.length === 0) return <p className="lienzo-vacio">Sin datos para mostrar.</p>;

  const ancho = Math.max(320, barras.length * 38);
  const maximo = techo(Math.max(...barras.map((b) => b.valor), 0));
  const altoUtil = ALTO - MARGEN.arriba - MARGEN.abajo;
  const anchoUtil = ancho - MARGEN.izquierda - MARGEN.derecha;
  const paso = anchoUtil / barras.length;
  const anchoBarra = paso * 0.62;
  const y = (valor: number) => MARGEN.arriba + altoUtil - (valor / maximo) * altoUtil;
  const marcas = [0, 0.25, 0.5, 0.75, 1].map((f) => Math.round(maximo * f));

  return (
    <svg
      className="lienzo-barras"
      viewBox={`0 0 ${ancho} ${ALTO}`}
      role="img"
      aria-labelledby={idTitulo}
      preserveAspectRatio="xMinYMid meet"
    >
      <title id={idTitulo}>{titulo}</title>
      {marcas.map((marca) => (
        <g key={marca}>
          <line x1={MARGEN.izquierda} x2={ancho - MARGEN.derecha} y1={y(marca)} y2={y(marca)} className="lienzo-eje" />
          <text x={MARGEN.izquierda - 6} y={y(marca) + 4} textAnchor="end">
            {marca}
          </text>
        </g>
      ))}
      {barras.map((barra, i) => {
        const x = MARGEN.izquierda + i * paso + (paso - anchoBarra) / 2;
        const ultima = resaltarUltima && i === barras.length - 1;
        return (
          <g key={barra.etiqueta}>
            <rect
              x={x}
              y={y(barra.valor)}
              width={anchoBarra}
              height={Math.max(0, MARGEN.arriba + altoUtil - y(barra.valor))}
              rx={3}
              className={ultima ? "lienzo-barra lienzo-barra-hoy" : "lienzo-barra"}
            >
              <title>{`${barra.etiqueta}: ${barra.valor}`}</title>
            </rect>
            <text x={x + anchoBarra / 2} y={ALTO - 8} textAnchor="middle">
              {barra.etiqueta.length > 8 ? `${barra.etiqueta.slice(0, 7)}…` : barra.etiqueta}
            </text>
          </g>
        );
      })}
    </svg>
  );
}

export function TablaLienzo({ tabla }: { tabla: TablaCalculada }) {
  if (tabla.total === 0) return <p className="lienzo-vacio">Sin datos para mostrar.</p>;
  return (
    <div className="lienzo-tabla-caja">
      <table className="lienzo-tabla">
        <thead>
          <tr>
            {tabla.columnas.map((c) => (
              <th key={c.id} scope="col">
                {c.etiqueta}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {tabla.filas.map((fila, i) => (
            <tr key={i}>
              {tabla.columnas.map((c) => (
                <td key={c.id}>{textoValor(fila[c.id])}</td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
      {tabla.total > tabla.filas.length && (
        <p className="lienzo-nota">
          Se muestran {tabla.filas.length.toLocaleString("es-CR")} de {tabla.total.toLocaleString("es-CR")} filas.
        </p>
      )}
    </div>
  );
}
