import { useId } from "react";
import type { DatosGrafico } from "../calculo";
import { Tooltip } from "./comun";
import {
  formatoNumero,
  marcasEje,
  pasoEtiquetas,
  recortar,
  retraso,
  useAncho,
  useIndiceActivo,
  useInterpolado,
  type Seleccion,
} from "./utiles";

/**
 * Barras en SVG propio: verticales (columnas) u horizontales, de una serie o
 * apiladas. Barras finas (≤ 24 px), extremo de dato redondeado y base recta,
 * 2 px de aire entre segmentos apilados, cuadrícula tenue. Cada categoría
 * responde al puntero y al teclado con un tooltip que lista todas sus series.
 */
const RADIO = 4;
const SEPARACION = 2;
const GROSOR_MAXIMO = 24;
const ANCHO_CARACTER = 6.6;
/** Las mayúsculas (nombres y empresas vienen así) ocupan más que el promedio. */
const ANCHO_MAYUSCULA = 8.2;

/** Ancho aproximado de una etiqueta del eje, en píxeles. */
function anchoTexto(texto: string): number {
  let ancho = 0;
  for (const letra of texto) ancho += letra !== letra.toLowerCase() ? ANCHO_MAYUSCULA : ANCHO_CARACTER;
  return ancho;
}

/** La etiqueta entera si entra en `disponible` píxeles; si no, recortada con "…". */
function ajustarAlAncho(texto: string, disponible: number): string {
  if (anchoTexto(texto) <= disponible) return texto;
  let corte = texto.length - 1;
  while (corte > 1 && anchoTexto(texto.slice(0, corte)) + ANCHO_CARACTER > disponible) corte--;
  return `${texto.slice(0, corte)}…`;
}

/** Rectángulo con sólo el extremo del dato redondeado (arriba o a la derecha). */
function barra(x: number, y: number, w: number, h: number, redondeada: boolean, horizontal: boolean): string {
  if (w <= 0 || h <= 0) return "";
  if (!redondeada) return `M${x},${y}h${w}v${h}h${-w}Z`;
  if (horizontal) {
    const r = Math.min(RADIO, w, h / 2);
    return `M${x},${y}h${w - r}q${r},0 ${r},${r}v${h - 2 * r}q0,${r} ${-r},${r}h${-(w - r)}Z`;
  }
  const r = Math.min(RADIO, h, w / 2);
  return `M${x},${y + h}v${-(h - r)}q0,${-r} ${r},${-r}h${w - 2 * r}q${r},0 ${r},${r}v${h - r}Z`;
}

export interface PropsBarras {
  datos: DatosGrafico;
  /** Color de cada serie, en el orden de `datos.series`. */
  colores: string[];
  titulo: string;
  horizontal?: boolean;
  alto?: number;
  /** Pinta más suave la última barra (el día en curso, que todavía no termina). */
  resaltarUltima?: boolean;
  /** Texto del eje para cada categoría. */
  etiquetar?: (clave: string, i: number) => string;
  /** Clic (o Enter) sobre una categoría: profundizar en ella. */
  seleccion?: Seleccion;
}

export default function GraficoBarras({
  datos,
  colores,
  titulo,
  horizontal = false,
  alto,
  resaltarUltima = false,
  etiquetar = (c) => c,
  seleccion,
}: PropsBarras) {
  const idTitulo = useId();
  const { ref, ancho } = useAncho<HTMLDivElement>();
  const n = datos.categorias.length;
  const seleccionar = (c: number) => {
    const clave = datos.categorias[c];
    if (seleccion?.puede(clave)) seleccion.alSeleccionar(clave);
  };
  const activo = useIndiceActivo(n, seleccionar);
  // Las marcas del eje salen de los valores finales.
  const totales = datos.categorias.map((_, i) => datos.series.reduce((s, serie) => s + (serie.valores[i] ?? 0), 0));
  const { marcas, techo: techoFinal } = marcasEje(Math.max(...totales, 0));
  // Lo que se dibuja: los valores en camino del dibujo anterior al nuevo, y el
  // techo del eje con ellos (si no, las barras viejas se salen del eje nuevo).
  const animados = useInterpolado(
    [...datos.series.flatMap((s) => datos.categorias.map((c) => `${s.nombre}\u0000${c}`)), "\u0000techo"],
    [...datos.series.flatMap((s) => s.valores), techoFinal],
  );

  if (n === 0 || datos.series.length === 0) return <p className="lienzo-vacio">Sin datos para mostrar.</p>;

  const techo = animados[animados.length - 1] || techoFinal;
  const dibujo = datos.series.map((_, k) => animados.slice(k * n, (k + 1) * n));
  const totalesDibujo = datos.categorias.map((_, c) => dibujo.reduce((s, valores) => s + (valores[c] ?? 0), 0));
  const etiquetas = datos.categorias.map((c, i) => etiquetar(c, i));
  const varias = datos.series.length > 1;
  const anchoMarca = Math.max(...marcas.map((m) => formatoNumero(m, true).length)) * ANCHO_CARACTER + 10;

  // Geometría según la orientación.
  const m = horizontal
    ? {
        arriba: 4,
        abajo: 22,
        izquierda: Math.min(ancho * 0.38, Math.max(...etiquetas.map(anchoTexto)) + 14),
        derecha: 52,
      }
    : { arriba: 14, abajo: 26, izquierda: anchoMarca, derecha: 8 };
  const alto_ = horizontal ? n * 32 + m.arriba + m.abajo : (alto ?? 240);
  const anchoUtil = Math.max(10, ancho - m.izquierda - m.derecha);
  const altoUtil = Math.max(10, alto_ - m.arriba - m.abajo);
  const banda = (horizontal ? altoUtil : anchoUtil) / n;
  const grosor = Math.min(GROSOR_MAXIMO, banda * (horizontal ? 0.62 : 0.68));
  const escala = (v: number) => (v / techo) * (horizontal ? anchoUtil : altoUtil);
  const cadaCuanto = horizontal ? 1 : pasoEtiquetas(n, anchoUtil, Math.max(...etiquetas.map((e) => e.length)) * ANCHO_CARACTER + 10);

  const i = activo.indice;
  const tooltip =
    i === null
      ? null
      : {
          x: horizontal ? m.izquierda + escala(totales[i]) : m.izquierda + banda * i + banda / 2,
          y: horizontal ? m.arriba + banda * i + banda / 2 : m.arriba + altoUtil - escala(totales[i]),
          titulo: etiquetas[i],
          filas: [
            ...datos.series.map((s, k) => ({
              nombre: s.nombre,
              valor: formatoNumero(s.valores[i] ?? 0),
              color: colores[k],
              marca: "barra" as const,
            })),
            ...(varias ? [{ nombre: "Total", valor: formatoNumero(totales[i]) }] : []),
          ],
          pista: seleccion?.puede(datos.categorias[i]) ? "Clic para filtrar por esto" : undefined,
        };

  return (
    <div ref={ref} className="lienzo-grafico">
      <svg
        width={ancho}
        height={alto_}
        role="img"
        aria-labelledby={idTitulo}
        className="lienzo-svg"
        onPointerLeave={() => activo.alPuntero(null)}
        {...activo.propsTeclado}
      >
        <title id={idTitulo}>{`${titulo}. Use las flechas para recorrer los valores.`}</title>

        {/* Cuadrícula y eje de valores. */}
        {marcas.map((v) => {
          // Mientras el techo crece, las marcas nuevas esperan a entrar en el dibujo.
          if (v > techo * 1.001) return null;
          const pos = horizontal ? m.izquierda + escala(v) : m.arriba + altoUtil - escala(v);
          return horizontal ? (
            <g key={v}>
              <line x1={pos} x2={pos} y1={m.arriba} y2={m.arriba + altoUtil} className="lienzo-rejilla" />
              <text x={pos} y={alto_ - 6} textAnchor="middle" className="lienzo-eje-texto">
                {formatoNumero(v, true)}
              </text>
            </g>
          ) : (
            <g key={v}>
              <line x1={m.izquierda} x2={ancho - m.derecha} y1={pos} y2={pos} className="lienzo-rejilla" />
              <text x={m.izquierda - 8} y={pos + 4} textAnchor="end" className="lienzo-eje-texto">
                {formatoNumero(v, true)}
              </text>
            </g>
          );
        })}

        {/* Barras: entran creciendo; después, cada cambio de datos las mueve de su valor anterior al nuevo. */}
        <g>
          {datos.categorias.map((clave, c) => {
            const inicioBanda = (horizontal ? m.arriba : m.izquierda) + banda * c;
            const centro = inicioBanda + (banda - grosor) / 2;
            const ultimoConValor = dibujo.reduce((u, valores, k) => ((valores[c] ?? 0) > 0 ? k : u), -1);
            let acumulado = 0;
            const atenuada = i !== null && i !== c;
            return (
              <g key={clave} className={`lienzo-categoria${atenuada ? " lienzo-atenuada" : ""}`}>
                <g
                  className={horizontal ? "lienzo-crecer-h" : "lienzo-crecer-v"}
                  style={{ animationDelay: retraso(c, n) }}
                >
                {datos.series.map((s, k) => {
                  const valor = dibujo[k][c] ?? 0;
                  if (valor <= 0) return null;
                  const largoTotal = escala(valor);
                  // Aire entre segmentos apilados: cada uno, salvo el de la base, cede 2 px.
                  const largo = acumulado > 0 ? Math.max(0, largoTotal - SEPARACION) : largoTotal;
                  const desplazamiento = acumulado > 0 ? acumulado + SEPARACION : acumulado;
                  acumulado += largoTotal;
                  const hoy = resaltarUltima && c === n - 1;
                  const d = horizontal
                    ? barra(m.izquierda + desplazamiento, centro, largo, grosor, k === ultimoConValor, true)
                    : barra(centro, m.arriba + altoUtil - desplazamiento - largo, grosor, largo, k === ultimoConValor, false);
                  return <path key={s.nombre} d={d} fill={colores[k]} className={hoy ? "lienzo-barra-hoy" : undefined} />;
                })}
                </g>
                {/* Valor en la punta (barras horizontales, donde siempre hay lugar). */}
                {horizontal && (
                  <text
                    x={m.izquierda + escala(totalesDibujo[c]) + 6}
                    y={centro + grosor / 2 + 4}
                    className="lienzo-valor-texto"
                    style={{ animationDelay: retraso(c, n, 700) }}
                  >
                    {formatoNumero(totales[c])}
                  </text>
                )}
              </g>
            );
          })}
        </g>

        {/* Etiquetas del eje de categorías. */}
        {etiquetas.map((texto, c) =>
          c % cadaCuanto !== 0 && c !== n - 1 ? null : horizontal ? (
            <text
              key={c}
              x={m.izquierda - 8}
              y={m.arriba + banda * c + banda / 2 + 4}
              textAnchor="end"
              className="lienzo-eje-texto lienzo-eje-categoria"
            >
              {ajustarAlAncho(texto, m.izquierda - 14)}
            </text>
          ) : (
            <text
              key={c}
              x={m.izquierda + banda * c + banda / 2}
              y={alto_ - 8}
              textAnchor="middle"
              className="lienzo-eje-texto"
            >
              {recortar(texto, 12)}
            </text>
          ),
        )}
        <line
          className="lienzo-base"
          {...(horizontal
            ? { x1: m.izquierda, x2: m.izquierda, y1: m.arriba, y2: m.arriba + altoUtil }
            : { x1: m.izquierda, x2: ancho - m.derecha, y1: m.arriba + altoUtil, y2: m.arriba + altoUtil })}
        />

        {/* Zonas de puntero: la banda entera, más grande que la barra. */}
        {datos.categorias.map((clave, c) => (
          <rect
            key={clave}
            className={`lienzo-zona${seleccion?.puede(clave) ? " lienzo-zona-seleccionable" : ""}`}
            {...(horizontal
              ? { x: 0, y: m.arriba + banda * c, width: ancho, height: banda }
              : { x: m.izquierda + banda * c, y: m.arriba, width: banda, height: altoUtil })}
            onPointerEnter={() => activo.alPuntero(c)}
            onClick={() => seleccionar(c)}
          />
        ))}
      </svg>
      {tooltip && <Tooltip {...tooltip} ancho={ancho} />}
    </div>
  );
}
