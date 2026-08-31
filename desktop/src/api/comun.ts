// Espejo de `application::CargaCompleta<T>` del núcleo — resultado de las
// cargas "todo de una vez" que alimentan AG Grid (Historial, Auditoría),
// acotadas por `LIMITE_CARGA_COMPLETA_MAXIMO` para que un dataset sin
// filtro de fecha (o un rango muy abierto) no intente traer años de datos
// en un solo mensaje IPC.

export interface CargaCompleta<T> {
  items: T[];
  /** `true` si el conjunto real supera el tope y se cortó antes de traerlo
   * completo — mostrar un aviso pidiendo acotar el filtro en vez de asumir
   * que la grilla tiene todo. */
  truncado: boolean;
}
