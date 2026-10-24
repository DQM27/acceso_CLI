import { CircleAlert, CircleCheck, Minus, TrendingDown, TrendingUp, TriangleAlert } from "lucide-react";
import { textoComparacion, type Comparacion } from "./comparacion";
import { formatoNumero, useNumeroAnimado } from "./graficos/utiles";

/** Tono de estado de una tarjeta: el color siempre va con su ícono y su texto, nunca solo. */
export type TonoKpi = "neutro" | "exito" | "advertencia" | "error";

const ICONO_TONO = { exito: CircleCheck, advertencia: TriangleAlert, error: CircleAlert } as const;

function ValorAnimado({ valor }: { valor: number }) {
  const actual = useNumeroAnimado(valor);
  return <>{formatoNumero(Math.round(actual))}</>;
}

/** Mini línea de tendencia: el recorrido en gris, el último valor resaltado (tarjetas KPI y filas de tabla). */
export function Tendencia({
  valores,
  titulo,
  ancho = 120,
  alto = 30,
}: {
  valores: number[];
  titulo: string;
  ancho?: number;
  alto?: number;
}) {
  if (valores.length < 2) return null;
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
  const comparado = comparacion?.actual ?? (typeof valor === "number" ? valor : null);
  const diferencia = comparacion && comparado !== null ? comparado - comparacion.anterior : null;
  // Subir no es bueno ni malo en sí (más ingresos no es una alerta): flecha y texto, sin color de estado.
  const Flecha = diferencia === null || diferencia === 0 ? Minus : diferencia > 0 ? TrendingUp : TrendingDown;
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
      {comparacion && comparado !== null && (
        <small className="lienzo-kpi-cambio">
          <Flecha size={14} aria-hidden="true" />
          {textoComparacion(comparado, comparacion)}
        </small>
      )}
      {detalle && <small>{detalle}</small>}
    </div>
  );
}
