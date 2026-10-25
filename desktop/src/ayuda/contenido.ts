import { armarManual } from "../../../supabase/functions/_shared/manual/manual.ts";

// El manual entero (`docs/manual/`) va dentro del programa: la ayuda funciona
// sin internet. Se incluyen también el móvil y el panel porque algunos
// procesos enlazan a ellos; el índice del escritorio muestra sólo los suyos.
const crudos = import.meta.glob<string>("../../../docs/manual/**/*.md", {
  query: "?raw",
  import: "default",
  eager: true,
});

/** De `../../../docs/manual/escritorio/x.md` a `escritorio/x.md`. */
export function rutaEnManual(ruta: string): string {
  return ruta.replace(/^.*docs\/manual\//, "");
}

export const manual = armarManual(
  Object.fromEntries(Object.entries(crudos).map(([ruta, contenido]) => [rutaEnManual(ruta), contenido])),
);
