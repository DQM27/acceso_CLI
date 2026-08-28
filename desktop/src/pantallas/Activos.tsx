import { useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import type { ColDef, ICellRendererParams } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import Modal from "../componentes/Modal";
import PantallaEncabezado from "../componentes/PantallaEncabezado";
import { listarIngresosActivos, mensajeMotivoDenegacion, registrarSalida } from "../api";
import type { IngresoActivoResumen } from "../api";

function textoMedio(medio: IngresoActivoResumen["medio_ingreso"]): string {
  return medio === "Vehiculo" ? "Vehículo" : "Caminando";
}

/** Formato de 24 horas a propósito (hour12: false) — sin esto
 * toLocaleTimeString usa AM/PM según el locale del sistema. */
function textoHora(iso: string): string {
  return new Date(iso).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", hour12: false });
}

/** Año-mes-día en hora LOCAL (no UTC) como string ordenable ("2026-08-28")
 * — año-mes-día en vez de `toLocaleDateString()` para que el filtro/orden
 * de la columna funcione como texto plano cronológico, sin volver a pasar
 * por `Date` (que interpretaría "2026-08-28" como medianoche UTC y podría
 * mostrar el día anterior en un huso horario negativo como Costa Rica). */
function fechaLocalYMD(iso: string): string {
  const d = new Date(iso);
  const mes = String(d.getMonth() + 1).padStart(2, "0");
  const dia = String(d.getDate()).padStart(2, "0");
  return `${d.getFullYear()}-${mes}-${dia}`;
}

function textoFechaDDMMYYYY(ymd: string): string {
  const [anio, mes, dia] = ymd.split("-");
  return `${dia}/${mes}/${anio}`;
}

/** Texto plano del estado — separado del componente visual `EstadoAcceso`
 * para que la columna tenga un `field` real: sin eso no aparece en el
 * selector "Columnas ▾" (que sólo lista columnas con `field`) ni el filtro
 * por columna puede buscar sobre ella. */
function textoEstado(fila: IngresoActivoResumen): string {
  const r = fila.resultado_acceso;
  if (r === "Permitido") return "Al día";
  if (r === "PermitidoConAdvertencia") return "PRAIND próximo a vencer";
  return mensajeMotivoDenegacion(r.Denegado);
}

function colorEstado(fila: IngresoActivoResumen): string {
  const r = fila.resultado_acceso;
  if (r === "Permitido") return "var(--exito)";
  if (r === "PermitidoConAdvertencia") return "var(--advertencia)";
  return "var(--error)";
}

function EstadoAcceso({ fila }: { fila: IngresoActivoResumen }) {
  return (
    <span className="chip" style={{ ["--chip-color" as string]: colorEstado(fila) }}>
      {textoEstado(fila)}
    </span>
  );
}

type FilaActiva = IngresoActivoResumen & { estado_texto: string };

export default function Activos({
  refrescarSenal,
  onAbrirNuevoIngreso,
  onAbrirSalida,
}: {
  /** Los modales de Nuevo Ingreso y Salida viven en el Shell (se disparan
   * desde cualquier pantalla vía Ctrl+Shift+N/S, no sólo desde acá) —
   * este número sube cada vez que registran algo, para que la grilla se
   * refresque aunque ya estuviera montada. */
  refrescarSenal?: number;
  onAbrirNuevoIngreso: () => void;
  onAbrirSalida: () => void;
}) {
  const [filas, setFilas] = useState<FilaActiva[]>([]);
  const [total, setTotal] = useState(0);
  const [busqueda, setBusqueda] = useState("");
  const [seleccionadas, setSeleccionadas] = useState<FilaActiva[]>([]);
  const [confirmarSalidaMasiva, setConfirmarSalidaMasiva] = useState(false);
  const [procesando, setProcesando] = useState(false);

  const recargar = useCallback(() => {
    return listarIngresosActivos().then((pagina) => {
      setFilas(pagina.items.map((item) => ({ ...item, estado_texto: textoEstado(item) })));
      setTotal(pagina.total);
      setSeleccionadas([]);
    });
  }, []);

  useEffect(() => {
    let vigente = true;
    recargar().catch((error) => vigente && toast.error(String(error)));
    return () => {
      vigente = false;
    };
  }, [recargar, refrescarSenal]);

  // useCallback a propósito: esta función se cierra dentro de una celda de
  // `columnas` — sin identidad estable, `columnas` (memoizado más abajo)
  // se recrearía en cada render igual, y con eso AG Grid reasignaría el
  // orden/ancho originales del código encima de lo que el usuario acomodó
  // (ver el comentario junto a `columnas`).
  const salidaIndividual = useCallback(
    async (id: number) => {
      try {
        await registrarSalida(id);
        recargar();
      } catch (error) {
        toast.error(String(error));
      }
    },
    [recargar],
  );

  async function confirmarSalidasSeleccionadas() {
    setProcesando(true);
    try {
      for (const fila of seleccionadas) {
        await registrarSalida(fila.registro_id);
      }
      setConfirmarSalidaMasiva(false);
      await recargar();
    } catch (error) {
      toast.error(String(error));
    } finally {
      setProcesando(false);
    }
  }

  // useMemo a propósito: si `columnas` se recrea en cada render (ej. cada
  // vez que `recargar()` trae datos nuevos), AG Grid recibe un `columnDefs`
  // "nuevo" y reaplica el orden/ancho literales de acá encima de lo que el
  // usuario ya había acomodado a mano — el layout persistido en
  // `Tabla`/localStorage quedaba pisado en el siguiente refresco.
  const columnas: ColDef<FilaActiva>[] = useMemo(
    () => [
      { field: "cedula", headerName: "Cédula", width: 120, cellStyle: { textAlign: "left" } },
      {
        field: "contratista_nombre",
        headerName: "Nombre",
        flex: 1.6,
        minWidth: 170,
        cellStyle: { textAlign: "left" },
      },
      { field: "empresa_nombre", headerName: "Empresa", flex: 1.1, minWidth: 140 },
      { field: "tipo_ingreso", headerName: "Tipo", width: 100 },
      {
        field: "medio_ingreso",
        headerName: "Medio",
        width: 100,
        valueFormatter: (p) => textoMedio(p.value),
      },
      {
        field: "gafete_numero",
        headerName: "Gafete",
        width: 90,
        valueFormatter: (p) => (p.value == null ? "S/G" : String(p.value)),
      },
      {
        colId: "fecha_ingreso",
        headerName: "Fecha",
        width: 110,
        // `valueGetter` (no `field`) a propósito, igual que Hora: si la
        // columna lee el string ISO crudo, AG Grid la infiere como fecha y
        // el filtro flotante termina siendo el selector nativo de
        // fecha+hora del navegador (`datetime-local`) en vez de un texto
        // simple — igual que le pasaba a Hora. `fechaLocalYMD` mantiene el
        // orden cronológico correcto como texto ("2026-08-28" ordena bien
        // sin volver a pasar por `Date`); `valueFormatter` sólo reacomoda
        // ese mismo string a DD/MM/AAAA para mostrar, sin re-parsearlo.
        valueGetter: (p) => (p.data ? fechaLocalYMD(p.data.fecha_hora_ingreso) : ""),
        valueFormatter: (p) => (p.value ? textoFechaDDMMYYYY(p.value) : ""),
      },
      {
        colId: "hora_ingreso",
        headerName: "Hora",
        width: 90,
        // `valueGetter` (no `field`+`valueFormatter`) a propósito: si lee
        // el string ISO crudo, AG Grid vuelve a inferirla como fecha (con
        // el mismo problema de la columna Fecha). Devolviendo ya el texto
        // "HH:MM", la columna se filtra/ordena como texto plano — mismo
        // comportamiento que el resto de columnas de texto, sin ícono raro.
        valueGetter: (p) => (p.data ? textoHora(p.data.fecha_hora_ingreso) : ""),
      },
      { field: "usuario_ingreso_nombre", headerName: "Dio ingreso", width: 130 },
      {
        field: "estado_texto",
        headerName: "Estado",
        width: 170,
        cellRenderer: (p: ICellRendererParams<FilaActiva>) =>
          p.data ? <EstadoAcceso fila={p.data} /> : null,
      },
      {
        headerName: "Acción",
        width: 90,
        filter: false,
        sortable: false,
        // Sin pinned a propósito: una columna fijada reserva el ancho del
        // scrollbar vertical aunque no haga falta scroll, dejando un
        // espacio en blanco justo antes de ella — con las 10 columnas ya
        // entrando sin scroll horizontal (ver el ajuste de anchos previo),
        // fijarla no aporta nada y sí ese espacio no deseado.
        cellRenderer: (p: ICellRendererParams<FilaActiva>) =>
          p.data ? (
            <button
              type="button"
              className="boton"
              style={{ padding: "0.15rem 0.55rem", fontSize: "0.78rem" }}
              onClick={() => salidaIndividual(p.data!.registro_id)}
            >
              Salida
            </button>
          ) : null,
      },
    ],
    [salidaIndividual],
  );

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <PantallaEncabezado titulo="Ingresos activos" />

      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
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
          <Tabla<FilaActiva>
            id="activos"
            columnas={columnas}
            filas={filas}
            busqueda={busqueda}
            seleccionMultiple
            onSeleccionCambia={setSeleccionadas}
            controles={
              <>
                <button className="boton" title="Ctrl+Shift+N" onClick={onAbrirNuevoIngreso}>
                  + Nuevo ingreso
                </button>
                <div className="campo" style={{ flex: "1 1 16rem" }}>
                  Buscar
                  <input
                    placeholder="Cédula, nombre, empresa…"
                    value={busqueda}
                    onChange={(evento) => setBusqueda(evento.target.value)}
                  />
                </div>
              </>
            }
            accionesDerecha={
              <button className="boton" title="Ctrl+Shift+S" onClick={onAbrirSalida}>
                Salida
              </button>
            }
          />
        </div>
        <p style={{ color: "var(--muted)", margin: 0 }}>{total} adentro</p>
      </div>

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
