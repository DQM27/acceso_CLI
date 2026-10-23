/**
 * Detector de pedidos que no son una petición de datos sino SQL o un intento
 * de saltarse las reglas del asistente. Corre ANTES de llamar al modelo: lo que
 * detecta ni siquiera le llega, y queda como alerta (quién, qué escribió y por
 * qué) para el dueño en `ia_alertas`.
 *
 * Es deliberadamente específico (sentencias completas, no la palabra suelta
 * "select"): un falso positivo bloquea un mensaje legítimo. El modelo hace de
 * segunda red con su herramienta `reportar_intento`.
 */

const REGLAS: { motivo: string; patron: RegExp }[] = [
  { motivo: "Consulta SQL (SELECT … FROM)", patron: /\bselect\b[\s\S]{1,400}\bfrom\b/i },
  { motivo: "Instrucción SQL (INSERT)", patron: /\binsert\s+into\b/i },
  { motivo: "Instrucción SQL (UPDATE)", patron: /\bupdate\s+["\w.]+\s+set\b/i },
  { motivo: "Instrucción SQL (DELETE)", patron: /\bdelete\s+from\b/i },
  {
    motivo: "Instrucción SQL (DDL)",
    patron: /\b(drop|alter|truncate|create)\s+(table|schema|role|function|view|database|extension|policy|trigger|index)\b/i,
  },
  { motivo: "Instrucción SQL (permisos)", patron: /\b(grant|revoke)\b[\s\S]{1,200}\b(on|to|from)\b/i },
  {
    motivo: "Funciones o esquemas internos de la base",
    patron: /\b(set_config|current_setting|pg_sleep|pg_read_file|pg_catalog|information_schema|service_role)\b/i,
  },
  {
    motivo: "Intento de saltarse las reglas del asistente",
    patron:
      /\b(ignor[aáeé]\w*|olvid[aáeé]\w*|salt[aáeé]\w*|desactiv[aáeé]\w*)\s+(todas\s+)?(tus|las|sus|esas)\s+(instrucciones|reglas|restricciones|limites|límites)\b|\b(prompt|instrucciones)\s+del\s+sistema\b|\bsystem\s+prompt\b|\bmodo\s+(dios|desarrollador|developer|admin)\b/i,
  },
];

/** El motivo por el que el mensaje es sospechoso, o `null` si parece un pedido normal. */
export function detectarIntento(mensaje: string): string | null {
  for (const { motivo, patron } of REGLAS) {
    if (patron.test(mensaje)) return motivo;
  }
  return null;
}

export interface Alerta {
  correo: string;
  origen: "detector" | "modelo" | "parser";
  motivo: string;
  mensaje: string;
  detalle?: string | null;
}

/** Lo que se le contesta a quien lo intentó: sin explicar qué se detectó. */
export const RESPUESTA_A_INTENTO =
  "No ejecuto instrucciones SQL ni comandos. Dígame con palabras qué datos quiere ver y yo armo la vista.";
