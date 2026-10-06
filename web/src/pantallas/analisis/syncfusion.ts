import { useSyncExternalStore } from "react";
import { L10n, loadCldr, registerLicense, setCulture } from "@syncfusion/ej2-base";
import traducciones from "@syncfusion/ej2-locale/src/es.json";
import numberingSystems from "cldr-core/supplemental/numberingSystems.json";
import weekData from "cldr-core/supplemental/weekData.json";
import numeros from "cldr-numbers-full/main/es-CR/numbers.json";
import monedas from "cldr-numbers-full/main/es-CR/currencies.json";
import gregoriano from "cldr-dates-full/main/es-CR/ca-gregorian.json";
import husos from "cldr-dates-full/main/es-CR/timeZoneNames.json";
import "./syncfusion.css";

/**
 * Configuración única de Syncfusion (Essential JS 2) para el panel. Se importa
 * sólo desde la pantalla "Análisis", que se carga bajo demanda: nada de esto
 * (ni su peso) llega al resto del panel.
 *
 * Licencia: la llave va en `VITE_SYNCFUSION_LICENSE` (variable de entorno del
 * build en Cloudflare, o `web/.env.local` en desarrollo), nunca en el
 * repositorio. Syncfusion la valida en el navegador sin conectarse a nada, así
 * que termina dentro del JavaScript publicado como cualquier build con
 * licencia; lo que se evita es versionarla. Sin llave los componentes
 * funcionan igual, con el aviso de licencia de Syncfusion encima.
 */
const llave = import.meta.env.VITE_SYNCFUSION_LICENSE;
if (llave) registerLicense(llave);

/** Cultura de números y fechas (separadores, nombres de meses y días) y de los
 * textos de los componentes. */
export const CULTURA = "es-CR";

loadCldr(numberingSystems, weekData, numeros, monedas, gregoriano, husos);
setCulture(CULTURA);
// Las traducciones oficiales vienen bajo "es"; los componentes las buscan por
// su `locale`, así que se registran con la misma clave que la cultura.
L10n.load({ [CULTURA]: traducciones.es });

// --- Tema claro/oscuro --------------------------------------------------------
//
// El panel fija `data-theme="light"|"dark"` en <html> (`SelectorTema`). El tema
// Fluent 2 de Syncfusion pasa a oscuro con la clase `e-dark-mode` en un
// ancestro; se sincroniza aquí, y los colores de marca y superficies los
// toman de los tokens del panel (`syncfusion.css`), así Syncfusion se ve como
// el resto de la interfaz en los dos modos.

function temaActual(): "light" | "dark" {
  return document.documentElement.getAttribute("data-theme") === "dark" ? "dark" : "light";
}

function suscribirTema(avisar: () => void): () => void {
  const observador = new MutationObserver(avisar);
  observador.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
  return () => observador.disconnect();
}

function aplicarModoOscuro(tema: "light" | "dark") {
  document.documentElement.classList.toggle("e-dark-mode", tema === "dark");
}

aplicarModoOscuro(temaActual());
suscribirTema(() => aplicarModoOscuro(temaActual()));

/** Tema vigente del panel; vuelve a renderizar cuando la persona lo cambia.
 * Los gráficos son SVG con colores propios y lo necesitan para repintarse. */
export function useTemaPanel(): "light" | "dark" {
  return useSyncExternalStore(suscribirTema, temaActual);
}

/** Valor de un token de color del panel (`--texto`, `--borde`...) tal como
 * está resuelto ahora mismo en <html>. */
export function colorToken(nombre: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(nombre).trim();
}
