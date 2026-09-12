import { z } from "zod";

// Zod puede probar compilación dinámica incluso si luego usa su intérprete.
// El modo sin JIT evita ese intento bajo la CSP estricta de esta aplicación.
z.config({ jitless: true });

export { z };
