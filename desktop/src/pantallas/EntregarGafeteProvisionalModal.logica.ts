import type { EncargadoRuta } from "../api/rutas";
import { coincideBusqueda } from "../busqueda";

export const MAX_RESULTADOS = 5;

/** Encargados cuyo nombre o código de empleado contienen todas las palabras
 * de `texto` (sin distinguir tildes ni mayúsculas). El catálogo es chico y
 * ya viene completo (`listarEncargadosRutaSeleccionables`), así que se
 * filtra acá en vez de buscar del lado del servidor. */
export function filtrarEncargados(
  encargados: EncargadoRuta[],
  texto: string,
  maximo: number = MAX_RESULTADOS,
): EncargadoRuta[] {
  return encargados
    .filter((encargado) =>
      coincideBusqueda(texto, `${encargado.nombre} ${encargado.codigo_empleado}`),
    )
    .slice(0, maximo);
}
