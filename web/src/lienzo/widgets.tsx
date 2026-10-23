import { CircleAlert, CircleCheck, Minus, TrendingDown, TrendingUp, TriangleAlert } from "lucide-react";
import { textoValor, type TablaCalculada } from "./calculo";
import { formatoNumero, useNumeroAnimado } from "./graficos/utiles";

/** Tono de estado de una tarjeta: el color siempre va con su ícono y su texto, nunca solo. */
export type TonoKpi = "neutro" | "exito" | "advertencia" | "error";

const ICONO_TONO = { exito: CircleCheck, advertencia: TriangleAlert, error: CircleAlert } as const;

function ValorAnimado({ valor }: { valor: number }) {
  const actual = useNumeroAnimado(valor);
  return <>{formatoNumero(Math.round(actual))}</>;
}

const PORCENTAJE = new Intl.NumberFormat("es-CR", { style: "percent", maximumFractionDigits: 0, signDisplay: "exceptZero" });

/** Mini línea de tendencia: el recorrido en gris, el último valor resaltado. */
function Tendencia({ valores, titulo }: { valores: number[]; titulo: string }) {
  if (valores.length < 2) return null;
  const ancho = 120;
  const alto = 30;
  const maximo = Math.max(...valores, 1);
  const x = (i: number) => 2 + ((ancho - 4) * i) / (valores.length - 1);
  const y = (v: number) => alto - 3 - (v / maximo) * (alto - 6);
  const puntos = valores.map((v, i) => `${x(i)},${y(v)}`).join(" ");
  const ultimo = valores.length - 1;
  return (
    <svg className="lienzo-tendencia" width={ancho} height={alto} role="img" aria-label={`Tendencia de ${titulo}`}>
      <polyline points={puntos} className="lienzo-tendencia-linea" pathLength={1} />
      <circle cx={x(ultimo)} cy={y(valores[ultimo])} r={3} className="lienzo-tendencia-punto" />
    </svg>
  );
}

export interface Comparacion {
  /** Valor del período anterior. */
  anterior: number;
  /** Contra qué se compara: "vs. las 2 semanas anteriores". */
  contra: string;
}

export function TarjetaKpi({
  valor,
  titulo,
  detalle,
  tono = "neutro",
  comparacion,
  tendencia,
}: {
  valor: number | string | null;
  titulo: string;
  detalle?: string;
  tono?: TonoKpi;
  comparacion?: Comparacion;
  tendencia?: number[];
}) {
  const Icono = tono === "neutro" ? null : ICONO_TONO[tono];
  const cambio =
    comparacion && typeof valor === "number" && comparacion.anterior !== 0
      ? (valor - comparacion.anterior) / comparacion.anterior
      : null;
  // Subir no es bueno ni malo en sí (más ingresos no es una alerta): flecha y texto, sin color de estado.
  const Flecha = cambio === null || cambio === 0 ? Minus : cambio > 0 ? TrendingUp : TrendingDown;
  return (
    <div className={`tarjeta lienzo-kpi lienzo-kpi-${tono}`}>
      <div className="lienzo-kpi-cabeza">
        <div className="lienzo-kpi-valor">
          {valor === null ? "—" : typeof valor === "number" ? <ValorAnimado valor={valor} /> : valor}
          {Icono && <Icono size={18} aria-hidden="true" className="lienzo-kpi-icono" />}
        </div>
        {tendencia && <Tendencia valores={tendencia} titulo={titulo} />}
      </div>
      <div className="lienzo-kpi-titulo">{titulo}</div>
      {comparacion && typeof valor === "number" && (
        <small className="lienzo-kpi-cambio">
          <Flecha size={14} aria-hidden="true" />
          {cambio === null
            ? `${formatoNumero(comparacion.anterior)} ${comparacion.contra}`
            : `${PORCENTAJE.format(cambio)} ${comparacion.contra} (${formatoNumero(comparacion.anterior)})`}
        </small>
      )}
      {detalle && <small>{detalle}</small>}
    </div>
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
                <td key={c.id} className={typeof fila[c.id] === "number" ? "lienzo-numero" : undefined}>
                  {textoValor(fila[c.id])}
                </td>
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
