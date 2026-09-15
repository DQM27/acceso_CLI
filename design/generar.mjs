import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const leer = p => readFileSync(resolve(root, p), 'utf8').replace(/\r\n/g, '\n');
const d = JSON.parse(leer('design/brisas.json'));
const comprobar = process.argv.includes('--check');
const salidas = new Map();
const aviso = 'Generado desde design/brisas.json. Editar la fuente y ejecutar node design/generar.mjs.';
if (JSON.stringify(Object.keys(d.temas.light)) !== JSON.stringify(Object.keys(d.temas.dark))) throw Error('Los temas deben tener los mismos roles.');
for (const tema of Object.values(d.temas)) for (const [rol, valor] of Object.entries(tema)) {
  if (!['sombra-panel', 'velo'].includes(rol) && !/^#[0-9A-F]{6}$/.test(valor)) throw Error(`Color inválido: ${rol}`);
}
const declaraciones = tema => Object.entries(tema).map(([k,v]) => `  --${k}: ${v};`).join('\n');
const metricas = `  --fuente: ${d.tipografia.familia};
  --fuente-base: ${d.tipografia.base}px;
  --fuente-control: ${d.tipografia.control}px;
  --fuente-titulo: ${d.tipografia.titulo}px;
  --peso-control: ${d.tipografia.pesoControl};
  --radio: ${d.formas.panel}px;
  --radio-chico: ${d.formas.control}px;
  --radio-capsula: ${d.formas.capsula}px;
${Object.entries(d.espaciado).map(([k,v]) => `  --espacio-${k}: ${v}px;`).join('\n')}
${Object.entries(d.controles).filter(([k]) => k !== 'duracion').map(([k,v]) => `  --control-${k}: ${v}px;`).join('\n')}
  --duracion: ${d.controles.duracion}ms;`;
const css = `/* ${aviso} */
:root {
${metricas}
}
:root, [data-theme="light"] {
${declaraciones(d.temas.light)}
  color-scheme: light;
}
@media (prefers-color-scheme: dark) {
  :root:not([data-theme="light"]) {
${declaraciones(d.temas.dark)}
    color-scheme: dark;
  }
}
[data-theme="dark"] {
${declaraciones(d.temas.dark)}
  color-scheme: dark;
}
`;
salidas.set('design/paleta.js', `// ${aviso}\nwindow.Brisas = ${JSON.stringify(d, null, 2)};\n`);
salidas.set('desktop/src/diseno.css', css);
salidas.set('desktop/src/controles.css', leer('design/controles.css'));
salidas.set('web/src/diseno.css', css);
salidas.set('web/src/controles.css', leer('design/controles.css'));
salidas.set('design/brisas.css', css + '\n' + leer('design/controles.css'));

// Android/mobile YA NO sale de acá (2026-09-15) -- tiene su propia identidad
// visual (estilo "Kash": acento verde-azulado, fondo lavanda, esquinas más
// redondeadas), a pedido explícito y a propósito distinta de desktop/web.
// Ver `mobile/android/.../DisenoMovil.kt`, que ahora se edita a mano.

let errores = 0;
for (const [ruta, contenido] of salidas) {
  const destino = resolve(root, ruta);
  if (existsSync(destino) && leer(ruta) === contenido) continue;
  if (comprobar) { console.error(`Desactualizado: ${ruta}`); errores++; }
  else { mkdirSync(dirname(destino), {recursive:true}); writeFileSync(destino, contenido); console.log(`Generado: ${ruta}`); }
}
if (errores) process.exitCode = 1;
else console.log(comprobar ? 'Diseño sincronizado.' : 'Diseño generado.');
