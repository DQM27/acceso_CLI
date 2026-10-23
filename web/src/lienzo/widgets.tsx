import { CircleAlert, CircleCheck, TriangleAlert } from "lucide-react";
import { textoValor, type TablaCalculada } from "./calculo";
import { formatoNumero, useNumeroAnimado } from "./graficos/utiles";

/** Tono de estado de una tarjeta: el color siempre va con su ícono y su texto, nunca solo. */
export type TonoKpi = "neutro" | "exito" | "advertencia" | "error";

const ICONO_TONO = { exito: CircleCheck, advertencia: TriangleAlert, error: CircleAlert } as const;

function ValorAnimado({ valor }: { valor: number }) {
  const actual = useNumeroAnimado(valor);
  return <>{formatoNumero(Math.round(actual))}</>;
}

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
  const Icono = tono === "neutro" ? null : ICONO_TONO[tono];
  return (
    <div className={`tarjeta lienzo-kpi lienzo-kpi-${tono}`}>
      <div className="lienzo-kpi-valor">
        {valor === null ? "—" : typeof valor === "number" ? <ValorAnimado valor={valor} /> : valor}
        {Icono && <Icono size={18} aria-hidden="true" className="lienzo-kpi-icono" />}
      </div>
      <div className="lienzo-kpi-titulo">{titulo}</div>
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
