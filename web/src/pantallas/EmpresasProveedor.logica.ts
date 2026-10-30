import type { EmpresaProveedorPanel } from "../api/empresasProveedor";
import { plegarTexto } from "../api/historialFiltros";

export type FiltroRevision = "por_revisar" | "vigentes" | "fusionadas";

export type EstadoRevision = "por_revisar" | "revisada" | "fusionada";

export function estadoRevision(empresa: EmpresaProveedorPanel): EstadoRevision {
  if (empresa.fusionada_en) return "fusionada";
  return empresa.revisada_en ? "revisada" : "por_revisar";
}

/** "Por revisar": creadas en la portería y todavía sin confirmar ni fusionar.
 * "Vigentes": todas las que no se fusionaron. "Fusionadas": los nombres
 * viejos, para ver qué se fusionó en qué. */
export function filtrarPorRevision(
  empresas: readonly EmpresaProveedorPanel[],
  filtro: FiltroRevision,
): EmpresaProveedorPanel[] {
  return empresas.filter((empresa) => {
    const estado = estadoRevision(empresa);
    if (filtro === "por_revisar") return estado === "por_revisar";
    if (filtro === "fusionadas") return estado === "fusionada";
    return estado !== "fusionada";
  });
}

function compacto(texto: string): string {
  return plegarTexto(texto).replace(/[^a-z0-9]/g, "");
}

/** Qué tan parecidos son dos nombres (0 a 1): pares de letras en común, sin
 * mayúsculas, tildes, espacios ni signos; si uno contiene al otro (4 letras o
 * más), 1. El mismo criterio que usan las apps al sugerir
 * (`similitud_nombres` en src/texto.rs). */
export function similitudNombres(a: string, b: string): number {
  const [x, y] = [compacto(a), compacto(b)];
  const [corto, largo] = x.length <= y.length ? [x, y] : [y, x];
  if (corto.length >= 4 && largo.includes(corto)) return 1;
  const pares = (texto: string) =>
    Array.from({ length: Math.max(texto.length - 1, 0) }, (_, i) =>
      texto.slice(i, i + 2),
    );
  const deX = pares(x);
  const deY = pares(y);
  if (deX.length === 0 || deY.length === 0) return 0;
  const total = deX.length + deY.length;
  let comunes = 0;
  for (const par of deX) {
    const posicion = deY.indexOf(par);
    if (posicion >= 0) {
      deY.splice(posicion, 1);
      comunes += 1;
    }
  }
  return (2 * comunes) / total;
}

/** Desde qué similitud se sugiere como "probablemente la misma" (igual que en
 * las apps). */
export const UMBRAL_PARECIDA = 0.5;

/** Las empresas en las que se puede fusionar `origen`: de la misma unidad, sin
 * fusionar y distintas de ella. Primero las parecidas (de la más a la menos),
 * después el resto por nombre. */
export function destinosPosibles(
  origen: EmpresaProveedorPanel,
  empresas: readonly EmpresaProveedorPanel[],
): { empresa: EmpresaProveedorPanel; parecida: boolean }[] {
  return empresas
    .filter(
      (empresa) =>
        empresa.sitio_id === origen.sitio_id &&
        empresa.id !== origen.id &&
        !empresa.fusionada_en,
    )
    .map((empresa) => ({
      empresa,
      similitud: similitudNombres(origen.nombre, empresa.nombre),
    }))
    .sort((a, b) => {
      const parecidaA = a.similitud >= UMBRAL_PARECIDA;
      const parecidaB = b.similitud >= UMBRAL_PARECIDA;
      if (parecidaA !== parecidaB) return parecidaA ? -1 : 1;
      if (parecidaA && a.similitud !== b.similitud)
        return b.similitud - a.similitud;
      return a.empresa.nombre.localeCompare(b.empresa.nombre, "es");
    })
    .map(({ empresa, similitud }) => ({
      empresa,
      parecida: similitud >= UMBRAL_PARECIDA,
    }));
}
