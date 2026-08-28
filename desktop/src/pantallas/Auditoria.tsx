import { useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import PantallaEncabezado from "../componentes/PantallaEncabezado";
import { etiquetaCampo, etiquetaEntidad, listarAuditoria, valorPresentable } from "../api";
import type { CambioAuditado } from "../api";

/** Formato de 24 horas a propósito — ver el mismo criterio en Activos.tsx. */
function textoHora(iso: string): string {
  return new Date(iso).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", hour12: false });
}

/** Año-mes-día en hora LOCAL como string ordenable — mismo criterio que
 * Activos.tsx/Historial.tsx (evita que AG Grid infiera "fecha" y dispare el
 * selector nativo del navegador). */
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

type FilaAuditoria = CambioAuditado & {
  entidad_texto: string;
  campo_texto: string;
  anterior_texto: string;
  nuevo_texto: string;
};

export default function Auditoria() {
  const [filas, setFilas] = useState<FilaAuditoria[]>([]);
  const [cargando, setCargando] = useState(true);
  const [busqueda, setBusqueda] = useState("");

  useEffect(() => {
    let vigente = true;
    setCargando(true);
    listarAuditoria()
      .then((items) =>
        vigente &&
        setFilas(
          items.map((item) => ({
            ...item,
            entidad_texto: etiquetaEntidad(item.entidad),
            campo_texto: etiquetaCampo(item.campo),
            anterior_texto: valorPresentable(item.campo, item.valor_anterior),
            nuevo_texto: valorPresentable(item.campo, item.valor_nuevo),
          })),
        ),
      )
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
        field: "entidad_nombre",
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
