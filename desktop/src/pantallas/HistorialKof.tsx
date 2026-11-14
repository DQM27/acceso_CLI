import type { ColDef } from "ag-grid-community";
import { listarGafetesProvisionalesHistorialSitio } from "../api/gafetesProvisionales";
import type { PrestamoGafeteProvisionalHistorialSitio } from "../api/gafetesProvisionales";
import { columnaFecha, columnaHora } from "../componentes/columnasComunes";
import type { PropsHistorialClase } from "./Historial.logica";
import HistorialSitio from "./HistorialSitio";

const COLUMNAS: ColDef<PrestamoGafeteProvisionalHistorialSitio>[] = [
  {
    field: "encargado_nombre",
    headerName: "Encargado",
    flex: 1.4,
    minWidth: 160,
    cellStyle: { textAlign: "left" },
  },
  {
    field: "encargado_codigo_empleado",
    headerName: "Código empleado",
    flex: 0.9,
    minWidth: 120,
  },
  // Sin "S/G": un préstamo KOF siempre es de un gafete.
  { field: "gafete_numero", type: "numero", headerName: "Gafete", flex: 0.7, minWidth: 90 },
  columnaFecha("fecha_entrega", (fila) => fila.fecha_hora_entrega, { flex: 0.9, minWidth: 100 }),
  columnaHora("hora_entrega", (fila) => fila.fecha_hora_entrega, {
    headerName: "Entrega",
    flex: 0.8,
    minWidth: 85,
  }),
  columnaHora("hora_devolucion", (fila) => fila.fecha_hora_devolucion, {
    headerName: "Devolución",
    flex: 0.8,
    minWidth: 90,
    valueFormatter: (p) => p.value || "—",
  }),
  {
    field: "usuario_entrega_nombre",
    headerName: "Entregó",
    flex: 1.1,
    minWidth: 130,
  },
  {
    field: "usuario_devolucion_nombre",
    headerName: "Recibió",
    flex: 1.1,
    minWidth: 130,
    valueFormatter: (p) => p.value ?? "—",
  },
];

/** Historial de gafetes provisionales KOF: lee
 * `prestamos_gafete_provisional_historial_sitio`, una caché que sólo
 * sincroniza la PC. */
export default function HistorialKof(props: PropsHistorialClase) {
  return (
    <HistorialSitio<PrestamoGafeteProvisionalHistorialSitio>
      {...props}
      listar={listarGafetesProvisionalesHistorialSitio}
      columnas={COLUMNAS}
      idGrilla="gafetes-provisionales-historial"
      placeholderBusqueda="Nombre o código de empleado…"
      nombreArchivo="historial-kof"
      tituloExportacion="Historial de Gafetes Provisionales KOF"
    />
  );
}
