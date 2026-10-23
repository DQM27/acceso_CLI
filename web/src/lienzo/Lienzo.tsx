import { useState, type CSSProperties, type ReactNode } from "react";
import { CATALOGO, definicionCampo, type AgrupacionFecha, type Ancho, type Fuente } from "./catalogo";
import {
  calcularKpi,
  calcularPorciones,
  calcularSeries,
  calcularTabla,
  etiquetaFecha,
  formatearFecha,
  type DatosGrafico,
  type Fila,
} from "./calculo";
import { COLOR_UNICO, RegistroColores } from "./colores";
import type { Lienzo, Widget, WidgetBarras, WidgetLinea } from "./esquema";
import GraficoBarras from "./graficos/Barras";
import { Leyenda, MarcoGrafico, type TablaDeDatos } from "./graficos/comun";
import GraficoLinea from "./graficos/Linea";
import GraficoTorta from "./graficos/Torta";
import { TablaLienzo, TarjetaKpi } from "./widgets";

/**
 * Renderizador del lienzo: dibuja un documento ya validado (`validarLienzo`)
 * con componentes propios. No ejecuta nada que venga en el documento: cada
 * widget es un dato que se lee, no código.
 *
 * Disposición: "cuadricula" (12 columnas; cada panel con su `ancho`) o
 * "enfoque" (un panel debajo del otro, grandes y al centro).
 */
const ANCHO_POR_OMISION: Record<Widget["tipo"], Ancho> = {
  kpi: "cuarto",
  barras: "mitad",
  linea: "mitad",
  torta: "tercio",
  tabla: "completo",
};

const TEXTO_AGRUPACION: Record<AgrupacionFecha, string> = { dia: "por día", semana: "por semana", mes: "por mes" };

function etiquetaCampo(fuente: Fuente, campo: string): string {
  return definicionCampo(fuente, campo)?.etiqueta ?? campo;
}

/** "Contratistas", "Empresas distintas"… lo que mide un gráfico, dicho en palabras. */
function textoMedida(fuente: Fuente, medida: string, campo?: string): string {
  return medida === "contar_distintos" && campo
    ? `${etiquetaCampo(fuente, campo)} (distintos)`
    : CATALOGO[fuente].etiqueta;
}

/** Colores estables por entidad, uno por cada campo (empresa, sitio…). */
function useColores() {
  const [registros] = useState(() => new Map<string, RegistroColores>());
  return (campo: string, nombres: string[]) => {
    let registro = registros.get(campo);
    if (!registro) {
      registro = new RegistroColores();
      registros.set(campo, registro);
    }
    return registro.asignar(nombres);
  };
}

function tablaDeSeries(datos: DatosGrafico, columnaEje: string, etiquetar: (c: string) => string): TablaDeDatos {
  return {
    columnas: [columnaEje, ...datos.series.map((s) => s.nombre)],
    filas: datos.categorias.map((c, i) => [etiquetar(c), ...datos.series.map((s) => s.valores[i] ?? 0)]),
  };
}

function GraficoDeSeries({
  lienzo,
  widget,
  filas,
  colores,
  enfoque,
}: {
  lienzo: Lienzo;
  widget: WidgetBarras | WidgetLinea;
  filas: Fila[];
  colores: (campo: string, nombres: string[]) => string[];
  enfoque: boolean;
}) {
  const medida = textoMedida(lienzo.fuente, widget.medida, widget.campo);
  const datos = calcularSeries(lienzo.fuente, widget, filas, medida);
  const eje = etiquetaCampo(lienzo.fuente, widget.eje_x);
  const etiquetar = (c: string) =>
    datos.agrupacion ? etiquetaFecha(c, datos.agrupacion, lienzo.formato?.fecha) : c;
  const unica = !widget.serie;
  const paleta = unica ? [COLOR_UNICO] : colores(widget.serie ?? "", datos.series.map((s) => s.nombre));
  const titulo = widget.titulo ?? `${medida} por ${eje.toLowerCase()}`;
  const subtitulo = [
    datos.agrupacion ? TEXTO_AGRUPACION[datos.agrupacion] : null,
    widget.serie ? `por ${etiquetaCampo(lienzo.fuente, widget.serie).toLowerCase()}` : null,
  ]
    .filter(Boolean)
    .join(" · ");
  const alto = enfoque ? 380 : 240;

  return (
    <MarcoGrafico titulo={titulo} subtitulo={subtitulo || undefined} datos={tablaDeSeries(datos, eje, etiquetar)}>
      {widget.tipo === "barras" ? (
        <GraficoBarras
          datos={datos}
          colores={paleta}
          titulo={titulo}
          alto={alto}
          horizontal={widget.orientacion ? widget.orientacion === "horizontal" : !datos.agrupacion}
          etiquetar={etiquetar}
        />
      ) : (
        <GraficoLinea datos={datos} colores={paleta} titulo={titulo} alto={alto} area={widget.area} etiquetar={etiquetar} />
      )}
      {!unica && datos.series.length > 1 && (
        <Leyenda
          marca={widget.tipo === "linea" ? "linea" : "barra"}
          elementos={datos.series.map((s, k) => ({ nombre: s.nombre, color: paleta[k] }))}
        />
      )}
    </MarcoGrafico>
  );
}

function WidgetLienzo({
  lienzo,
  widget,
  filas,
  colores,
  enfoque,
}: {
  lienzo: Lienzo;
  widget: Widget;
  filas: Fila[];
  colores: (campo: string, nombres: string[]) => string[];
  enfoque: boolean;
}): ReactNode {
  switch (widget.tipo) {
    case "kpi":
    {
      const valor = calcularKpi(widget, filas);
      const esFecha = widget.campo && definicionCampo(lienzo.fuente, widget.campo)?.tipo === "fecha";
      return (
        <TarjetaKpi
          valor={esFecha && typeof valor === "string" ? formatearFecha(valor, lienzo.formato?.fecha) : valor}
          titulo={widget.titulo}
          tono={widget.tono}
        />
      );
    }
    case "barras":
    case "linea":
      return <GraficoDeSeries lienzo={lienzo} widget={widget} filas={filas} colores={colores} enfoque={enfoque} />;
    case "torta": {
      const calculadas = calcularPorciones(widget, filas);
      const total = calculadas.total;
      const porciones =
        definicionCampo(lienzo.fuente, widget.categoria)?.tipo === "fecha"
          ? calculadas.porciones.map((p) => ({ ...p, nombre: formatearFecha(p.nombre, lienzo.formato?.fecha) }))
          : calculadas.porciones;
      const categoria = etiquetaCampo(lienzo.fuente, widget.categoria);
      const medida = textoMedida(lienzo.fuente, widget.medida, widget.campo);
      const titulo = widget.titulo ?? `${medida} por ${categoria.toLowerCase()}`;
      const paleta = colores(widget.categoria, porciones.map((p) => p.nombre));
      const suma = porciones.reduce((s, p) => s + p.valor, 0) || 1;
      return (
        <MarcoGrafico
          titulo={titulo}
          datos={{
            columnas: [categoria, medida, "Porcentaje"],
            filas: porciones.map((p) => [p.nombre, p.valor, `${((p.valor / suma) * 100).toFixed(1)} %`]),
          }}
        >
          <GraficoTorta
            porciones={porciones}
            total={total}
            colores={paleta}
            titulo={titulo}
            dona={widget.dona ?? true}
            tamano={enfoque ? 380 : 260}
          />
        </MarcoGrafico>
      );
    }
    case "tabla":
      return (
        <section className="tarjeta lienzo-bloque">
          {widget.titulo && (
            <header className="lienzo-bloque-cabecera">
              <h3>{widget.titulo}</h3>
            </header>
          )}
          <TablaLienzo tabla={calcularTabla(lienzo.fuente, widget, filas, lienzo.formato?.fecha)} />
        </section>
      );
  }
}

export default function LienzoVista({ lienzo, filas }: { lienzo: Lienzo; filas: Fila[] }) {
  const colores = useColores();
  const enfoque = lienzo.disposicion === "enfoque";
  return (
    <div className={`lienzo-documento ${enfoque ? "lienzo-enfoque" : "lienzo-cuadricula"}`} data-fuente={lienzo.fuente}>
      {lienzo.titulo && <h2 className="lienzo-titulo">{lienzo.titulo}</h2>}
      {lienzo.widgets.map((widget, i) => (
        <div
          key={i}
          className={`lienzo-panel lienzo-ancho-${enfoque ? "completo" : (widget.ancho ?? ANCHO_POR_OMISION[widget.tipo])}`}
          style={{ "--i": i } as CSSProperties}
        >
          <WidgetLienzo lienzo={lienzo} widget={widget} filas={filas} colores={colores} enfoque={enfoque} />
        </div>
      ))}
    </div>
  );
}
