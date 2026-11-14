/** Capa de una vista que queda montada aunque no se vea (Historial por
 * clase, las tres grillas de Visitas): se apilan una encima de la otra y
 * sólo se muestra la elegida, así cambiar de vista no destruye la grilla ni
 * pierde scroll y filtros. Ver `.capa-vista` en index.css (oculta: sin
 * clics ni foco). Estaba repetida en Historial y Visitas. */
export function claseCapaVista(visible: boolean): string {
  return visible ? "capa-vista" : "capa-vista capa-vista-oculta";
}
