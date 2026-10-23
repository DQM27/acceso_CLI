import "../componentes/modulosTabla";
import { useMemo, useState } from "react";
import { AgGridReact, type CustomCellRendererProps } from "ag-grid-react";
import type { ColDef, RowClassParams } from "ag-grid-community";
import { AG_GRID_LOCALE_ES } from "@ag-grid-community/locale";
import { ChevronRight, CircleAlert, CircleCheck, Minus, Search, TriangleAlert } from "lucide-react";
import { temaBrisas } from "../componentes/temaTabla";
import type { FormatoFecha, Origen } from "./catalogo";
import type { WidgetTabla } from "./esquema";
import type { Fila, Valor } from "./calculo";
import {
  LIMITE_POR_GRUPO,
  compararValores,
  modeloTabla,
  textoCelda,
  tonoCelda,
  type ColumnaRica,
  type Grupo,
  type Tono,
} from "./TablaRica.logica";
import { Tendencia } from "./widgets";

/**
 * Tabla del lienzo sobre AG Grid (la misma grilla y el mismo tema que el resto
 * del panel): etiquetas de estado con ícono y texto, barras dentro de las
 * celdas de número, orden al tocar una columna, búsqueda, y en una agrupada,
 * grupos que se abren para ver sus filas, con su cantidad y su mini línea.
 * Todo sale de `TablaRica.logica.ts`; acá sólo se dibuja.
 *
 * Con `domLayout="autoHeight"` la grilla dibuja todas sus filas (como mucho
 * 500): así se imprime completa. En pantalla la caja tiene un alto máximo y
 * se desplaza.
 */

type FilaGrilla =
  | { tipo: "fila"; id: string; fila: Fila }
  | { tipo: "grupo"; id: string; grupo: Grupo; abierto: boolean }
  | { tipo: "mas"; id: string; restantes: number };

/** Con más filas que esto aparece la búsqueda. */
const FILAS_PARA_BUSCAR = 8;

const ICONO_TONO = { exito: CircleCheck, advertencia: TriangleAlert, error: CircleAlert, neutro: Minus } as const;

function Etiqueta({ tono, texto }: { tono: Tono; texto: string }) {
  const Icono = ICONO_TONO[tono];
  return (
    <span className={`lienzo-estado lienzo-estado-${tono}`}>
      <Icono size={13} aria-hidden="true" />
      {texto}
    </span>
  );
}

/** Un número con su barra: el largo es relativo al mayor de la columna. */
function Barra({ valor, maximo, texto }: { valor: number; maximo: number; texto: string }) {
  const parte = maximo > 0 && valor > 0 ? Math.min(1, valor / maximo) : 0;
  return (
    <span className="lienzo-celda-barra">
      <span className="lienzo-celda-barra-fondo" aria-hidden="true">
        <span style={{ width: `${(parte * 100).toFixed(1)}%` }} />
      </span>
      <span className="lienzo-numero">{texto}</span>
    </span>
  );
}

function Celda({ columna, valor, formatoFecha }: { columna: ColumnaRica; valor: Valor | undefined; formatoFecha?: FormatoFecha }) {
  const texto = textoCelda(columna, valor, formatoFecha);
  const tono = tonoCelda(columna, valor);
  if (tono) return <Etiqueta tono={tono} texto={texto} />;
  if (columna.maximoBarra !== undefined && typeof valor === "number") {
    return <Barra valor={valor} maximo={columna.maximoBarra} texto={texto} />;
  }
  return <span className={typeof valor === "number" ? "lienzo-numero" : undefined}>{texto}</span>;
}

function plural(n: number, uno: string, varios: string) {
  return `${n.toLocaleString("es-CR")} ${n === 1 ? uno : varios}`;
}

export default function TablaRica({
  origen,
  widget,
  filas,
  formatoFecha,
}: {
  origen: Origen;
  widget: WidgetTabla;
  filas: Fila[];
  formatoFecha?: FormatoFecha;
}) {
  const [busqueda, setBusqueda] = useState("");
  const [abiertos, setAbiertos] = useState<ReadonlySet<string>>(new Set());
  const modelo = useMemo(
    () => modeloTabla(origen, widget, filas, { busqueda, formatoFecha }),
    [origen, widget, filas, busqueda, formatoFecha],
  );
  const agrupada = modelo.agrupada;

  const alternar = (clave: string) =>
    setAbiertos((actual) => {
      const siguiente = new Set(actual);
      if (siguiente.has(clave)) siguiente.delete(clave);
      else siguiente.add(clave);
      return siguiente;
    });

  const filasGrilla = useMemo<FilaGrilla[]>(() => {
    if (!agrupada) return modelo.filas.map((fila, i) => ({ tipo: "fila", id: `f${i}`, fila }));
    // Buscando, los grupos que quedan se ven abiertos: lo buscado está adentro.
    const abrirTodo = busqueda.trim() !== "" && modelo.columnas.length > 0;
    return modelo.grupos.flatMap((grupo): FilaGrilla[] => {
      const abierto = modelo.columnas.length > 0 && (abrirTodo || abiertos.has(grupo.clave));
      const cabeza: FilaGrilla = { tipo: "grupo", id: `g:${grupo.clave}`, grupo, abierto };
      if (!abierto) return [cabeza];
      const hijas = grupo.filas
        .slice(0, LIMITE_POR_GRUPO)
        .map((fila, i): FilaGrilla => ({ tipo: "fila", id: `g:${grupo.clave}:${i}`, fila }));
      const restantes = grupo.filas.length - hijas.length;
      return restantes > 0 ? [cabeza, ...hijas, { tipo: "mas", id: `g:${grupo.clave}:mas`, restantes }] : [cabeza, ...hijas];
    });
  }, [agrupada, modelo, abiertos, busqueda]);

  const columnas = useMemo<ColDef<FilaGrilla>[]>(() => {
    const deDetalle = modelo.columnas.map(
      (c, i): ColDef<FilaGrilla> => ({
        colId: c.id,
        headerName: c.etiqueta,
        flex: c.tipo === "texto" || c.tipo === "enumeracion" ? 2 : 1,
        minWidth: 110,
        sortable: !agrupada,
        valueGetter: (p) => (p.data?.tipo === "fila" ? p.data.fila[c.id] : null),
        comparator: (a: Valor, b: Valor, _na, _nb, descendente) => {
          // AG Grid invierte el resultado en descendente: los vacíos se mantienen al final igual.
          const vacio = (v: Valor) => v === null || v === undefined || v === "";
          const r = compararValores(c.id, a, b);
          return descendente && (vacio(a) || vacio(b)) ? -r : r;
        },
        cellRenderer: (p: CustomCellRendererProps<FilaGrilla>) => {
          if (p.data?.tipo === "mas" && i === 0) {
            return <span className="lienzo-nota-celda">y {plural(p.data.restantes, "fila más", "filas más")}: búsquela o exporte la vista</span>;
          }
          return p.data?.tipo === "fila" ? <Celda columna={c} valor={p.data.fila[c.id]} formatoFecha={formatoFecha} /> : null;
        },
      }),
    );
    if (!agrupada) return deDetalle;

    const deGrupo: ColDef<FilaGrilla>[] = [
      {
        colId: agrupada.campo.id,
        headerName: agrupada.campo.etiqueta,
        flex: 2,
        minWidth: 160,
        sortable: false,
        cellRenderer: (p: CustomCellRendererProps<FilaGrilla>) => {
          if (p.data?.tipo !== "grupo") return null;
          const { grupo, abierto } = p.data;
          const texto = textoCelda(agrupada.campo, grupo.valor, formatoFecha);
          const contenido = tonoCelda(agrupada.campo, grupo.valor) ? (
            <Etiqueta tono={tonoCelda(agrupada.campo, grupo.valor) ?? "neutro"} texto={texto} />
          ) : (
            <span>{texto}</span>
          );
          if (modelo.columnas.length === 0) return contenido;
          return (
            <button
              type="button"
              className="lienzo-grupo-boton"
              aria-expanded={abierto}
              aria-label={`${abierto ? "Cerrar" : "Ver"} las filas de ${texto}`}
              onClick={() => alternar(grupo.clave)}
            >
              <ChevronRight size={15} aria-hidden="true" className="lienzo-grupo-flecha" />
              {contenido}
            </button>
          );
        },
      },
      {
        colId: "cantidad",
        headerName: "Cantidad",
        flex: 1,
        minWidth: 130,
        sortable: false,
        cellRenderer: (p: CustomCellRendererProps<FilaGrilla>) =>
          p.data?.tipo === "grupo" ? (
            <Barra
              valor={p.data.grupo.cantidad}
              maximo={agrupada.cantidad.maximoBarra ?? 0}
              texto={p.data.grupo.cantidad.toLocaleString("es-CR")}
            />
          ) : null,
      },
    ];
    if (agrupada.conTendencia) {
      deGrupo.push({
        colId: "tendencia",
        headerName: "Por día",
        width: 140,
        sortable: false,
        cellRenderer: (p: CustomCellRendererProps<FilaGrilla>) =>
          p.data?.tipo === "grupo" && p.data.grupo.tendencia ? (
            <Tendencia
              valores={p.data.grupo.tendencia}
              titulo={textoCelda(agrupada.campo, p.data.grupo.valor, formatoFecha)}
              ancho={110}
              alto={24}
            />
          ) : null,
      });
    }
    return [...deGrupo, ...deDetalle];
  }, [modelo, agrupada, formatoFecha]);

  const hayBusqueda = busqueda.trim() !== "";
  const cuantas = agrupada ? plural(modelo.total, "grupo", "grupos") : plural(modelo.total, "fila", "filas");
  const mostradas = agrupada ? modelo.grupos.length : modelo.filas.length;

  if (filas.length === 0) return <p className="lienzo-vacio">Sin datos para mostrar.</p>;
  return (
    <div className="lienzo-tabla-rica">
      {filas.length > FILAS_PARA_BUSCAR && (
        <label className="lienzo-tabla-buscar">
          <Search size={15} aria-hidden="true" />
          <input
            type="search"
            value={busqueda}
            onChange={(e) => setBusqueda(e.target.value)}
            placeholder="Buscar en la tabla"
            aria-label={`Buscar en la tabla${widget.titulo ? ` ${widget.titulo}` : ""}`}
          />
        </label>
      )}
      {mostradas === 0 ? (
        <p className="lienzo-vacio">{hayBusqueda ? `Nada coincide con «${busqueda.trim()}».` : "Sin datos para mostrar."}</p>
      ) : (
        <div className="lienzo-tabla-grilla">
          <AgGridReact<FilaGrilla>
            theme={temaBrisas}
            rowData={filasGrilla}
            columnDefs={columnas}
            getRowId={(p) => p.data.id}
            getRowClass={(p: RowClassParams<FilaGrilla>) =>
              p.data?.tipo === "grupo" ? "lienzo-fila-grupo" : p.data?.tipo === "mas" ? "lienzo-fila-mas" : undefined
            }
            domLayout="autoHeight"
            localeText={AG_GRID_LOCALE_ES}
            suppressCellFocus
            accentedSort
            suppressMovableColumns
            rowHeight={36}
            headerHeight={36}
            defaultColDef={{ resizable: true }}
          />
        </div>
      )}
      {/* Sólo si faltan filas: sin el aviso no se sabría que la tabla está recortada. */}
      {mostradas < modelo.total && (
        <p className="lienzo-nota">
          Se muestran {mostradas.toLocaleString("es-CR")} de {cuantas}.
        </p>
      )}
    </div>
  );
}
