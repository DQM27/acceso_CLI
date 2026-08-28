import { useCallback, useEffect, useState } from "react";
import type { ColDef, ICellRendererParams } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import Modal from "../componentes/Modal";
import PantallaEncabezado from "../componentes/PantallaEncabezado";
import NuevoIngresoModal from "./NuevoIngresoModal";
import { listarIngresosActivos, mensajeMotivoDenegacion, registrarSalida } from "../api";
import type { IngresoActivoResumen } from "../api";

function textoMedio(medio: IngresoActivoResumen["medio_ingreso"]): string {
  return medio === "Vehiculo" ? "Vehículo" : "Caminando";
}

function EstadoAcceso({ fila }: { fila: IngresoActivoResumen }) {
  const r = fila.resultado_acceso;
  if (r === "Permitido") {
    return <span className="chip" style={{ ["--chip-color" as string]: "var(--exito)" }}>Al día</span>;
  }
  if (r === "PermitidoConAdvertencia") {
    return (
      <span className="chip" style={{ ["--chip-color" as string]: "var(--advertencia)" }}>
        PRAIND próximo
      </span>
    );
  }
  return (
    <span
      className="chip"
      style={{ ["--chip-color" as string]: "var(--error)" }}
      title={mensajeMotivoDenegacion(r.Denegado)}
    >
      {mensajeMotivoDenegacion(r.Denegado)}
    </span>
  );
}

export default function Activos() {
  const [filas, setFilas] = useState<IngresoActivoResumen[]>([]);
  const [total, setTotal] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [seleccionadas, setSeleccionadas] = useState<IngresoActivoResumen[]>([]);
  const [modalNuevoIngreso, setModalNuevoIngreso] = useState(false);
  const [confirmarSalidaMasiva, setConfirmarSalidaMasiva] = useState(false);
  const [procesando, setProcesando] = useState(false);

  const recargar = useCallback(() => {
    return listarIngresosActivos().then((pagina) => {
      setFilas(pagina.items);
      setTotal(pagina.total);
      setSeleccionadas([]);
    });
  }, []);

  useEffect(() => {
    let vigente = true;
    recargar()
      .then(() => vigente && setError(null))
      .catch((error) => vigente && setError(String(error)));
    return () => {
      vigente = false;
    };
  }, [recargar]);

  async function salidaIndividual(id: number) {
    setError(null);
    try {
      await registrarSalida(id);
      recargar();
    } catch (error) {
      setError(String(error));
    }
  }

  async function confirmarSalidasSeleccionadas() {
    setProcesando(true);
    setError(null);
    try {
      for (const fila of seleccionadas) {
        await registrarSalida(fila.registro_id);
      }
      setConfirmarSalidaMasiva(false);
      await recargar();
    } catch (error) {
      setError(String(error));
    } finally {
      setProcesando(false);
    }
  }

  const columnas: ColDef<IngresoActivoResumen>[] = [
    { field: "cedula", headerName: "Cédula", width: 130 },
    { field: "contratista_nombre", headerName: "Nombre", flex: 1.4, minWidth: 160 },
    { field: "empresa_nombre", headerName: "Empresa", flex: 1, minWidth: 140 },
    { field: "tipo_ingreso", headerName: "Tipo", width: 110 },
    {
      headerName: "Medio",
      width: 110,
      valueGetter: (p) => (p.data ? textoMedio(p.data.medio_ingreso) : ""),
    },
    { field: "gafete_numero", headerName: "Gafete", width: 100 },
    {
      headerName: "Ingreso",
      width: 160,
      valueGetter: (p) => (p.data ? new Date(p.data.fecha_hora_ingreso).toLocaleString() : ""),
    },
    { field: "usuario_ingreso_nombre", headerName: "Dio ingreso", width: 150 },
    {
      headerName: "Estado",
      width: 170,
      filter: false,
      cellRenderer: (p: ICellRendererParams<IngresoActivoResumen>) =>
        p.data ? <EstadoAcceso fila={p.data} /> : null,
    },
    {
      headerName: "",
      width: 110,
      filter: false,
      sortable: false,
      pinned: "right",
      cellRenderer: (p: ICellRendererParams<IngresoActivoResumen>) =>
        p.data ? (
          <button
            type="button"
            className="boton"
            style={{ padding: "0.25rem 0.7rem", fontSize: "0.8rem" }}
            onClick={() => salidaIndividual(p.data!.registro_id)}
          >
            Salida
          </button>
        ) : null,
    },
  ];

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <PantallaEncabezado
        titulo="Ingresos activos"
        acciones={
          <button className="boton boton-primario" onClick={() => setModalNuevoIngreso(true)}>
            + Nuevo ingreso
          </button>
        }
      />

      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        {error && <p style={{ color: "var(--error)", margin: 0 }}>{error}</p>}

        {seleccionadas.length > 0 && (
          <div
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              padding: "0.5rem 0.75rem",
              border: "1px solid var(--borde)",
              borderRadius: "var(--radio-chico)",
              background: "var(--campo-fondo)",
            }}
          >
            <span style={{ color: "var(--texto)", fontSize: "0.9rem" }}>
              {seleccionadas.length} seleccionado(s)
            </span>
            <button
              type="button"
              className="boton boton-primario"
              onClick={() => setConfirmarSalidaMasiva(true)}
            >
              Registrar salida ({seleccionadas.length})
            </button>
          </div>
        )}

        <div style={{ flex: 1, minHeight: 0 }}>
          <Tabla<IngresoActivoResumen>
            columnas={columnas}
            filas={filas}
            filtrosPorColumna
            seleccionMultiple
            onSeleccionCambia={setSeleccionadas}
          />
        </div>
        <p style={{ color: "var(--muted)", margin: 0 }}>{total} adentro</p>
      </div>

      {modalNuevoIngreso && (
        <NuevoIngresoModal
          onRegistrado={recargar}
          onCerrar={() => setModalNuevoIngreso(false)}
        />
      )}

      {confirmarSalidaMasiva && (
        <Modal titulo="Confirmar salida" onCerrar={() => setConfirmarSalidaMasiva(false)}>
          <p style={{ marginTop: 0 }}>
            ¿Registrar la salida de {seleccionadas.length} persona(s)?
          </p>
          <ul style={{ margin: "0 0 1rem", paddingLeft: "1.2rem", color: "var(--muted)" }}>
            {seleccionadas.map((fila) => (
              <li key={fila.registro_id}>{fila.contratista_nombre}</li>
            ))}
          </ul>
          <div style={{ display: "flex", justifyContent: "flex-end", gap: "0.5rem" }}>
            <button
              type="button"
              className="boton"
              onClick={() => setConfirmarSalidaMasiva(false)}
              disabled={procesando}
            >
              Cancelar
            </button>
            <button
              type="button"
              className="boton boton-primario"
              onClick={confirmarSalidasSeleccionadas}
              disabled={procesando}
            >
              {procesando ? "Registrando…" : "Confirmar"}
            </button>
          </div>
        </Modal>
      )}
    </div>
  );
}
