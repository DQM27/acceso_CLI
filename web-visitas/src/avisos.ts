import { useSyncExternalStore } from "react";

/** Avisos breves ("Visita agendada", "Visita cancelada"). Propios y no una
 * librería: la CSP de esta web (`style-src 'self'`) no deja inyectar estilos. */
export type Aviso = { id: number; texto: string; accion?: { texto: string; alPulsar: () => void } };

const DURACION = 6000;
let avisos: Aviso[] = [];
let siguiente = 1;
const oyentes = new Set<() => void>();
const temporizadores = new Map<number, ReturnType<typeof setTimeout>>();

function emitir() {
  for (const oyente of oyentes) oyente();
}

export function cerrarAviso(id: number) {
  clearTimeout(temporizadores.get(id));
  temporizadores.delete(id);
  avisos = avisos.filter((a) => a.id !== id);
  emitir();
}

export function avisar(texto: string, accion?: Aviso["accion"]) {
  const id = siguiente++;
  // Sólo el último: dos avisos apilados en un teléfono tapan la pantalla.
  for (const viejo of avisos) clearTimeout(temporizadores.get(viejo.id));
  avisos = [{ id, texto, accion }];
  temporizadores.set(
    id,
    setTimeout(() => cerrarAviso(id), DURACION),
  );
  emitir();
}

function suscribir(oyente: () => void) {
  oyentes.add(oyente);
  return () => oyentes.delete(oyente);
}

export function useAvisos() {
  return useSyncExternalStore(suscribir, () => avisos);
}
