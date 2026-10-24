export type Seccion =
  | "dispositivos"
  | "historial"
  | "adentro"
  | "contratistas"
  | "usuarios"
  | "sesiones"
  | "avisos"
  | "asistente";

/** Ruta real de cada sección -- `Sidebar` arma sus `NavLink` con esto y
 * `Shell` compara `location.pathname` contra el mismo valor para decidir
 * qué mostrar, así las dos fuentes no pueden desincronizarse en silencio. */
export function rutaSeccion(id: Seccion): string {
  return `/${id}`;
}
