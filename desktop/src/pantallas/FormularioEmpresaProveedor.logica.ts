import { z } from "zod";

// Sólo alta -- a diferencia de `FormularioEmpresa`, el núcleo no expone
// renombrar una empresa proveedora (ver docs/features-futuras/plan-control-proveedores.md,
// "mismo CRUD mínimo... crear, buscar, listar, activar/desactivar"). Activar/
// desactivar se hace inline en la grilla, mismo patrón que `Empresas.tsx`.
export const esquema = z.object({
  nombre: z.string().min(1, "El nombre es obligatorio"),
});
