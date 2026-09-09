import { useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import type { ColDef, ICellRendererParams } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import { useBarraEstado } from "../contexto/BarraEstadoContexto";
import {
  listarVisitasActivas,
  registrarEntradaVisita,
  registrarSalidaVisita,
  verificarCheckInVisita,
} from "../api";
import type { MovimientoVisitaActivoResumen, PreparacionVisita } from "../api";
import { fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";

type CheckIn =
  | { tipo: "ninguno" }
  | { tipo: "verificando"; cedula: string }
  | { tipo: "encontrada"; cedula: string; preparacion: PreparacionVisita }
  | { tipo: "sin-cita"; cedula: string; mensaje: string };

/**
 * Check-in de visitas por cédula (`docs/plan-control-visitas.md`) --
 * mucho más simple que `NuevoIngresoModal` a propósito: no hay buscador con
 * lista flotante porque no hay nada que buscar por texto parcial, la cédula
 * es exacta. Un solo panel arriba (verificar → confirmar) y la tabla de
 * quienes están adentro ahora mismo debajo, sin modal — el dominio entero es
 * más chico que el de contratistas (ver domain::cita), no hace falta la
 * misma ceremonia.
 */
export default function Visitas() {
  const [cedula, setCedula] = useState("");
  const [checkIn, setCheckIn] = useState<CheckIn>({ tipo: "ninguno" });
  const [gafeteTexto, setGafeteTexto] = useState("");
  const [enviando, setEnviando] = useState(false);
  const [mensaje, setMensaje] = useState<string | null>(null);

  const [filas, setFilas] = useState<MovimientoVisitaActivoResumen[]>([]);
  const [cargando, setCargando] = useState(true);

  useBarraEstado(cargando ? "Cargando…" : `${filas.length} visitantes adentro`);

  const recargar = useCallback(() => {
    setCargando(true);
    return listarVisitasActivas()
      .then(setFilas)
      .finally(() => setCargando(false));
  }, []);

  useEffect(() => {
    let vigente = true;
    recargar().catch((error) => vigente && toast.error(String(error)));
    return () => {
      vigente = false;
    };
  }, [recargar]);

  async function verificar() {
    const valor = cedula.trim();
    if (!valor) return;
    setMensaje(null);
    setCheckIn({ tipo: "verificando", cedula: valor });
    setGafeteTexto("");
    try {
      const preparacion = await verificarCheckInVisita(valor);
      setCheckIn({ tipo: "encontrada", cedula: valor, preparacion });
    } catch (error) {
      setCheckIn({ tipo: "sin-cita", cedula: valor, mensaje: String(error) });
    }
  }

  function cancelarCheckIn() {
    setCheckIn({ tipo: "ninguno" });
    setCedula("");
  }

  async function confirmarEntrada() {
    if (checkIn.tipo !== "encontrada") return;
    const numero = gafeteTexto.trim() ? Number.parseInt(gafeteTexto.trim(), 10) : null;
    if (gafeteTexto.trim() && Number.isNaN(numero)) {
      toast.error("Ingrese un número de gafete válido");
      return;
    }
    setEnviando(true);
    try {
      await registrarEntradaVisita(checkIn.cedula, numero);
      setMensaje(`✓ Entrada registrada — ${checkIn.preparacion.visitante.nombre}`);
      cancelarCheckIn();
      await recargar();
    } catch (error) {
      toast.error(String(error));
    } finally {
      setEnviando(false);
    }
  }

  const salida = useCallback(
    async (fila: MovimientoVisitaActivoResumen) => {
      try {
        await registrarSalidaVisita(fila.id);
        await recargar();
      } catch (error) {
        toast.error(String(error));
      }
    },
    [recargar],
  );

  const columnas: ColDef<MovimientoVisitaActivoResumen>[] = useMemo(
    () => [
      { field: "cedula", headerName: "Cédula", flex: 1.1, minWidth: 110, cellStyle: { textAlign: "left" } },
      { field: "nombre", headerName: "Nombre", flex: 1.6, minWidth: 170, cellStyle: { textAlign: "left" } },
      { field: "empresa", headerName: "Empresa", flex: 1.1, minWidth: 130, valueFormatter: (p) => p.value ?? "—" },
      {
        field: "gafete_numero",
        headerName: "Gafete",
        flex: 0.8,
        minWidth: 90,
        valueFormatter: (p) => (p.value == null ? "S/G" : String(p.value)),
      },
      { field: "anfitrion_nombre", headerName: "Anfitrión", flex: 1.2, minWidth: 130 },
      { field: "motivo", headerName: "Motivo", flex: 1.2, minWidth: 130, valueFormatter: (p) => p.value ?? "—" },
      {
        colId: "fecha_entrada",
        headerName: "Fecha",
        flex: 1,
        minWidth: 105,
        valueGetter: (p) => (p.data ? fechaLocalYMD(p.data.fecha_hora_entrada) : ""),
        valueFormatter: (p) => (p.value ? textoFechaDDMMYYYY(p.value) : ""),
      },
      {
        colId: "hora_entrada",
        headerName: "Hora",
        flex: 0.8,
        minWidth: 85,
        valueGetter: (p) => (p.data ? textoHora(p.data.fecha_hora_entrada) : ""),
      },
      {
        headerName: "Acción",
        flex: 0.9,
        minWidth: 90,
        filter: false,
        sortable: false,
        cellRenderer: (p: ICellRendererParams<MovimientoVisitaActivoResumen>) =>
          p.data ? (
            <button
              type="button"
              className="boton"
              style={{ padding: "0.15rem 0.55rem", fontSize: "0.78rem" }}
              onClick={() => salida(p.data!)}
            >
              Salida
            </button>
          ) : null,
      },
    ],
    [salida],
  );

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            gap: "0.75rem",
            padding: "0.85rem",
            border: "1px solid var(--borde)",
            borderRadius: "var(--radio-chico)",
            background: "var(--campo-fondo)",
          }}
        >
          <form
            onSubmit={(evento) => {
              evento.preventDefault();
              verificar();
            }}
            style={{ display: "flex", gap: "0.75rem", alignItems: "flex-end" }}
          >
            <label className="campo" style={{ flex: "0 1 16rem" }}>
              Cédula del visitante
              <input
                value={cedula}
                onChange={(evento) => setCedula(evento.target.value)}
                autoFocus
                placeholder="Escanee o escriba la cédula…"
              />
            </label>
            <button
              type="submit"
              className="boton boton-primario"
              disabled={checkIn.tipo === "verificando" || !cedula.trim()}
            >
              {checkIn.tipo === "verificando" ? "Verificando…" : "Verificar"}
            </button>
          </form>

          {mensaje && <p style={{ color: "var(--exito)", margin: 0 }}>{mensaje}</p>}

          {checkIn.tipo === "sin-cita" && (
            <p className="login-error" role="alert">
              {checkIn.mensaje}
            </p>
          )}

          {checkIn.tipo === "encontrada" && (
            <form
              onSubmit={(evento) => {
                evento.preventDefault();
                confirmarEntrada();
              }}
              style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}
            >
              <div>
                <p style={{ margin: 0, fontWeight: 600, color: "var(--texto)" }}>
                  {checkIn.preparacion.visitante.nombre}
                </p>
                <p style={{ margin: "0.15rem 0 0", color: "var(--muted)", fontSize: "0.85rem" }}>
                  {checkIn.preparacion.visitante.cedula}
                  {checkIn.preparacion.visitante.empresa && ` · ${checkIn.preparacion.visitante.empresa}`}
                  {" · Anfitrión: "}
                  {checkIn.preparacion.cita.anfitrion_nombre}
                  {checkIn.preparacion.cita.motivo && ` · ${checkIn.preparacion.cita.motivo}`}
                </p>
              </div>

              <label className="campo" style={{ flex: "0 1 12rem" }}>
                Número de gafete (opcional)
                <input
                  value={gafeteTexto}
                  onChange={(evento) => setGafeteTexto(evento.target.value.replace(/\D/g, ""))}
                  inputMode="numeric"
                  placeholder="S/G si vacío"
                />
              </label>

              <div style={{ display: "flex", justifyContent: "flex-end", gap: "0.5rem" }}>
                <button type="button" className="boton" onClick={cancelarCheckIn} disabled={enviando}>
                  Cancelar
                </button>
                <button type="submit" className="boton boton-primario" disabled={enviando}>
                  {enviando ? "Registrando…" : "Registrar entrada"}
                </button>
              </div>
            </form>
          )}
        </div>

        <div style={{ flex: 1, minHeight: 0 }}>
          <Tabla<MovimientoVisitaActivoResumen>
            id="visitas-activas"
            columnas={columnas}
            filas={filas}
            filtrosPorColumna
          />
        </div>
      </div>
    </div>
  );
}
