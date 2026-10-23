import { useId } from "react";
import type { Porcion } from "../calculo";
import { Leyenda, Tooltip } from "./comun";
import { formatoNumero, idSvg, useAncho, useIndiceActivo, useInterpolado, type Seleccion } from "./utiles";

/**
 * Torta o dona en SVG propio. Las porciones van de mayor a menor desde las 12
 * en sentido horario, con 2 px de aire entre ellas; "Otros" al final, en gris.
 * La leyenda lleva valor y porcentaje (la identidad nunca depende sólo del
 * color). Al aparecer, un barrido circular la dibuja.
 */
const PORCENTAJE = new Intl.NumberFormat("es-CR", { style: "percent", maximumFractionDigits: 1 });

function punto(cx: number, cy: number, radio: number, angulo: number): [number, number] {
  return [cx + radio * Math.cos(angulo), cy + radio * Math.sin(angulo)];
}

/** Sector entre dos ángulos (radianes, 0 = las 3, sentido horario); `interno` 0 = torta llena. */
function sector(cx: number, cy: number, externo: number, interno: number, a0: number, a1: number): string {
  // Una porción del 100 %: dos medias vueltas (un arco de 360° no se dibuja).
  if (a1 - a0 >= Math.PI * 2 - 1e-6) {
    const medio = a0 + Math.PI;
    return sector(cx, cy, externo, interno, a0, medio) + sector(cx, cy, externo, interno, medio, a0 + Math.PI * 2);
  }
  const grande = a1 - a0 > Math.PI ? 1 : 0;
  const [x0, y0] = punto(cx, cy, externo, a0);
  const [x1, y1] = punto(cx, cy, externo, a1);
  if (interno <= 0) return `M${cx},${cy}L${x0},${y0}A${externo},${externo} 0 ${grande} 1 ${x1},${y1}Z`;
  const [x2, y2] = punto(cx, cy, interno, a1);
  const [x3, y3] = punto(cx, cy, interno, a0);
  return `M${x0},${y0}A${externo},${externo} 0 ${grande} 1 ${x1},${y1}L${x2},${y2}A${interno},${interno} 0 ${grande} 0 ${x3},${y3}Z`;
}

export interface PropsTorta {
  porciones: Porcion[];
  total: number;
  colores: string[];
  titulo: string;
  dona?: boolean;
  /** Diámetro máximo. */
  tamano?: number;
  /** Clic (o Enter) sobre una porción: profundizar en ella. */
  seleccion?: Seleccion;
}

export default function GraficoTorta({
  porciones,
  total,
  colores,
  titulo,
  dona = true,
  tamano = 260,
  seleccion,
}: PropsTorta) {
  const idTitulo = useId();
  const idMascara = idSvg(useId());
  const { ref, ancho } = useAncho<HTMLDivElement>();
  const seleccionar = (k: number) => {
    const nombre = porciones[k]?.nombre;
    if (nombre !== undefined && seleccion?.puede(nombre)) seleccion.alSeleccionar(nombre);
  };
  const activo = useIndiceActivo(porciones.length, seleccionar);
  const suma = porciones.reduce((s, p) => s + p.valor, 0);
  // Las porciones crecen o se achican hasta su tamaño nuevo cuando cambian los datos.
  const dibujo = useInterpolado(
    porciones.map((p) => p.nombre),
    porciones.map((p) => p.valor),
  );
  const sumaDibujo = dibujo.reduce((s, v) => s + v, 0) || suma;

  if (porciones.length === 0 || suma <= 0) return <p className="lienzo-vacio">Sin datos para mostrar.</p>;

  // Lado a lado si hay lugar para la leyenda; si no, la leyenda va debajo.
  const lado = Math.max(140, Math.min(tamano, ancho >= 460 ? ancho * 0.5 : ancho));
  const centro = lado / 2;
  const externo = centro - 6;
  const interno = dona ? externo * 0.62 : 0;

  // Ángulos acumulados desde las 12, en sentido horario.
  const sectores = porciones.map((_, k) => {
    const antes = dibujo.slice(0, k).reduce((s, v) => s + v, 0);
    const a0 = -Math.PI / 2 + (antes / sumaDibujo) * Math.PI * 2;
    const a1 = a0 + ((dibujo[k] ?? 0) / sumaDibujo) * Math.PI * 2;
    return { a0, a1, medio: (a0 + a1) / 2 };
  });

  const i = activo.indice;
  const tooltip =
    i === null
      ? null
      : (() => {
          const [tx, ty] = punto(centro, centro, (externo + interno) / 2, sectores[i].medio);
          return {
            x: tx,
            y: ty,
            titulo: porciones[i].nombre,
            filas: [
              {
                nombre: PORCENTAJE.format(porciones[i].valor / suma),
                valor: formatoNumero(porciones[i].valor),
                color: colores[i],
                marca: "barra" as const,
              },
            ],
            pista: seleccion?.puede(porciones[i].nombre) ? "Clic para filtrar por esto" : undefined,
          };
        })();

  return (
    <div ref={ref} className={`lienzo-grafico lienzo-torta ${ancho >= 460 ? "lienzo-torta-lado" : ""}`}>
      <div className="lienzo-torta-dibujo" style={{ width: lado, height: lado }}>
        <svg
          width={lado}
          height={lado}
          role="img"
          aria-labelledby={idTitulo}
          className="lienzo-svg"
          onPointerLeave={() => activo.alPuntero(null)}
          {...activo.propsTeclado}
        >
          <title id={idTitulo}>{`${titulo}. Use las flechas para recorrer las porciones.`}</title>
          <defs>
            <mask id={idMascara}>
              {/* Barrido: un trazo tan ancho como la torta que se va completando. */}
              <circle
                className="lienzo-barrido"
                cx={centro}
                cy={centro}
                r={externo / 2 + 1}
                fill="none"
                stroke="#fff"
                strokeWidth={externo + 4}
                pathLength={1}
                transform={`rotate(-90 ${centro} ${centro})`}
              />
            </mask>
          </defs>
          <g mask={`url(#${idMascara})`}>
            {porciones.map((p, k) => {
              const { a0, a1, medio } = sectores[k];
              // La porción activa se separa un poco del centro.
              const [dx, dy] = i === k ? punto(0, 0, 5, medio) : [0, 0];
              return (
                <path
                  key={p.nombre}
                  d={sector(centro, centro, externo, interno, a0, a1)}
                  fill={colores[k]}
                  className={`lienzo-porcion${i !== null && i !== k ? " lienzo-atenuada" : ""}${
                    seleccion?.puede(p.nombre) ? " lienzo-zona-seleccionable" : ""
                  }`}
                  transform={`translate(${dx} ${dy})`}
                  onPointerEnter={() => activo.alPuntero(k)}
                  onClick={() => seleccionar(k)}
                />
              );
            })}
          </g>
          {dona && (
            <g className="lienzo-torta-centro">
              <text x={centro} y={centro - 2} textAnchor="middle" className="lienzo-torta-total">
                {formatoNumero(total, true)}
              </text>
              <text x={centro} y={centro + 16} textAnchor="middle" className="lienzo-eje-texto">
                total
              </text>
            </g>
          )}
        </svg>
        {tooltip && <Tooltip {...tooltip} ancho={lado} />}
      </div>
      <Leyenda
        marca="barra"
        elementos={porciones.map((p, k) => ({
          nombre: p.nombre,
          color: colores[k],
          detalle: `${formatoNumero(p.valor)} · ${PORCENTAJE.format(p.valor / suma)}`,
        }))}
      />
    </div>
  );
}
