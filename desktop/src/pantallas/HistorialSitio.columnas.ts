import type { ColDef } from "ag-grid-community";
import { columnaFecha, columnaGafete, columnaHora } from "../componentes/columnasComunes";

/** Lo que tienen en común los ingresos de la unidad que la PC copia de la
 * nube (proveedores y visitas por correo). */
interface IngresoSitio {
  cedula: string;
  nombre: string;
  placa: string | null;
  gafete_numero: number | null;
  fecha_hora_ingreso: string;
  fecha_hora_salida: string | null;
  usuario_ingreso_nombre: string | null;
  usuario_salida_nombre: string | null;
}

type Campo<T> = ColDef<T>["field"];

/** Columnas de los historiales de proveedores y de visitas por correo:
 * idénticas salvo la tercera (`detalle`: la empresa o el motivo). Los
 * `colId`/`field` no cambian: son la clave del layout guardado. */
export function columnasIngresoSitio<T extends IngresoSitio>(detalle: ColDef<T>): ColDef<T>[] {
  return [
    {
      field: "cedula" as Campo<T>,
      headerName: "Cédula",
      flex: 0.9,
      minWidth: 110,
      cellStyle: { textAlign: "left" },
    },
    {
      field: "nombre" as Campo<T>,
      headerName: "Nombre",
      flex: 1.4,
      minWidth: 160,
      cellStyle: { textAlign: "left" },
    },
    detalle,
    {
      field: "placa" as Campo<T>,
      headerName: "Placa",
      flex: 0.8,
      minWidth: 100,
      valueFormatter: (p) => p.value ?? "Caminando",
    },
    columnaGafete<T>({ flex: 0.7 }),
    columnaFecha<T>("fecha_entrada", (fila) => fila.fecha_hora_ingreso, { flex: 0.9, minWidth: 100 }),
    columnaHora<T>("hora_entrada", (fila) => fila.fecha_hora_ingreso, {
      headerName: "Entrada",
      flex: 0.8,
      minWidth: 85,
    }),
    columnaHora<T>("hora_salida", (fila) => fila.fecha_hora_salida, {
      headerName: "Salida",
      flex: 0.8,
      minWidth: 85,
      valueFormatter: (p) => p.value || "—",
    }),
    {
      field: "usuario_ingreso_nombre" as Campo<T>,
      headerName: "Dio ingreso",
      flex: 1.1,
      minWidth: 120,
      valueFormatter: (p) => p.value ?? "—",
    },
    {
      field: "usuario_salida_nombre" as Campo<T>,
      headerName: "Dio salida",
      flex: 1.1,
      minWidth: 120,
      valueFormatter: (p) => p.value ?? "—",
    },
  ];
}
