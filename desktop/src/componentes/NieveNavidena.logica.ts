// Cálculos de la nieve navideña, sin DOM (ver `NieveNavidena.tsx`).

export interface Copo {
  // Posición horizontal, de 0 a 1 del ancho.
  x: number;
  // Desfase de 0 a 1: dónde arranca su caída y su vaivén.
  fase: number;
  // Caídas completas por segundo.
  velocidad: number;
  // Radio en px CSS.
  radio: number;
  // Opacidad propia, de 0 a 1 (se multiplica por la del tema).
  alfa: number;
  // Cuánto se mece de lado, en fracción del ancho.
  vaivenAncho: number;
  vaivenRitmo: number;
}

// Generador con semilla fija (mulberry32): los mismos copos en cada
// arranque, sin saltos.
function azarConSemilla(semilla: number): () => number {
  let estado = semilla >>> 0;
  return () => {
    estado = (estado + 0x6d2b79f5) >>> 0;
    let t = estado;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

// Entre 25 y 45 s para cruzar la ventana: caída lenta, que no distraiga.
export function crearCopos(cantidad: number, semilla = 1225): Copo[] {
  const azar = azarConSemilla(semilla);
  return Array.from({ length: cantidad }, () => ({
    x: azar(),
    fase: azar(),
    velocidad: 1 / (25 + azar() * 20),
    radio: 1 + azar() * 1.8,
    alfa: 0.35 + azar() * 0.65,
    vaivenAncho: 0.004 + azar() * 0.01,
    vaivenRitmo: 0.3 + azar() * 0.5,
  }));
}

// Cuántos copos para una ventana de `ancho` x `alto` px CSS: la misma
// densidad en cualquier tamaño, con techo para monitores grandes.
export function cantidadCopos(ancho: number, alto: number): number {
  return Math.min(140, Math.max(20, Math.round((ancho * alto) / 16000)));
}

// Dónde está el copo a los `segundos`, en px CSS. Arranca un poco por
// encima del borde de arriba y sale por abajo, para no aparecer de golpe.
export function posicionCopo(
  copo: Copo,
  segundos: number,
  ancho: number,
  alto: number,
): { x: number; y: number } {
  const recorrido = alto + 12;
  const avance = (copo.fase + segundos * copo.velocidad) % 1;
  const vaiven =
    copo.vaivenAncho *
    Math.sin(segundos * copo.vaivenRitmo + copo.fase * 2 * Math.PI);
  return { x: (copo.x + vaiven) * ancho, y: avance * recorrido - 6 };
}

// Tema oscuro según el color de fondo: la nieve blanca no se ve en claro.
export function esFondoOscuro(colorCss: string): boolean {
  const canales = colorCss
    .match(/\d+(\.\d+)?/g)
    ?.slice(0, 3)
    .map(Number);
  if (!canales || canales.length < 3) return true;
  const [r, g, b] = canales;
  return 0.2126 * r + 0.7152 * g + 0.0722 * b < 128;
}
