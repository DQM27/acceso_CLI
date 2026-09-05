import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const { temas } = JSON.parse(readFileSync(new URL('./brisas.json', import.meta.url), 'utf8'));
const luminancia = hex => hex.slice(1).match(/../g)
  .map(v => parseInt(v, 16) / 255)
  .map(v => v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4)
  .reduce((s,v,i) => s + v * [0.2126, 0.7152, 0.0722][i], 0);
const contraste = (a,b) => {
  const valores = [luminancia(a), luminancia(b)].sort((x,y) => y-x);
  return (valores[0]+0.05)/(valores[1]+0.05);
};
let pares = 0;
for (const [nombre, t] of Object.entries(temas)) {
  const revisar = (texto, fondo, minimo) => {
    const ratio = contraste(t[texto], t[fondo]); pares++;
    assert.ok(ratio >= minimo, `${nombre}: ${texto} sobre ${fondo} = ${ratio.toFixed(2)}, requiere ${minimo}`);
  };
  for (const fondo of ['fondo','panel','panel-lateral','panel-suave','campo-fondo','elevado']) {
    for (const texto of ['texto','muted']) revisar(texto, fondo, 4.5);
  }
  for (const rol of ['acento-relleno','acento-hover','acento-presionado']) revisar('sobre-acento', rol, 4.5);
  revisar('sobre-acento-indicador', 'acento', 4.5);
  for (const rol of ['acento','exito','advertencia','error','info']) {
    revisar(rol, 'panel', 4.5);
    revisar(rol, rol+'-suave', 4.5);
  }
  for (const fondo of ['campo-fondo','panel']) revisar('borde-fuerte', fondo, 3);
}
console.log(`${pares} combinaciones de contraste verificadas en claro y oscuro.`);
