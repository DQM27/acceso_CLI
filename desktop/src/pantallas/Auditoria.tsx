import { useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import PantallaEncabezado from "../componentes/PantallaEncabezado";
import {
  etiquetaCampo,
  etiquetaEntidad,
  listarAuditoria,
  listarAuditoriaGafetes,
  valorPresentable,
} from "../api";
import type { CambioAuditado, IncidenteGafete } from "../api";
import { fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";

/** Una fila de la grilla — de `auditoria_cambios` (Contratista/Empresa/
 * Usuario) o de `gafetes_incidentes` (marcar perdido/resolver), ya
 * normalizadas a la misma forma de presentación. Sólo lleva los campos que
 * la grilla realmente pinta — ninguna columna lee `entidad`/`campo`/
 * `valor_anterior`/`valor_nuevo`/`usuario_id`/`entidad_id` crudos, así que no
 * hace falta inventarles un valor a las filas de gafetes (que no tienen
 * esos campos en ese sentido). */
interface FilaAuditoria {
  id: string;
  fecha_hora: string;
  usuario_nombre: string;
  entidad_actual: string;
  entidad_texto: string;
  campo_texto: string;
  anterior_texto: string;
  nuevo_texto: string;
}

/** Para cada (entidad, entidad_id), el `entidad_nombre` de la fila con la
 * `fecha_hora` más reciente — `items` ya viene ordenado por fecha DESC
 * desde el núcleo, así que la primera fila vista por combinación ya es la
 * más nueva. */
export function nombresActuales(items: CambioAuditado[]): Map<string, string> {
  const nombres = new Map<string, string>();
  for (const item of items) {
    const clave = `${item.entidad}:${item.entidad_id}`;
    if (!nombres.has(clave)) nombres.set(clave, item.entidad_nombre);
  }
  return nombres;
}

export default function Auditoria() {
  const [filas, setFilas] = useState<FilaAuditoria[]>([]);
  const [cargando, setCargando] = useState(true);
  const [busqueda, setBusqueda] = useState("");

  useEffect(() => {
    let vigente = true;
    setCargando(true);
    Promise.all([listarAuditoria(), listarAuditoriaGafetes()])
      .then(([cambios, incidentesGafetes]) => {
        if (!vigente) return;
        const actuales = nombresActuales(cambios);
        const filasCambios: FilaAuditoria[] = cambios.map((item) => ({
          id: `cambio-${item.id}`,
          fecha_hora: item.fecha_hora,
          usuario_nombre: item.usuario_nombre,
          entidad_actual: actuales.get(`${item.entidad}:${item.entidad_id}`) ?? item.entidad_nombre,
          entidad_texto: etiquetaEntidad(item.entidad),
          campo_texto: etiquetaCampo(item.campo),
          anterior_texto: valorPresentable(item.campo, item.valor_anterior),
          nuevo_texto: valorPresentable(item.campo, item.valor_nuevo),
        }));
        const filasGafetes: FilaAuditoria[] = incidentesGafetes.map((incidente) => {
          const numero = `Gafete ${String(incidente.gafete_numero).padStart(2, "0")}`;
          return {
            id: `gafete-${incidente.id}`,
            fecha_hora: incidente.fecha_hora,
            usuario_nombre: incidente.usuario_nombre,
            entidad_actual: numero,
            entidad_texto: "Gafete",
            campo_texto: "Estado",
            anterior_texto: textoEstadoAnterior(incidente),
            nuevo_texto: textoEstadoNuevo(incidente),
          };
        });
        setFilas(
          [...filasCambios, ...filasGafetes].sort((a, b) =>
            b.fecha_hora.localeCompare(a.fecha_hora),
          ),
        );
      })
      .catch((error) => vigente && toast.error(String(error)))
      .finally(() => vigente && setCargando(false));
    return () => {
      vigente = false;
    };
  }, []);

  // useMemo a propósito — mismo motivo que Activos.tsx/Historial.tsx: si
  // `columnas` se recrea en cada render, AG Grid reaplica el orden/ancho
  // literales de acá encima del layout que el usuario ya acomodó.
  const columnas: ColDef<FilaAuditoria>[] = useMemo(
    () => [
      {
        colId: "fecha",
        headerName: "Fecha",
        width: 110,
        valueGetter: (p) => (p.data ? fechaLocalYMD(p.data.fecha_hora) : ""),
        valueFormatter: (p) => (p.value ? textoFechaDDMMYYYY(p.value) : ""),
      },
      {
        colId: "hora",
        headerName: "Hora",
        width: 90,
        valueGetter: (p) => (p.data ? textoHora(p.data.fecha_hora) : ""),
      },
      {
        field: "entidad_actual",
        headerName: "Entidad",
        flex: 1.3,
        minWidth: 160,
        cellStyle: { textAlign: "left" },
      },
      { field: "entidad_texto", headerName: "Tipo", width: 120 },
      { field: "campo_texto", headerName: "Campo", width: 160 },
      {
        field: "anterior_texto",
        headerName: "Valor anterior",
        flex: 1,
        minWidth: 140,
      },
      {
        field: "nuevo_texto",
        headerName: "Valor nuevo",
        flex: 1,
        minWidth: 140,
      },
      {
        field: "usuario_nombre",
        headerName: "Modificado por",
        width: 150,
      },
    ],
    [],
  );

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <PantallaEncabezado titulo="Auditoría" />

      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        <div style={{ flex: 1, minHeight: 0 }}>
          <Tabla<FilaAuditoria>
            id="auditoria"
            columnas={columnas}
            filas={filas}
            busqueda={busqueda}
            controles={
              <div className="campo" style={{ flex: "1 1 16rem" }}>
                Buscar
                <input
                  placeholder="Entidad, campo, valor…"
                  value={busqueda}
                  onChange={(evento) => setBusqueda(evento.target.value)}
                />
              </div>
            }
          />
        </div>
        <p style={{ color: "var(--muted)", margin: 0 }}>
          {cargando ? "Cargando…" : `${filas.length} cambio(s) auditado(s)`}
        </p>
      </div>
    </div>
  );
}

function textoEstadoAnterior(incidente: IncidenteGafete): string {
  return incidente.tipo === "Perdido" ? "Disponible" : "Perdido";
}

function textoEstadoNuevo(incidente: IncidenteGafete): string {
  if (incidente.tipo === "Perdido") {
    return `Perdido — asignado a ${incidente.contratista_nombre ?? "—"}`;
  }
  const motivo = incidente.motivo_resolucion === "Pagado" ? "pagado" : "apareció";
  return `Disponible — ${motivo}`;
}
