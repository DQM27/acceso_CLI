import { CATALOGO, definicionCampo } from "./catalogo";
import { calcularBarras, calcularKpi, calcularTabla, type Fila } from "./calculo";
import type { Lienzo, Widget } from "./esquema";
import { GraficoBarras, TablaLienzo, TarjetaKpi } from "./widgets";

/**
 * Renderizador del lienzo: dibuja un documento ya validado (`validarLienzo`)
 * con componentes propios. No ejecuta nada que venga en el documento: cada
 * widget es un dato que se lee, no código.
 */
function WidgetLienzo({ lienzo, widget, filas }: { lienzo: Lienzo; widget: Widget; filas: Fila[] }) {
  if (widget.tipo === "kpi") {
    return <TarjetaKpi valor={calcularKpi(widget, filas)} titulo={widget.titulo} />;
  }
  if (widget.tipo === "barras") {
    const campo = definicionCampo(lienzo.fuente, widget.eje_x)?.etiqueta ?? widget.eje_x;
    const titulo = widget.titulo ?? `${CATALOGO[lienzo.fuente].etiqueta} por ${campo.toLowerCase()}`;
    return (
      <section className="tarjeta lienzo-bloque">
        <h3>{titulo}</h3>
        <GraficoBarras barras={calcularBarras(lienzo.fuente, widget, filas)} titulo={titulo} />
      </section>
    );
  }
  return (
    <section className="tarjeta lienzo-bloque">
      {widget.titulo && <h3>{widget.titulo}</h3>}
      <TablaLienzo tabla={calcularTabla(lienzo.fuente, widget, filas)} />
    </section>
  );
}

export default function LienzoVista({ lienzo, filas }: { lienzo: Lienzo; filas: Fila[] }) {
  const kpis = lienzo.widgets.filter((w) => w.tipo === "kpi");
  const bloques = lienzo.widgets.filter((w) => w.tipo !== "kpi");
  return (
    <div className="lienzo-documento" data-fuente={lienzo.fuente}>
      {kpis.length > 0 && (
        <div className="lienzo-kpis">
          {kpis.map((widget, i) => (
            <WidgetLienzo key={i} lienzo={lienzo} widget={widget} filas={filas} />
          ))}
        </div>
      )}
      {bloques.map((widget, i) => (
        <WidgetLienzo key={i} lienzo={lienzo} widget={widget} filas={filas} />
      ))}
    </div>
  );
}
