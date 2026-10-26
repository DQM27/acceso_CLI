import type { ColDef } from "ag-grid-community";
import type { ClaseOtra, MovimientoClase } from "../api/historialClases";
import type { DefinicionColumnaExport } from "../exportacion";
import { fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";
import { textoDispositivo } from "./Historial.logica";

/** Cómo se rotula cada dato en cada clase -- los mismos nombres que en el
 * historial de escritorio (`desktop/src/pantallas/HistorialProveedores.tsx`,
 * `HistorialCorreo.tsx`, `HistorialKof.tsx`). `detalle` y `placa` en
 * `null`: la clase no tiene esa columna. */
interface Etiquetas {
  titulo: string;
  archivo: string;
  identificacion: string;
  nombre: string;
  detalle: string | null;
  placa: string | null;
  fechaEntrada: string;
  horaEntrada: string;
  fechaSalida: string;
  horaSalida: string;
  usuarioEntrada: string;
  usuarioSalida: string;
  /** Lo que se ve en la salida mientras sigue adentro (o el gafete sin devolver). */
  sinSalida: string;
}

export const ETIQUETAS_CLASE: Record<ClaseOtra, Etiquetas> = {
  proveedores: {
    titulo: "Historial de proveedores",
    archivo: "historial-proveedores",
    identificacion: "Cédula",
    nombre: "Nombre",
    detalle: "Empresa",
    placa: "Placa",
    fechaEntrada: "Fecha ingreso",
    horaEntrada: "Hora ingreso",
    fechaSalida: "Fecha salida",
    horaSalida: "Hora salida",
    usuarioEntrada: "Dio ingreso",
    usuarioSalida: "Dio salida",
    sinSalida: "Activo",
  },
  correo: {
    titulo: "Historial de visitas por correo",
    archivo: "historial-correo",
    identificacion: "Cédula",
    nombre: "Nombre",
    detalle: "Motivo",
    placa: "Placa",
    fechaEntrada: "Fecha ingreso",
    horaEntrada: "Hora ingreso",
    fechaSalida: "Fecha salida",
    horaSalida: "Hora salida",
    usuarioEntrada: "Dio ingreso",
    usuarioSalida: "Dio salida",
    sinSalida: "Activo",
  },
  kof: {
    titulo: "Historial de gafetes KOF",
    archivo: "historial-kof",
    identificacion: "Código empleado",
    nombre: "Encargado",
    detalle: null,
    placa: null,
    fechaEntrada: "Fecha entrega",
    horaEntrada: "Entrega",
    fechaSalida: "Fecha devolución",
    horaSalida: "Devolución",
    usuarioEntrada: "Entregó",
    usuarioSalida: "Recibió",
    sinSalida: "Prestado",
  },
};

/** Una sola definición por columna para el Excel, el CSV y el PDF, con el
 * mismo `colId` que la grilla (`columnasClase`): así se exporta lo que la
 * persona tiene visible, en su orden. */
export function definicionesExportClase(clase: ClaseOtra): DefinicionColumnaExport<MovimientoClase>[] {
  const e = ETIQUETAS_CLASE[clase];
  const definiciones: (DefinicionColumnaExport<MovimientoClase> | null)[] = [
    { colId: "sitio_nombre", etiqueta: "Unidad operativa", valor: (f) => f.sitio_nombre ?? "" },
    { colId: "identificacion", etiqueta: e.identificacion, izquierda: true, valor: (f) => f.identificacion ?? "" },
    { colId: "nombre", etiqueta: e.nombre, izquierda: true, valor: (f) => f.nombre },
    e.detalle ? { colId: "detalle", etiqueta: e.detalle, izquierda: true, valor: (f) => f.detalle ?? "" } : null,
    { colId: "dispositivo_entrada_tipo", etiqueta: "Dispositivo", valor: (f) => textoDispositivo(f.dispositivo_entrada_tipo) },
    e.placa ? { colId: "placa", etiqueta: e.placa, valor: (f) => f.placa ?? "" } : null,
    {
      colId: "gafete_numero",
      etiqueta: "Gafete",
      valor: (f) => (f.gafete_numero == null ? "S/G" : String(f.gafete_numero)),
    },
    { colId: "fecha_entrada", etiqueta: e.fechaEntrada, valor: (f) => textoFechaDDMMYYYY(fechaLocalYMD(f.hora_entrada)) },
    { colId: "hora_entrada", etiqueta: e.horaEntrada, valor: (f) => textoHora(f.hora_entrada) },
    {
      colId: "fecha_salida",
      etiqueta: e.fechaSalida,
      valor: (f) => (f.hora_salida ? textoFechaDDMMYYYY(fechaLocalYMD(f.hora_salida)) : e.sinSalida),
    },
    { colId: "hora_salida", etiqueta: e.horaSalida, valor: (f) => (f.hora_salida ? textoHora(f.hora_salida) : e.sinSalida) },
    { colId: "usuario_entrada_nombre", etiqueta: e.usuarioEntrada, izquierda: true, valor: (f) => f.usuario_entrada_nombre ?? "" },
    { colId: "usuario_salida_nombre", etiqueta: e.usuarioSalida, izquierda: true, valor: (f) => f.usuario_salida_nombre ?? "—" },
  ];
  return definiciones.filter((d): d is DefinicionColumnaExport<MovimientoClase> => d !== null);
}

/** Columnas de la grilla de la clase. Las de hora no filtran (como en
 * contratistas): para acotar por tiempo están las de fecha, que filtran el
 * instante completo. */
export function columnasClase(clase: ClaseOtra): ColDef<MovimientoClase>[] {
  const e = ETIQUETAS_CLASE[clase];
  const columnas: (ColDef<MovimientoClase> | null)[] = [
    { field: "sitio_nombre", headerName: "Unidad operativa", flex: 1.3, minWidth: 160 },
    { colId: "identificacion", field: "identificacion", headerName: e.identificacion, flex: 1.1, minWidth: 120 },
    { field: "nombre", headerName: e.nombre, flex: 1.4, minWidth: 160, cellStyle: { textAlign: "left" } },
    e.detalle ? { colId: "detalle", field: "detalle", headerName: e.detalle, flex: 1.2, minWidth: 130 } : null,
    {
      field: "dispositivo_entrada_tipo",
      headerName: "Dispositivo",
      flex: 0.9,
      minWidth: 110,
      valueFormatter: (p) => textoDispositivo(p.value ?? null),
    },
    e.placa ? { colId: "placa", field: "placa", headerName: e.placa, flex: 0.9, minWidth: 100 } : null,
    {
      field: "gafete_numero",
      type: "numero",
      headerName: "Gafete",
      flex: 0.8,
      minWidth: 90,
      valueFormatter: (p) => (p.value == null ? "S/G" : String(p.value)),
    },
    {
      colId: "fecha_entrada",
      type: "fecha",
      headerName: e.fechaEntrada,
      flex: 1.3,
      minWidth: 140,
      valueGetter: (p) => (p.data ? fechaLocalYMD(p.data.hora_entrada) : ""),
      valueFormatter: (p) => (p.value ? textoFechaDDMMYYYY(p.value) : ""),
    },
    {
      colId: "hora_entrada",
      headerName: e.horaEntrada,
      flex: 1,
      minWidth: 110,
      filter: false,
      valueGetter: (p) => (p.data ? textoHora(p.data.hora_entrada) : ""),
    },
    {
      colId: "fecha_salida",
      type: "fecha",
      headerName: e.fechaSalida,
      flex: 1.3,
      minWidth: 140,
      valueGetter: (p) => (p.data ? (p.data.hora_salida ? fechaLocalYMD(p.data.hora_salida) : e.sinSalida) : ""),
      valueFormatter: (p) => (p.value === e.sinSalida || !p.value ? (p.value ?? "") : textoFechaDDMMYYYY(p.value)),
    },
    {
      colId: "hora_salida",
      headerName: e.horaSalida,
      flex: 1,
      minWidth: 110,
      filter: false,
      valueGetter: (p) => (p.data ? (p.data.hora_salida ? textoHora(p.data.hora_salida) : e.sinSalida) : ""),
    },
    { field: "usuario_entrada_nombre", headerName: e.usuarioEntrada, flex: 1.2, minWidth: 130 },
    { field: "usuario_salida_nombre", headerName: e.usuarioSalida, flex: 1.2, minWidth: 130 },
  ];
  return columnas.filter((c): c is ColDef<MovimientoClase> => c !== null);
}
