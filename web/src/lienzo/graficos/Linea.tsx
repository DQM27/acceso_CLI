import { useId, type PointerEvent } from "react";
import type { DatosGrafico } from "../calculo";
import { Tooltip } from "./comun";
import {
  formatoNumero,
  idSvg,
  marcasEje,
  pasoEtiquetas,
  recortar,
  useAncho,
  useIndiceActivo,
  useInterpolado,
} from "./utiles";

/**
 * Línea en SVG propio: una o varias series sobre un eje de fecha. Trazo de
 * 2 px, punto final con anillo del color del fondo, relleno suave opcional
 * (una sola serie). Una línea vertical sigue al puntero (o a las flechas) y el
 * tooltip lista todas las series en ese punto: no hace falta apuntarle a la línea.
 */
const ANCHO_CARACTER = 6.6;

export interface PropsLinea {
  datos: DatosGrafico;
  colores: string[];
  titulo: string;
  alto?: number;
  area?: boolean;
  etiquetar?: (clave: string, i: number) => string;
}

export default function GraficoLinea({ datos, colores, titulo, alto = 240, area = false, etiquetar = (c) => c }: PropsLinea) {
  const idTitulo = useId();
  const idDegradado = idSvg(useId());
  const { ref, ancho } = useAncho<HTMLDivElement>();
  const n = datos.categorias.length;
  const activo = useIndiceActivo(n);
  // Las marcas del eje salen de los valores finales.
  const { marcas, techo: techoFinal } = marcasEje(Math.max(...datos.series.flatMap((s) => s.valores), 0));
  // Lo que se dibuja: los valores en camino del dibujo anterior al nuevo, y el
  // techo del eje con ellos (si no, la línea vieja se sale del eje nuevo).
  const animados = useInterpolado(
    [...datos.series.flatMap((s) => datos.categorias.map((c) => `${s.nombre}\u0000${c}`)), "\u0000techo"],
    [...datos.series.flatMap((s) => s.valores), techoFinal],
  );

  if (n === 0 || datos.series.length === 0) return <p className="lienzo-vacio">Sin datos para mostrar.</p>;

  const techo = animados[animados.length - 1] || techoFinal;
  const dibujo = datos.series.map((_, k) => animados.slice(k * n, (k + 1) * n));
  // Mientras el techo crece, las marcas nuevas esperan a entrar en el dibujo.
  const marcasVisibles = marcas.filter((v) => v <= techo * 1.001);
  const etiquetas = datos.categorias.map((c, i) => etiquetar(c, i));
  const m = {
    arriba: 14,
    abajo: 26,
    izquierda: Math.max(...marcas.map((v) => formatoNumero(v, true).length)) * ANCHO_CARACTER + 10,
    derecha: 14,
  };
  const anchoUtil = Math.max(10, ancho - m.izquierda - m.derecha);
  const altoUtil = Math.max(10, alto - m.arriba - m.abajo);
  // Con un solo punto, al centro.
  const x = (i: number) => m.izquierda + (n === 1 ? anchoUtil / 2 : (anchoUtil * i) / (n - 1));
  const y = (v: number) => m.arriba + altoUtil - (v / techo) * altoUtil;
  const cadaCuanto = pasoEtiquetas(n, anchoUtil, Math.max(...etiquetas.map((e) => e.length)) * ANCHO_CARACTER + 12);
  const conArea = area && datos.series.length === 1;

  const trazo = (valores: number[]) => valores.map((v, i) => `${i === 0 ? "M" : "L"}${x(i)},${y(v)}`).join("");

  const alMover = (e: PointerEvent<SVGRectElement>) => {
    const caja = e.currentTarget.getBoundingClientRect();
    // La zona sobresale 8 px a cada lado del área de dibujo.
    const relativo = (e.clientX - caja.left - 8) / Math.max(caja.width - 16, 1);
    activo.alPuntero(Math.max(0, Math.min(n - 1, Math.round(relativo * (n - 1)))));
  };

  const i = activo.indice;
  const tooltip =
    i === null
      ? null
      : {
          x: x(i),
          y: y(Math.max(...datos.series.map((s) => s.valores[i] ?? 0))),
          titulo: etiquetas[i],
          filas: datos.series.map((s, k) => ({ nombre: s.nombre, valor: formatoNumero(s.valores[i] ?? 0), color: colores[k] })),
        };

  return (
    <div ref={ref} className="lienzo-grafico">
      <svg width={ancho} height={alto} role="img" aria-labelledby={idTitulo} className="lienzo-svg" {...activo.propsTeclado}>
        <title id={idTitulo}>{`${titulo}. Use las flechas para recorrer los valores.`}</title>
        {conArea && (
          <defs>
            <linearGradient id={idDegradado} x1="0" x2="0" y1="0" y2="1">
              <stop offset="0" stopColor={colores[0]} stopOpacity="0.18" />
              <stop offset="1" stopColor={colores[0]} stopOpacity="0.02" />
            </linearGradient>
          </defs>
        )}

        {marcasVisibles.map((v) => (
          <g key={v}>
            <line x1={m.izquierda} x2={ancho - m.derecha} y1={y(v)} y2={y(v)} className="lienzo-rejilla" />
            <text x={m.izquierda - 8} y={y(v) + 4} textAnchor="end" className="lienzo-eje-texto">
              {formatoNumero(v, true)}
            </text>
          </g>
        ))}
        <line x1={m.izquierda} x2={ancho - m.derecha} y1={y(0)} y2={y(0)} className="lienzo-base" />

        {etiquetas.map((texto, c) =>
          c % cadaCuanto !== 0 && c !== n - 1 ? null : (
            <text key={c} x={x(c)} y={alto - 8} textAnchor="middle" className="lienzo-eje-texto">
              {recortar(texto, 12)}
            </text>
          ),
        )}

        {/* Las series: se trazan al entrar; después, cada cambio de datos mueve los puntos a su valor nuevo. */}
        <g>
          {conArea && (
            <path
              className="lienzo-area"
              d={`${trazo(dibujo[0])}L${x(n - 1)},${y(0)}L${x(0)},${y(0)}Z`}
              fill={`url(#${idDegradado})`}
            />
          )}
          {datos.series.map((s, k) => (
            <path
              key={s.nombre}
              d={trazo(dibujo[k])}
              pathLength={1}
              className="lienzo-linea"
              stroke={colores[k]}
              style={{ animationDelay: `${k * 90}ms` }}
            />
          ))}
          {datos.series.map((s, k) => (
            <circle
              key={s.nombre}
              cx={x(n - 1)}
              cy={y(dibujo[k][n - 1] ?? 0)}
              r={4}
              fill={colores[k]}
              className="lienzo-punto-final"
            />
          ))}
        </g>

        {i !== null && (
          <g className="lienzo-cursor">
            <line x1={x(i)} x2={x(i)} y1={m.arriba} y2={m.arriba + altoUtil} />
            {datos.series.map((s, k) => (
              <circle key={s.nombre} cx={x(i)} cy={y(s.valores[i] ?? 0)} r={4} fill={colores[k]} className="lienzo-punto-cursor" />
            ))}
          </g>
        )}

        <rect
          className="lienzo-zona"
          x={m.izquierda - 8}
          y={m.arriba}
          width={anchoUtil + 16}
          height={altoUtil}
          onPointerMove={alMover}
          onPointerLeave={() => activo.alPuntero(null)}
        />
      </svg>
      {tooltip && <Tooltip {...tooltip} ancho={ancho} />}
    </div>
  );
}
