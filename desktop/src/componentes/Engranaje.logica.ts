/** Trazo SVG de un engranaje centrado en (50, 50) de un lienzo de 100 x 100:
 * `dientes` dientes trapezoidales de radio `radioExterior`, unidos por arcos
 * de radio `radioBase`. Los lados del diente se achican hacia la punta, así
 * se parece a un engranaje y no a una estrella. */
export function trazoEngranaje(dientes: number, radioExterior: number, radioBase: number): string {
  const paso = (2 * Math.PI) / dientes;
  const mitadBase = paso * 0.27;
  const mitadPunta = paso * 0.17;
  const punto = (radio: number, angulo: number) =>
    `${(50 + radio * Math.sin(angulo)).toFixed(2)} ${(50 - radio * Math.cos(angulo)).toFixed(2)}`;

  let trazo = "";
  for (let i = 0; i < dientes; i++) {
    const centro = i * paso;
    trazo += trazo === "" ? "M" : `A${radioBase} ${radioBase} 0 0 1 `;
    trazo += `${punto(radioBase, centro - mitadBase)} `;
    trazo += `L${punto(radioExterior, centro - mitadPunta)} `;
    trazo += `L${punto(radioExterior, centro + mitadPunta)} `;
    trazo += `L${punto(radioBase, centro + mitadBase)} `;
  }
  return `${trazo}A${radioBase} ${radioBase} 0 0 1 ${punto(radioBase, -mitadBase)} Z`;
}
