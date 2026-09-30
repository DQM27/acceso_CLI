import type { ContratistaConEstado, EstadoAcceso } from "../api/contratistas";

/** Qué tan grave es el estado: define el color de la celda. */
export type NivelEstado = "ok" | "aviso" | "bloqueo";

/** Texto y nivel de cada resultado de las reglas de acceso (las calcula la
 * vista `panel_contratistas_estado` con las mismas reglas que el núcleo). */
export const ESTADOS_ACCESO: Record<EstadoAcceso, { texto: string; nivel: NivelEstado }> = {
  PERMITIDO: { texto: "Puede entrar", nivel: "ok" },
  PERMITIDO_CON_ADVERTENCIA: { texto: "Puede entrar (PRAIND por vencer)", nivel: "aviso" },
  PRAIND_VENCIDO: { texto: "PRAIND vencida", nivel: "bloqueo" },
  PRAIND_NO_REGISTRADO: { texto: "Sin PRAIND registrada", nivel: "bloqueo" },
  SIN_ACCESO: { texto: "Acceso denegado", nivel: "bloqueo" },
  EMPRESA_INACTIVA: { texto: "Empresa inactiva", nivel: "bloqueo" },
};

function dias(n: number): string {
  return n === 1 ? "1 día" : `${n} días`;
}

/** Cómo va la PRAIND, en palabras: "Vencida hace 3 días", "Vence en 5 días",
 * "Vence hoy", "Vigente", "No requiere", "Sin registrar". */
export function textoEstadoPraind(
  fila: Pick<ContratistaConEstado, "estado_praind" | "dias_para_vencer">,
): string {
  const restantes = fila.dias_para_vencer;
  switch (fila.estado_praind) {
    case "NO_REQUIERE":
      return "No requiere";
    case "SIN_REGISTRO":
      return "Sin registrar";
    case "VENCIDA":
      return restantes === null ? "Vencida" : `Vencida hace ${dias(-restantes)}`;
    case "POR_VENCER":
      if (restantes === 0) return "Vence hoy";
      return restantes === null ? "Por vencer" : `Vence en ${dias(restantes)}`;
    case "VIGENTE":
      return "Vigente";
  }
}

/** Nivel de la PRAIND para colorear su celda. */
export function nivelPraind(estado: ContratistaConEstado["estado_praind"]): NivelEstado | null {
  if (estado === "VENCIDA" || estado === "SIN_REGISTRO") return "bloqueo";
  if (estado === "POR_VENCER") return "aviso";
  return null;
}

/** Color de texto de cada nivel (variables de `diseno.css`). */
export const COLOR_NIVEL: Record<NivelEstado, string> = {
  ok: "var(--exito)",
  aviso: "var(--advertencia)",
  bloqueo: "var(--error)",
};
