import { useState, type CSSProperties, type ReactNode } from "react";
import { definicionCampo, etiquetaOrigen, type AgrupacionFecha, type Ancho, type Origen } from "./catalogo";
import {
  calcularKpi,
  calcularPorciones,
  calcularSeries,
  calcularTabla,
  calcularTendencia,
  diasEntre,
  etiquetaFecha,
  formatearFecha,
  type DatosGrafico,
  type Fila,
} from "./calculo";
import { COLOR_UNICO, RegistroColores } from "./colores";
import { conFiltro, filtroParaCategoria } from "./edicion";
import { origenDe, type Lienzo, type Widget, type WidgetBarras, type WidgetKpi, type WidgetLinea } from "./esquema";
import GraficoBarras from "./graficos/Barras";
import { Leyenda, MarcoGrafico, type TablaDeDatos } from "./graficos/comun";
import GraficoLinea from "./graficos/Linea";
import GraficoTorta from "./graficos/Torta";
import type { Seleccion } from "./graficos/utiles";
import { TablaLienzo, TarjetaKpi } from "./widgets";

/**
 * Renderizador del lienzo: dibuja un documento ya validado (`validarLienzo`)
 * con componentes propios. No ejecuta nada que venga en el documento: cada
 * widget es un dato que se lee, no código.
 *
 * Disposición: "cuadricula" (12 columnas; cada panel con su `ancho`) o
 * "enfoque" (un panel debajo del otro, grandes y al centro). Con `onEditar`,
 * tocar una barra o una porción devuelve el documento filtrado por ella.
 */
const ANCHO_POR_OMISION: Record<Widget["tipo"], Ancho> = {
  kpi: "cuarto",
  barras: "mitad",
  linea: "mitad",
  torta: "tercio",
  tabla: "completo",
};

const TEXTO_AGRUPACION: Record<AgrupacionFecha, string> = { dia: "por día", semana: "por semana", mes: "por mes" };

function etiquetaCampo(fuente: Origen, campo: string): string {
  return definicionCampo(fuente, campo)?.etiqueta ?? campo;
}

/** "Contratistas", "Empresas (distintos)"… lo que mide un gráfico, dicho en palabras. */
function textoMedida(fuente: Origen, medida: string, campo?: string): string {
  return medida === "contar_distintos" && campo
    ? `${etiquetaCampo(fuente, campo)} (distintos)`
    : etiquetaOrigen(fuente);
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

interface Contexto {
  lienzo: Lienzo;
  filas: Fila[];
  anteriores?: Fila[];
  rango?: { desde: string; hasta: string };
  colores: (campo: string, nombres: string[]) => string[];
  enfoque: boolean;
  onEditar?: (nuevo: Lienzo) => void;
}

/** Clic para profundizar: filtra el lienzo por la categoría tocada, si el campo lo permite. */
function profundizar(ctx: Contexto, campo: string, agrupacion: AgrupacionFecha | null, crudo = (c: string) => c): Seleccion | undefined {
  const { lienzo, onEditar } = ctx;
  if (!onEditar) return undefined;
  return {
    puede: (clave) => filtroParaCategoria(origenDe(lienzo), campo, crudo(clave), agrupacion) !== null,
    alSeleccionar: (clave) => {
      const filtro = filtroParaCategoria(origenDe(lienzo), campo, crudo(clave), agrupacion);
      if (filtro) onEditar(conFiltro(lienzo, filtro));
    },
  };
}

function GraficoDeSeries({ ctx, widget }: { ctx: Contexto; widget: WidgetBarras | WidgetLinea }) {
  const { lienzo, filas, enfoque } = ctx;
  const medida = textoMedida(origenDe(lienzo), widget.medida, widget.campo);
  const datos = calcularSeries(origenDe(lienzo), widget, filas, medida);
  const eje = etiquetaCampo(origenDe(lienzo), widget.eje_x);
  const etiquetar = (c: string) => (datos.agrupacion ? etiquetaFecha(c, datos.agrupacion, lienzo.formato?.fecha) : c);
  const unica = !widget.serie;
  const paleta = unica ? [COLOR_UNICO] : ctx.colores(widget.serie ?? "", datos.series.map((s) => s.nombre));
  const titulo = widget.titulo ?? `${medida} por ${eje.toLowerCase()}`;
  const subtitulo = [
    datos.agrupacion ? TEXTO_AGRUPACION[datos.agrupacion] : null,
    widget.serie ? `por ${etiquetaCampo(origenDe(lienzo), widget.serie).toLowerCase()}` : null,
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
          seleccion={profundizar(ctx, widget.eje_x, datos.agrupacion)}
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

function Kpi({ ctx, widget }: { ctx: Contexto; widget: WidgetKpi }) {
  const { lienzo, filas, anteriores, rango } = ctx;
  const valor = calcularKpi(widget, filas);
  const esFecha = widget.campo && definicionCampo(origenDe(lienzo), widget.campo)?.tipo === "fecha";
  const dias = rango ? diasEntre(rango.desde, rango.hasta) + 1 : 0;
  const anterior = widget.comparar && anteriores ? calcularKpi(widget, anteriores) : null;
  return (
    <TarjetaKpi
      valor={esFecha && typeof valor === "string" ? formatearFecha(valor, lienzo.formato?.fecha) : valor}
      titulo={widget.titulo}
      tono={widget.tono}
      comparacion={
        typeof anterior === "number" ? { anterior, contra: `vs. ${dias === 1 ? "el día" : `los ${dias} días`} anteriores` } : undefined
      }
      tendencia={widget.tendencia && rango ? calcularTendencia(widget, filas, rango.desde, rango.hasta) : undefined}
    />
  );
}

function WidgetLienzo({ ctx, widget }: { ctx: Contexto; widget: Widget }): ReactNode {
  const { lienzo, filas, enfoque } = ctx;
  switch (widget.tipo) {
    case "kpi":
      return <Kpi ctx={ctx} widget={widget} />;
    case "barras":
    case "linea":
      return <GraficoDeSeries ctx={ctx} widget={widget} />;
    case "torta": {
      const calculadas = calcularPorciones(widget, filas);
      // Una categoría de fecha se muestra en el formato del lienzo; el filtro usa la fecha real.
      const esFecha = definicionCampo(origenDe(lienzo), widget.categoria)?.tipo === "fecha";
      const mostrar = (nombre: string) => (esFecha ? formatearFecha(nombre, lienzo.formato?.fecha) : nombre);
      const crudo = new Map(calculadas.porciones.map((p) => [mostrar(p.nombre), p.nombre]));
      const porciones = calculadas.porciones.map((p) => ({ ...p, nombre: mostrar(p.nombre) }));
      const categoria = etiquetaCampo(origenDe(lienzo), widget.categoria);
      const medida = textoMedida(origenDe(lienzo), widget.medida, widget.campo);
      const titulo = widget.titulo ?? `${medida} por ${categoria.toLowerCase()}`;
      const paleta = ctx.colores(widget.categoria, porciones.map((p) => p.nombre));
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
            total={calculadas.total}
            colores={paleta}
            titulo={titulo}
            dona={widget.dona ?? true}
            tamano={enfoque ? 380 : 260}
            seleccion={profundizar(ctx, widget.categoria, null, (c) => crudo.get(c) ?? c)}
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
          <TablaLienzo tabla={calcularTabla(origenDe(lienzo), widget, filas, lienzo.formato?.fecha)} />
        </section>
      );
  }
}

export default function LienzoVista({
  lienzo,
  filas,
  anteriores,
  rango,
  onEditar,
}: {
  lienzo: Lienzo;
  filas: Fila[];
  /** Filas del período anterior, para los KPI que comparan. */
  anteriores?: Fila[];
  rango?: { desde: string; hasta: string };
  /** Con esto, los gráficos aceptan clic para profundizar. */
  onEditar?: (nuevo: Lienzo) => void;
}) {
  const colores = useColores();
  const enfoque = lienzo.disposicion === "enfoque";
  const ctx: Contexto = { lienzo, filas, anteriores, rango, colores, enfoque, onEditar };
  return (
    <div className={`lienzo-documento ${enfoque ? "lienzo-enfoque" : "lienzo-cuadricula"}`} data-fuente={lienzo.fuente}>
      {lienzo.titulo && <h2 className="lienzo-titulo">{lienzo.titulo}</h2>}
      {lienzo.widgets.map((widget, i) => (
        <div
          key={i}
          className={`lienzo-panel lienzo-ancho-${enfoque ? "completo" : (widget.ancho ?? ANCHO_POR_OMISION[widget.tipo])}`}
          style={{ "--i": i } as CSSProperties}
        >
          <WidgetLienzo ctx={ctx} widget={widget} />
        </div>
      ))}
    </div>
  );
}
