import { useState, type CSSProperties, type ReactNode } from "react";
import { definicionCampo, etiquetaOrigen, type AgrupacionFecha, type Ancho, type Origen } from "./catalogo";
import {
  calcularKpi,
  calcularPorciones,
  calcularSeries,
  calcularTendencia,
  diasEntre,
  esDiasParaVencer,
  etiquetaFecha,
  formatearFecha,
  hoyCostaRica,
  sumarDias,
  textoDiasParaVencer,
  type DatosGrafico,
  type Fila,
} from "./calculo";
import { COLOR_UNICO, RegistroColores } from "./colores";
import { conFiltro, filtroParaCategoria } from "./edicion";
import {
  origenDe,
  type Lienzo,
  type Widget,
  type WidgetAnalisis,
  type WidgetBarras,
  type WidgetKpi,
  type WidgetLinea,
} from "./esquema";
import AnalisisPython from "./AnalisisPython";
import GraficoCalendario from "./graficos/Calendario";
import GraficoCalor from "./graficos/Calor";
import { calcularCalendario, calcularCalor, fechaLarga } from "./mapas";
import GraficoBarras from "./graficos/Barras";
import { Leyenda, MarcoGrafico, type TablaDeDatos } from "./graficos/comun";
import GraficoLinea from "./graficos/Linea";
import GraficoTorta from "./graficos/Torta";
import PanelLibre from "./PanelLibre";
import type { Seleccion } from "./graficos/utiles";
import TablaRica from "./TablaRica";
import { TarjetaKpi } from "./widgets";

/**
 * Renderizador del lienzo: dibuja un documento ya validado (`validarLienzo`)
 * con componentes propios. No ejecuta nada que venga en el documento: cada
 * widget es un dato que se lee, no código. La única excepción es el panel
 * "libre", cuyo código corre fuera del panel, en el sandbox (ver PanelLibre).
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
  calor: "completo",
  calendario: "completo",
  libre: "completo",
  // Un análisis toma el ancho del panel que muestra su resultado (ver anchoPorOmision).
  analisis: "mitad",
};

const TEXTO_AGRUPACION: Record<AgrupacionFecha, string> = { dia: "por día", semana: "por semana", mes: "por mes" };

function etiquetaCampo(fuente: Origen, campo: string): string {
  return definicionCampo(fuente, campo)?.etiqueta ?? campo;
}

/** "Contratistas", "Empresas (distintos)"… lo que mide un gráfico, dicho en palabras. */
function textoMedida(fuente: Origen, medida: string, campo?: string): string {
  if (medida === "contar_distintos" && campo) return `${etiquetaCampo(fuente, campo)} (distintos)`;
  if (medida === "sumar" && campo) return etiquetaCampo(fuente, campo);
  return etiquetaOrigen(fuente);
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
  const etiquetar = (c: string) =>
    datos.agrupacion
      ? etiquetaFecha(c, datos.agrupacion, lienzo.formato?.fecha)
      : esDiasParaVencer(widget.eje_x) && c.trim() !== "" && Number.isFinite(Number(c))
        ? textoDiasParaVencer(Number(c))
        : c;
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

/**
 * Comparación justa contra el período anterior. Si el rango termina hoy, el día
 * de hoy va a medias: se comparan sólo los días completos (sin hoy) contra la
 * misma cantidad de días antes. Si el rango es sólo hoy, no se compara: un día a
 * medias contra uno completo siempre "cae" y no dice nada.
 */
function comparacionKpi(widget: WidgetKpi, filas: Fila[], anteriores: Fila[] | undefined, rango: Contexto["rango"]) {
  if (!widget.comparar || !anteriores || !rango) return undefined;
  const dias = diasEntre(rango.desde, rango.hasta) + 1;
  const terminaHoy = rango.hasta === hoyCostaRica();
  const losAnteriores = (n: number) => (n === 1 ? "el día anterior" : `los ${n} días anteriores`);
  if (!terminaHoy) {
    const anterior = calcularKpi(widget, anteriores);
    return typeof anterior === "number" ? { anterior, contra: `que ${losAnteriores(dias)}` } : undefined;
  }
  if (dias < 2) return undefined;
  const dia = (f: Fila) => (typeof f.dia === "string" ? f.dia : "");
  const actual = calcularKpi(widget, filas.filter((f) => dia(f) !== "" && dia(f) < rango.hasta));
  // Los días completos actuales van de `desde` a ayer (dias - 1). Se comparan con los mismos días
  // justo antes de `desde`: el período anterior sin su PRIMER día.
  const desdeAnterior = sumarDias(rango.desde, -(dias - 1));
  const anterior = calcularKpi(widget, anteriores.filter((f) => dia(f) !== "" && dia(f) >= desdeAnterior && dia(f) < rango.desde));
  if (typeof actual !== "number" || typeof anterior !== "number") return undefined;
  return { actual, anterior, contra: `que ${losAnteriores(dias - 1)}, sin contar hoy` };
}

function Kpi({ ctx, widget }: { ctx: Contexto; widget: WidgetKpi }) {
  const { lienzo, filas, anteriores, rango } = ctx;
  const valor = calcularKpi(widget, filas);
  const esFecha = widget.campo && definicionCampo(origenDe(lienzo), widget.campo)?.tipo === "fecha";
  const mostrado =
    esFecha && typeof valor === "string"
      ? formatearFecha(valor, lienzo.formato?.fecha)
      : widget.campo && esDiasParaVencer(widget.campo) && typeof valor === "number"
        ? textoDiasParaVencer(valor)
        : valor;
  return (
    <TarjetaKpi
      valor={mostrado}
      titulo={widget.titulo}
      tono={widget.tono}
      comparacion={comparacionKpi(widget, filas, anteriores, rango)}
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
          <TablaRica origen={origenDe(lienzo)} widget={widget} filas={filas} formatoFecha={lienzo.formato?.fecha} />
        </section>
      );
    case "calor": {
      const origen = origenDe(lienzo);
      const datos = calcularCalor(origen, widget, filas, lienzo.formato?.fecha);
      const medida = textoMedida(origen, widget.medida, widget.campo);
      const titulo = widget.titulo ?? `${medida} por ${etiquetaCampo(origen, widget.filas).toLowerCase()} y ${etiquetaCampo(origen, widget.columnas).toLowerCase()}`;
      const { onEditar } = ctx;
      // Clic en una celda: la vista filtrada por esa fila y esa columna (si los dos campos se pueden filtrar).
      const alSeleccionar = onEditar
        ? (fila: string, columna: string) => {
            const porFila = filtroParaCategoria(origen, widget.filas, fila);
            const porColumna = filtroParaCategoria(origen, widget.columnas, columna);
            if (porFila && porColumna) onEditar(conFiltro(conFiltro(lienzo, porFila), porColumna));
          }
        : undefined;
      return (
        <MarcoGrafico
          titulo={titulo}
          datos={{
            columnas: [etiquetaCampo(origen, widget.filas), ...datos.columnas.map((c) => c.etiqueta)],
            filas: datos.filas.map((f, i) => [f.etiqueta, ...datos.valores[i]]),
          }}
        >
          <GraficoCalor datos={datos} titulo={titulo} medida={medida} alSeleccionar={alSeleccionar} />
        </MarcoGrafico>
      );
    }
    case "calendario": {
      const origen = origenDe(lienzo);
      const datos = calcularCalendario(widget, filas);
      const medida = textoMedida(origen, widget.medida, widget.campo);
      const titulo = widget.titulo ?? `${medida} por ${etiquetaCampo(origen, widget.fecha).toLowerCase()}`;
      const { onEditar } = ctx;
      const alSeleccionar = onEditar
        ? (fecha: string) => {
            const filtro = filtroParaCategoria(origen, widget.fecha, fecha);
            if (filtro) onEditar(conFiltro(lienzo, filtro));
          }
        : undefined;
      return (
        <MarcoGrafico
          titulo={titulo}
          datos={{
            columnas: [etiquetaCampo(origen, widget.fecha), medida],
            filas: datos.meses.flatMap((m) => m.dias.filter((d) => d.valor > 0).map((d) => [fechaLarga(d.fecha), d.valor])),
          }}
        >
          <GraficoCalendario datos={datos} titulo={titulo} medida={medida} alSeleccionar={alSeleccionar} />
        </MarcoGrafico>
      );
    }
    case "libre":
      return <PanelLibre widget={widget} filas={filas} origen={origenDe(lienzo)} />;
    case "analisis":
      return (
        <AnalisisPython
          widget={widget}
          filas={filas}
          origen={origenDe(lienzo)}
          dibujar={(resultado) => (
            <WidgetLienzo ctx={contextoDeAnalisis(ctx, widget, resultado)} widget={{ ...widget.mostrar, titulo: widget.titulo }} />
          )}
        />
      );
  }
}

/**
 * El resultado de un análisis se dibuja como un lienzo propio: sus filas son las
 * que devolvió Python, con las columnas que declara. Sin profundizar al tocar
 * (un filtro sobre el resultado no se puede llevar a los datos de origen).
 */
function contextoDeAnalisis(ctx: Contexto, widget: WidgetAnalisis, resultado: Fila[]): Contexto {
  return {
    lienzo: {
      fuente: "consulta",
      consulta: { sql: "(análisis en Python)", columnas: widget.columnas },
      filtros: [],
      widgets: [widget.mostrar],
      formato: ctx.lienzo.formato,
    },
    filas: resultado,
    colores: ctx.colores,
    enfoque: ctx.enfoque,
  };
}

function anchoPorOmision(widget: Widget): Ancho {
  return widget.tipo === "analisis" ? ANCHO_POR_OMISION[widget.mostrar.tipo] : ANCHO_POR_OMISION[widget.tipo];
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
          className={`lienzo-panel lienzo-ancho-${enfoque ? "completo" : (widget.ancho ?? anchoPorOmision(widget))}`}
          style={{ "--i": i } as CSSProperties}
        >
          <WidgetLienzo ctx={ctx} widget={widget} />
        </div>
      ))}
    </div>
  );
}
