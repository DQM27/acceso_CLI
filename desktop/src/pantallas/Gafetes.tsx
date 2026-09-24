import { useCallback, useMemo, useState } from "react";
import type { ColDef, ICellRendererParams } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import { useCargaAlCambiar } from "../componentes/useCargaAlCambiar";
import { useBarraEstado } from "../contexto/BarraEstadoContexto";
import FormularioGafete from "./FormularioGafete";
import GestionGafeteModal from "./GestionGafeteModal";
import HistorialGafeteModal from "./HistorialGafeteModal";
import { buscarGafetes, nombrePortador } from "../api";
import type { FiltroGafetes, GafeteResumen, TipoGafeteEntrada } from "../api";

/**
 * Catálogo de gafetes (`docs/plan-gafetes.md`) — sin restricción de rol a
 * propósito, mismo criterio que el núcleo: cualquier operador con sesión
 * gestiona alta/baja/perdido/resolver. Doble click en una fila abre las
 * acciones disponibles según su estado (mismo criterio que la TUI: B/P/R
 * sólo aplican según el estado actual, ver `src/tui/gafetes/state.rs`) — la
 * columna "Resolver" es el mismo modal, explícita a propósito para el
 * operador que no conozca el doble click (sólo visible en estado Perdido,
 * que es cuando resolver aplica). La columna "Historial" abre por separado
 * (`HistorialGafeteModal`) quién marcó perdido/resolvió cada incidente —
 * separado de la gestión para no mezclar "qué puedo hacer" con "qué le pasó
 * antes".
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

  useBarraEstado(cargando ? "Cargando…" : `${filas.length} resultado(s)`);

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
        headerName: "Asignado a",
        flex: 1.6,
        minWidth: 170,
        cellStyle: { textAlign: "left" },
        valueGetter: ({ data }) => (data ? nombrePortador(data) : null),
      },
      {
        headerName: "Resolver",
        flex: 1.1,
        minWidth: 110,
        filter: false,
        sortable: false,
        cellRenderer: (p: ICellRendererParams<GafeteResumen>) => {
          const gafete = p.data;
          return gafete && gafete.estado === "Perdido" ? (
            <button
              type="button"
              className="boton"
              style={{ padding: "0.15rem 0.55rem", fontSize: "0.78rem" }}
              onClick={() => setGestionAbierta(gafete)}
            >
              Resolver
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
              style={{ padding: "0.15rem 0.55rem", fontSize: "0.78rem" }}
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

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        <div style={{ flex: 1, minHeight: 0 }}>
          <Tabla<GafeteResumen>
            cargando={cargando}
            filtrosPorColumna
            id="gafetes"
            columnas={columnas}
            filas={filas}
            onFilaDobleClic={setGestionAbierta}
            controles={
              <>
                <button className="boton" onClick={() => setFormularioAbierto(true)}>
                  + Nuevo
                </button>
                <div className="campo" style={{ flex: "0 1 16rem" }}>
                  <input
                    placeholder="Número…"
                    value={texto}
                    onChange={(evento) => setTexto(evento.target.value.replace(/\D/g, ""))}
                    inputMode="numeric"
                  />
                </div>
                <div className="campo" style={{ flex: "0 1 12rem" }}>
                  <select
                    value={tipo}
                    onChange={(evento) => setTipo(evento.target.value as TipoGafeteEntrada)}
                  >
                    <option value="contratista">Contratista</option>
                    <option value="visita">Visita</option>
                    <option value="provisional_kof">Provisional KOF</option>
                    <option value="proveedor">Proveedor</option>
                  </select>
                </div>
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
