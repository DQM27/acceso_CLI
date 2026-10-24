import { useId } from "react";
import { fechaLarga, nivel, rangosDeNiveles, type DatosCalendario } from "../mapas";
import { Leyenda, Tooltip } from "./comun";
import { formatoNumero, useAncho, useIndiceActivo } from "./utiles";

/**
 * Calendario en un solo SVG (así se exporta entero como imagen): los meses uno
 * al lado del otro, de lunes a domingo, cada día con el tono de su valor. El
 * día de hoy lleva un borde. Tooltip con puntero o teclado y clic en un día con
 * valor para filtrar la vista por ese día.
 */
const INICIALES = ["L", "K", "M", "J", "V", "S", "D"];
const NOMBRES_DIAS = ["lunes", "martes", "miércoles", "jueves", "viernes", "sábado", "domingo"];
const SEPARACION_MESES = 24;
const ALTO_TITULO_MES = 22;
const ALTO_DIAS_SEMANA = 16;
const CELDA_MINIMA = 16;
const CELDA_MAXIMA = 34;
/** Con celdas más chicas que esto, el número del día no entra. */
const CELDA_PARA_NUMERO = 20;

export default function GraficoCalendario({
  datos,
  titulo,
  medida,
  alSeleccionar,
}: {
  datos: DatosCalendario;
  titulo: string;
  /** Qué se cuenta, para el tooltip ("Contratistas"). */
  medida: string;
  /** Clic (o Enter) sobre un día con valor: filtrar por ese día. */
  alSeleccionar?: (fecha: string) => void;
}) {
  const idTitulo = useId();
  const { ref, ancho } = useAncho<HTMLDivElement>();
  const dias = datos.meses.flatMap((m) => m.dias);
  const seleccionar = (indice: number) => {
    const dia = dias[indice];
    if (alSeleccionar && dia && dia.valor > 0) alSeleccionar(dia.fecha);
  };
  const activo = useIndiceActivo(dias.length, seleccionar);

  if (datos.meses.length === 0) return <p className="lienzo-vacio">Sin datos para mostrar.</p>;

  // Cuántos meses por fila entran: cada mes son 7 celdas más el aire entre meses.
  const porFila = Math.max(1, Math.min(datos.meses.length, Math.floor((ancho + SEPARACION_MESES) / (7 * CELDA_MINIMA + SEPARACION_MESES))));
  const celda = Math.max(CELDA_MINIMA, Math.min(CELDA_MAXIMA, Math.floor((ancho - SEPARACION_MESES * (porFila - 1)) / (7 * porFila))));
  const anchoMes = celda * 7;
  const altoMes = ALTO_TITULO_MES + ALTO_DIAS_SEMANA + celda * 6;
  const filasDeMeses = Math.ceil(datos.meses.length / porFila);
  const alto = filasDeMeses * altoMes + (filasDeMeses - 1) * SEPARACION_MESES + 2;
  const anchoDibujo = porFila * anchoMes + (porFila - 1) * SEPARACION_MESES;

  const origenMes = (m: number) => ({
    x: (m % porFila) * (anchoMes + SEPARACION_MESES),
    y: Math.floor(m / porFila) * (altoMes + SEPARACION_MESES),
  });
  const posicion = new Map<string, { x: number; y: number }>();
  datos.meses.forEach((mes, m) => {
    const o = origenMes(m);
    for (const dia of mes.dias) {
      posicion.set(dia.fecha, { x: o.x + dia.diaSemana * celda, y: o.y + ALTO_TITULO_MES + ALTO_DIAS_SEMANA + dia.semana * celda });
    }
  });

  const i = activo.indice;
  const tooltip =
    i === null || !dias[i]
      ? null
      : (() => {
          const dia = dias[i];
          const p = posicion.get(dia.fecha) ?? { x: 0, y: 0 };
          return {
            x: p.x + celda / 2,
            y: p.y + celda / 2,
            titulo: fechaLarga(dia.fecha),
            filas: [{ nombre: medida, valor: formatoNumero(dia.valor) }],
            pista: alSeleccionar && dia.valor > 0 ? "Clic para ver ese día" : undefined,
          };
        })();

  // Dónde empieza cada mes en la lista de días (para el índice de cada día).
  const inicioDeMes = datos.meses.map((_, m) => datos.meses.slice(0, m).reduce((n, mes) => n + mes.dias.length, 0));
  return (
    <div ref={ref} className="lienzo-grafico">
      <svg
        width={Math.max(anchoDibujo, 1)}
        height={alto}
        role="img"
        aria-labelledby={idTitulo}
        className="lienzo-svg"
        onPointerLeave={() => activo.alPuntero(null)}
        {...activo.propsTeclado}
      >
        <title id={idTitulo}>{`${titulo}. Use las flechas para recorrer los días.`}</title>
        {datos.meses.map((mes, m) => {
          const o = origenMes(m);
          return (
            <g key={mes.clave}>
              <text x={o.x} y={o.y + 14} className="lienzo-calendario-mes">
                {mes.etiqueta}
              </text>
              {INICIALES.map((letra, d) => (
                <text
                  key={d}
                  x={o.x + d * celda + celda / 2}
                  y={o.y + ALTO_TITULO_MES + 11}
                  textAnchor="middle"
                  className="lienzo-eje-texto"
                >
                  <title>{NOMBRES_DIAS[d]}</title>
                  {letra}
                </text>
              ))}
              {mes.dias.map((dia) => {
                const propio = inicioDeMes[m] + dia.numero - 1;
                const p = posicion.get(dia.fecha) ?? { x: 0, y: 0 };
                const n = nivel(dia.valor, datos.maximo);
                const atenuada = i !== null && i !== propio;
                const esHoy = dia.fecha === datos.hoy;
                return (
                  <g
                    key={dia.fecha}
                    className={`lienzo-calor-celda${atenuada ? " lienzo-atenuada" : ""}${alSeleccionar && dia.valor > 0 ? " lienzo-zona-seleccionable" : ""}`}
                    onPointerEnter={() => activo.alPuntero(propio)}
                    onClick={() => seleccionar(propio)}
                  >
                    <rect
                      x={p.x + 1}
                      y={p.y + 1}
                      width={celda - 2}
                      height={celda - 2}
                      rx={4}
                      className={`lienzo-calor-n${n}${esHoy ? " lienzo-calendario-hoy" : ""}`}
                    />
                    {celda >= CELDA_PARA_NUMERO && (
                      <text
                        x={p.x + celda / 2}
                        y={p.y + celda / 2 + 4}
                        textAnchor="middle"
                        className={`lienzo-calendario-numero lienzo-calor-numero-n${n}`}
                      >
                        {dia.numero}
                      </text>
                    )}
                  </g>
                );
              })}
            </g>
          );
        })}
      </svg>
      <Leyenda
        marca="barra"
        elementos={rangosDeNiveles(datos.maximo).map(({ nivel: n, texto }) => ({ nombre: texto, color: `var(--calor-${n})` }))}
      />
      {(datos.fuera > 0 || datos.sinFecha > 0) && (
        <p className="lienzo-nota">
          {datos.fuera > 0 && `${formatoNumero(datos.fuera)} más después de ${datos.meses[datos.meses.length - 1].etiqueta}. `}
          {datos.sinFecha > 0 && `${formatoNumero(datos.sinFecha)} sin fecha.`}
        </p>
      )}
      {tooltip && <Tooltip {...tooltip} ancho={ancho} />}
    </div>
  );
}
