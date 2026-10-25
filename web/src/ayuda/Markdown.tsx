import type { ReactNode } from "react";
import {
  type Bloque,
  type EnLinea,
  leerEnLinea,
} from "../../../supabase/functions/_shared/manual/markdown.ts";
import {
  type DestinoEnlace,
  resolverEnlace,
} from "../../../supabase/functions/_shared/manual/manual.ts";

/** Prefijo del `id` de cada título, para saltar a una sección por su ancla. */
export const PREFIJO_ANCLA = "ayuda-";

interface Props {
  bloques: Bloque[];
  /** Ruta del proceso que se está mostrando, para resolver sus enlaces. */
  ruta: string;
  onEnlace: (destino: DestinoEnlace) => void;
}

/** Dibuja un proceso del manual (lo que lee `leerMarkdown`). Igual al del
 * escritorio (`desktop/src/ayuda/Markdown.tsx`): son paquetes separados. */
export default function Markdown({ bloques, ruta, onEnlace }: Props) {
  const enLinea = (texto: string) => <EnLineaVista partes={leerEnLinea(texto)} ruta={ruta} onEnlace={onEnlace} />;

  const dibujar = (lista: Bloque[]): ReactNode[] =>
    lista.map((bloque, i) => {
      switch (bloque.tipo) {
        case "titulo": {
          const Etiqueta = (["h1", "h2", "h3", "h4"] as const)[Math.min(bloque.nivel, 4) - 1];
          return (
            <Etiqueta key={i} id={PREFIJO_ANCLA + bloque.ancla}>
              {enLinea(bloque.texto)}
            </Etiqueta>
          );
        }
        case "parrafo":
          return <p key={i}>{enLinea(bloque.texto)}</p>;
        case "lista": {
          const elementos = bloque.elementos.map((elemento, j) => (
            <li key={j}>
              {enLinea(elemento.texto)}
              {dibujar(elemento.hijos)}
            </li>
          ));
          return bloque.ordenada ? (
            <ol key={i} start={bloque.inicio}>
              {elementos}
            </ol>
          ) : (
            <ul key={i}>{elementos}</ul>
          );
        }
        case "tabla":
          return (
            <div key={i} className="ayuda-tabla">
              <table>
                <thead>
                  <tr>
                    {bloque.encabezado.map((celda, j) => (
                      <th key={j}>{enLinea(celda)}</th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {bloque.filas.map((fila, j) => (
                    <tr key={j}>
                      {fila.map((celda, k) => (
                        <td key={k}>{enLinea(celda)}</td>
                      ))}
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          );
      }
    });

  return <div className="ayuda-contenido">{dibujar(bloques)}</div>;
}

function EnLineaVista({
  partes,
  ruta,
  onEnlace,
}: {
  partes: EnLinea[];
  ruta: string;
  onEnlace: (destino: DestinoEnlace) => void;
}) {
  return (
    <>
      {partes.map((parte, i) => {
        switch (parte.tipo) {
          case "texto":
            return parte.texto;
          case "codigo":
            return <code key={i}>{parte.texto}</code>;
          case "negrita":
            return (
              <strong key={i}>
                <EnLineaVista partes={parte.hijos} ruta={ruta} onEnlace={onEnlace} />
              </strong>
            );
          case "enlace": {
            const destino = resolverEnlace(ruta, parte.destino);
            const hijos = <EnLineaVista partes={parte.hijos} ruta={ruta} onEnlace={onEnlace} />;
            // Sólo se navega dentro del manual: una dirección de afuera
            // abriría otra página dentro de la ventana del programa.
            if (!destino) return <span key={i}>{hijos}</span>;
            return (
              <a
                key={i}
                href={`#${destino.ruta}`}
                onClick={(evento) => {
                  evento.preventDefault();
                  onEnlace(destino);
                }}
              >
                {hijos}
              </a>
            );
          }
        }
      })}
    </>
  );
}
