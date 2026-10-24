// Tipos para las pruebas del panel. No se publican (.assetsignore).
import type { crearAyudas } from "./ayudas.js";

export function trabajar(crear: typeof crearAyudas): void;
export function fuenteDelTrabajador(crear: typeof crearAyudas): string;
