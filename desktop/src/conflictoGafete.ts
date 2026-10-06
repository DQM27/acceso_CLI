import type { ConflictoGafeteActivo } from "./api/nube";

/**
 * Aviso para un movimiento con gafete que la nube rechazó porque otro equipo
 * de esta unidad ya tiene ese gafete activo (`nube::ConflictoGafeteActivo`).
 * A diferencia de los conflictos simétricos entre unidades, acá Postgres ya
 * decidió: el movimiento de ESTE equipo es el que no quedó válido, así que el
 * aviso lo dice con esa certeza. `hora` ya viene formateada.
 */
export function mensajeConflictoGafete(conflicto: ConflictoGafeteActivo, hora: string): string {
  const { nombre, gafete_numero: gafete } = conflicto;
  const cierre = "no quedó registrado en la nube — otro dispositivo de este sitio ya lo tiene asignado.";
  switch (conflicto.tipo) {
    case "proveedor":
      return `El ingreso del proveedor ${nombre} con gafete ${gafete} (${hora}) ${cierre}`;
    case "por_correo":
      return `El ingreso por correo de ${nombre} con gafete de visita ${gafete} (${hora}) ${cierre}`;
    case "visita":
      return `El check-in de visita de ${nombre} con gafete de visita ${gafete} (${hora}) ${cierre}`;
    case "provisional_kof":
      return `El préstamo del gafete provisional ${gafete} a ${nombre} (${hora}) ${cierre}`;
    case "contratista":
      return `El ingreso de ${nombre} con gafete ${gafete} (${hora}) ${cierre}`;
  }
}
