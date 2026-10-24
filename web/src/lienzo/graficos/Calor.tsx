import { useId } from "react";
import { nivel, rangosDeNiveles, type DatosCalor } from "../mapas";
import { Leyenda, Tooltip } from "./comun";
import { formatoNumero, recortar, retraso, useAncho, useIndiceActivo } from "./utiles";

/**
 * Mapa de calor en SVG propio: una celda por fila × columna, con cinco tonos de
 * un solo color (más oscuro = más) y las vacías en gris tenue. 2 px de aire
 * entre celdas, número adentro cuando entra, tooltip con puntero o teclado y
 * clic para filtrar por esa fila y columna.
 */
const ALTO_CELDA = 28;
const SEPARACION = 2;
const ANCHO_CARACTER = 7;
const ANCHO_MINIMO_CELDA = 12;
/** Las celdas se estiran para llenar el ancho de la tarjeta, hasta este tope (con pocas columnas). */
const ANCHO_MAXIMO_CELDA = 120;
/** Con celdas más angostas que esto, el número no entra: queda en el tooltip y en la tabla. */
const ANCHO_PARA_NUMERO = 26;

export default function GraficoCalor({
  datos,
  titulo,
  medida,
  alSeleccionar,
}: {
  datos: DatosCalor;
  titulo: string;
  /** Qué se cuenta, para el tooltip ("Ingresos"). */
  medida: string;
  /** Clic (o Enter) sobre una celda con valor: profundizar en esa fila y columna. */
  alSeleccionar?: (fila: string, columna: string) => void;
}) {
  const idTitulo = useId();
  const { ref, ancho } = useAncho<HTMLDivElement>();
  const nf = datos.filas.length;
  const nc = datos.columnas.length;
  const total = nf * nc;
  const seleccionar = (indice: number) => {
    const f = Math.floor(indice / nc);
    const c = indice % nc;
    if (alSeleccionar && datos.valores[f][c] > 0) alSeleccionar(datos.filas[f].clave, datos.columnas[c].clave);
  };
  const activo = useIndiceActivo(total, seleccionar);

  if (nf === 0 || nc === 0) return <p className="lienzo-vacio">Sin datos para mostrar.</p>;

  const izquierda = Math.min(ancho * 0.3, Math.max(...datos.filas.map((f) => f.etiqueta.length)) * ANCHO_CARACTER + 12);
  const arriba = 22;
  const celda = Math.max(ANCHO_MINIMO_CELDA, Math.min(ANCHO_MAXIMO_CELDA, (ancho - izquierda) / nc));
  const anchoDibujo = izquierda + celda * nc;
  const alto = arriba + ALTO_CELDA * nf + 4;
  // Una etiqueta de columna cada tanto, para que no se pisen.
  const anchoEtiqueta = Math.max(...datos.columnas.map((c) => c.etiqueta.length)) * ANCHO_CARACTER + 6;
  const cadaCuanto = Math.max(1, Math.ceil(anchoEtiqueta / celda));
  const conNumero = celda >= ANCHO_PARA_NUMERO;

  const i = activo.indice;
  const tooltip =
    i === null
      ? null
      : (() => {
          const f = Math.floor(i / nc);
          const c = i % nc;
          return {
            x: izquierda + celda * c + celda / 2,
            y: arriba + ALTO_CELDA * f + ALTO_CELDA / 2,
            titulo: `${datos.filas[f].etiqueta} · ${datos.columnas[c].etiqueta}`,
            filas: [{ nombre: medida, valor: formatoNumero(datos.valores[f][c]) }],
            pista: alSeleccionar && datos.valores[f][c] > 0 ? "Clic para filtrar por esto" : undefined,
          };
        })();

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
        <title id={idTitulo}>{`${titulo}. Use las flechas para recorrer las celdas.`}</title>

        {datos.columnas.map((columna, c) =>
          c % cadaCuanto !== 0 ? null : (
            <text key={columna.clave} x={izquierda + celda * c + celda / 2} y={14} textAnchor="middle" className="lienzo-eje-texto">
              {recortar(columna.etiqueta, Math.max(2, Math.floor((celda * cadaCuanto) / ANCHO_CARACTER)))}
            </text>
          ),
        )}
        {datos.filas.map((fila, f) => (
          <text
            key={fila.clave}
            x={izquierda - 8}
            y={arriba + ALTO_CELDA * f + ALTO_CELDA / 2 + 4}
            textAnchor="end"
            className="lienzo-eje-texto lienzo-eje-categoria"
          >
            {recortar(fila.etiqueta, Math.max(3, Math.floor((izquierda - 12) / ANCHO_CARACTER)))}
          </text>
        ))}

        {datos.filas.map((fila, f) =>
          datos.columnas.map((columna, c) => {
            const valor = datos.valores[f][c];
            const n = nivel(valor, datos.maximo);
            const x = izquierda + celda * c;
            const y = arriba + ALTO_CELDA * f;
            const indice = f * nc + c;
            const atenuada = i !== null && i !== indice;
            return (
              <g
                key={`${fila.clave}\u0000${columna.clave}`}
                className={`lienzo-calor-celda${atenuada ? " lienzo-atenuada" : ""}${alSeleccionar && valor > 0 ? " lienzo-zona-seleccionable" : ""}`}
                style={{ animationDelay: retraso(indice, total, 500) }}
                onPointerEnter={() => activo.alPuntero(indice)}
                onClick={() => seleccionar(indice)}
              >
                <rect
                  x={x + SEPARACION / 2}
                  y={y + SEPARACION / 2}
                  width={Math.max(1, celda - SEPARACION)}
                  height={ALTO_CELDA - SEPARACION}
                  rx={4}
                  className={`lienzo-calor-n${n}`}
                />
                {conNumero && valor > 0 && (
                  <text x={x + celda / 2} y={y + ALTO_CELDA / 2 + 4} textAnchor="middle" className={`lienzo-calor-numero lienzo-calor-numero-n${n}`}>
                    {formatoNumero(valor, true)}
                  </text>
                )}
              </g>
            );
          }),
        )}
      </svg>
      <Leyenda
        marca="barra"
        elementos={rangosDeNiveles(datos.maximo).map(({ nivel: n, texto }) => ({
          nombre: texto,
          color: `var(--calor-${n})`,
        }))}
      />
      {tooltip && <Tooltip {...tooltip} ancho={ancho} />}
    </div>
  );
}
