import { useCallback, useMemo, useState } from "react";
import { List, ListChecks, Plus } from "lucide-react";
import type { ColDef, ICellRendererParams } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import { useCargaAlCambiar } from "../componentes/useCargaAlCambiar";
import { useBarraEstado } from "../contexto/BarraEstadoContexto";
import { textoConteo, useConteoVisible } from "../componentes/conteoTabla";
import FormularioGafete from "./FormularioGafete";
import GestionGafeteModal from "./GestionGafeteModal";
import HistorialGafeteModal from "./HistorialGafeteModal";
import ConteoGafetes from "./ConteoGafetes";
import SegmentadoOpciones from "../componentes/SegmentadoOpciones";
import type { OpcionSegmentada } from "../componentes/SegmentadoOpciones";
import { buscarGafetes, nombrePortador } from "../api";
import type { FiltroGafetes, GafeteResumen, TipoGafeteEntrada } from "../api";

type VistaGafetes = "lista" | "conteo";

/** Lista del catálogo o conteo físico del tipo elegido. */
const OPCIONES_VISTA: OpcionSegmentada<VistaGafetes>[] = [
  { valor: "lista", Icono: List, titulo: "Lista de gafetes" },
  { valor: "conteo", Icono: ListChecks, titulo: "Conteo de gafetes" },
];

/**
 * Catálogo de gafetes — sin restricción de rol a propósito, mismo criterio
 * que el núcleo: cualquier operador con sesión da de alta, reporta perdido,
 * registra el pago, recupera y cuenta. Doble clic en una fila abre las
 * acciones según su estado; la columna "Acciones" abre lo mismo, explícita
 * para quien no conozca el doble clic. La columna "Historial" abre por
 * separado (`HistorialGafeteModal`) cada reporte, quién lo hizo y qué
 * escribió — separado de la gestión para no mezclar "qué puedo hacer" con
 * "qué le pasó antes". La vista "Conteo" es el conteo físico del tipo
 * elegido (`ConteoGafetes`), en la misma pantalla y sin ventana encima.
 */
export default function Gafetes() {
  const [texto, setTexto] = useState("");
  // Sin opción "Todos los tipos" a propósito -- un número de gafete se
  // repite entre pools (`gafetes.tipo`/`gafetes.numero` es único por PAR,
  // no global, ver schema.rs), así que mezclar tipos en la misma vista
  // puede confundir a cuál categoría pertenece cada fila.
  const [tipo, setTipo] = useState<TipoGafeteEntrada>("contratista");
  const [filas, setFilas] = useState<GafeteResumen[]>([]);
  const [cargando, setCargando] = useState(true);
  const [formularioAbierto, setFormularioAbierto] = useState(false);
  const [gestionAbierta, setGestionAbierta] = useState<GafeteResumen | null>(null);
  const [detalleAbierto, setDetalleAbierto] = useState<GafeteResumen | null>(null);
  const [vista, setVista] = useState<VistaGafetes>("lista");

  const [conteo, alContar] = useConteoVisible();

  useBarraEstado(cargando ? "Cargando…" : textoConteo(conteo, filas.length, "resultado(s)"));

  const columnas: ColDef<GafeteResumen>[] = useMemo(
    () => [
      {
        field: "numero",
        type: "numero",
        headerName: "Número",
        flex: 1.1,
        minWidth: 110,
        valueFormatter: ({ value }) => String(value).padStart(2, "0"),
      },
      { field: "tipo", headerName: "Tipo", flex: 1, minWidth: 110 },
      { field: "estado", headerName: "Estado", flex: 1.3, minWidth: 130 },
      {
        headerName: "Responsable",
        flex: 1.6,
        minWidth: 170,
        cellStyle: { textAlign: "left" },
        valueGetter: ({ data }) =>
          data && data.estado === "Perdido" ? (nombrePortador(data) ?? "Sin responsable") : null,
      },
      {
        headerName: "Acciones",
        flex: 1.1,
        minWidth: 110,
        filter: false,
        sortable: false,
        cellRenderer: (p: ICellRendererParams<GafeteResumen>) => {
          const gafete = p.data;
          return gafete ? (
            <button
              type="button"
              className="boton"
              style={{ padding: "0.15rem 0.55rem", fontSize: "var(--tam-xs)" }}
              onClick={() => setGestionAbierta(gafete)}
            >
              {gafete.estado === "Perdido" ? "Pago / recuperar" : "Reportar perdido"}
            </button>
          ) : null;
        },
      },
      {
        headerName: "Historial",
        flex: 1.1,
        minWidth: 110,
        filter: false,
        sortable: false,
        cellRenderer: (p: ICellRendererParams<GafeteResumen>) => {
          const gafete = p.data;
          return gafete ? (
            <button
              type="button"
              className="boton"
              style={{ padding: "0.15rem 0.55rem", fontSize: "var(--tam-xs)" }}
              onClick={() => setDetalleAbierto(gafete)}
            >
              Detalles
            </button>
          ) : null;
        },
      },
    ],
    [],
  );

  const recargar = useCallback(
    (estaVigente: () => boolean = () => true) => {
      setCargando(true);
      const numero = /^\d+$/.test(texto.trim()) ? Number(texto.trim()) : undefined;
      const filtro: FiltroGafetes = { numero, tipo };
      return buscarGafetes(filtro)
        .then((datos) => {
          if (estaVigente()) setFilas(datos);
        })
        .finally(() => {
          if (estaVigente()) setCargando(false);
        });
    },
    [texto, tipo],
  );

  // `true`: el catálogo ahora también recibe cambios de OTRO dispositivo
  // del sitio (`recibir_catalogo_del_sitio`, ver `docs/planes-implementados/plan-persistencia-nube.md`)
  // -- sin esto, marcar un gafete perdido/resuelto desde otra PC no se
  // reflejaba acá hasta recargar a mano.
  useCargaAlCambiar(recargar, true);

  const selectorVista = (
    <SegmentadoOpciones<VistaGafetes>
      etiqueta="Vista"
      opciones={OPCIONES_VISTA}
      valor={vista}
      onCambiar={setVista}
    />
  );
  const selectorTipo = (
    <div className="campo" style={{ flex: "0 1 12rem" }}>
      <select
        aria-label="Tipo de gafete"
        value={tipo}
        onChange={(evento) => setTipo(evento.target.value as TipoGafeteEntrada)}
      >
        <option value="contratista">Contratista</option>
        <option value="visita">Visita</option>
        <option value="provisional_kof">Provisional KOF</option>
        <option value="proveedor">Proveedor</option>
      </select>
    </div>
  );

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        {vista === "conteo" && (
          <div style={{ display: "flex", flexDirection: "column", flex: 1, minHeight: 0, gap: "0.375rem" }}>
            <div style={{ display: "flex", alignItems: "flex-end", gap: "0.375rem" }}>
              {selectorVista}
              {selectorTipo}
            </div>
            <div style={{ flex: 1, minHeight: 0 }}>
              <ConteoGafetes key={tipo} tipo={tipo} />
            </div>
          </div>
        )}
        <div style={{ flex: 1, minHeight: 0, display: vista === "lista" ? undefined : "none" }}>
          <Tabla<GafeteResumen>
            cargando={cargando}
            onConteoVisible={alContar}
            filtrosPorColumna
            id="gafetes"
            columnas={columnas}
            filas={filas}
            onFilaDobleClic={setGestionAbierta}
            mensajeVacio="No hay gafetes de este tipo. «Nuevo gafete» agrega uno o un rango."
            // Misma barra en todas las pantallas (auditoría 2026-11-14, U5).
            accionesDerecha={
              <div className="campo" style={{ flex: "1 1 16rem", minWidth: "10rem", maxWidth: "18rem" }}>
                <input
                  placeholder="Número…"
                  value={texto}
                  onChange={(evento) => setTexto(evento.target.value.replace(/\D/g, ""))}
                  inputMode="numeric"
                />
              </div>
            }
            controles={
              <>
                {selectorVista}
                <button
                  type="button"
                  className="boton boton-icono"
                  onClick={() => setFormularioAbierto(true)}
                >
                  <Plus size={16} aria-hidden="true" />
                  Nuevo gafete
                </button>
                {selectorTipo}
              </>
            }
          />
        </div>
      </div>

      {formularioAbierto && (
        <FormularioGafete
          onCerrar={() => setFormularioAbierto(false)}
          onGuardado={() => {
            setFormularioAbierto(false);
            recargar();
          }}
        />
      )}

      {gestionAbierta && (
        <GestionGafeteModal
          gafete={gestionAbierta}
          onCerrar={() => setGestionAbierta(null)}
          onCambiado={() => {
            setGestionAbierta(null);
            recargar();
          }}
        />
      )}

      {detalleAbierto && (
        <HistorialGafeteModal gafete={detalleAbierto} onCerrar={() => setDetalleAbierto(null)} />
      )}
    </div>
  );
}
