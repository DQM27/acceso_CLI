import { z } from "zod";

// Sólo alta -- a diferencia de `FormularioEmpresa`, el núcleo no expone
// renombrar una empresa proveedora (ver docs/features-futuras/plan-control-proveedores.md,
// "mismo CRUD mínimo... crear, buscar, listar, activar/desactivar"). Activar/
// desactivar no se hace en el escritorio: es sólo del panel web (decisión del
// dueño 2026-10-24).
export const esquema = z.object({
  nombre: z.string().min(1, "El nombre es obligatorio"),
});
