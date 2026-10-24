import { useEffect, useState, type ReactNode } from "react";
import type { Origen } from "./catalogo";
import type { Fila } from "./calculo";
import type { WidgetAnalisis } from "./esquema";
import { ajustarTipos, calcularConPython, escucharEstadoPython, type EstadoPython } from "./motorPython";
import { columnasParaSandbox } from "./sandbox";

/**
 * Panel "analisis": el cálculo en Python que escribió el asistente (corre en el
 * sandbox, ver motorPython.ts) y su resultado dibujado con un panel común
 * (`dibujar`, que pone Lienzo.tsx). Mientras calcula muestra en qué anda; la
 * primera vez de la visita, Python tiene que arrancar.
 */
type Resultado = { codigo: string; filas: Fila[]; datos: Fila[] } | { codigo: string; error: string; datos: Fila[] };

export default function AnalisisPython({
  widget,
  filas,
  origen,
  dibujar,
}: {
  widget: WidgetAnalisis;
  filas: Fila[];
  origen: Origen;
  dibujar: (resultado: Fila[]) => ReactNode;
}) {
  const [motor, setMotor] = useState<EstadoPython>("apagado");
  const [resultado, setResultado] = useState<Resultado | null>(null);
  useEffect(() => escucharEstadoPython(setMotor), []);

  const { codigo, columnas } = widget;
  useEffect(() => {
    let vigente = true;
    calcularConPython({
      codigo,
      datos: filas,
      columnas: columnasParaSandbox(origen),
      salida: columnas.map((c) => c.nombre),
    })
      .then((salida) => vigente && setResultado({ codigo, datos: filas, filas: ajustarTipos(salida, columnas) }))
      .catch((error: unknown) => vigente && setResultado({ codigo, datos: filas, error: error instanceof Error ? error.message : String(error) }));
    return () => {
      vigente = false;
    };
  }, [codigo, columnas, filas, origen]);

  // Un resultado de otro código o de otras filas (cambió un filtro) ya no vale: se calcula de nuevo.
  const vigente = resultado && resultado.codigo === codigo && resultado.datos === filas ? resultado : null;
  if (vigente && "filas" in vigente) return dibujar(vigente.filas);

  return (
    <section className="tarjeta lienzo-bloque lienzo-analisis" aria-busy={!vigente}>
      <header className="lienzo-bloque-cabecera">
        <h3>{widget.titulo}</h3>
        <span className="lienzo-libre-marca" title="Calculado con Python, en un espacio aislado">
          Python
        </span>
      </header>
      {widget.descripcion && <p className="lienzo-libre-descripcion">{widget.descripcion}</p>}
      {vigente && "error" in vigente ? (
        <p className="asistente-error" role="alert">
          El análisis falló: {vigente.error}
        </p>
      ) : (
        <p className="lienzo-analisis-estado" role="status">
          <span className="asistente-pensando" aria-hidden="true">
            <span />
            <span />
            <span />
          </span>
          {motor === "cargando"
            ? "Preparando Python… la primera vez tarda unos segundos."
            : "Calculando…"}
        </p>
      )}
    </section>
  );
}
