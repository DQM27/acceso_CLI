import { armarManual } from "../../../supabase/functions/_shared/manual/manual.ts";

// El manual entero (`docs/manual/`) va dentro del panel, con el mismo lector
// que el escritorio: las tres apps, porque quien administra también ayuda a
// los puestos de control.
const crudos = import.meta.glob<string>("../../../docs/manual/**/*.md", {
  query: "?raw",
  import: "default",
  eager: true,
});

/** De `../../../docs/manual/panel/x.md` a `panel/x.md`. */
export function rutaEnManual(ruta: string): string {
  return ruta.replace(/^.*docs\/manual\//, "");
}

export const manual = armarManual(
  Object.fromEntries(Object.entries(crudos).map(([ruta, contenido]) => [rutaEnManual(ruta), contenido])),
);
