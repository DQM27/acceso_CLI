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
salidas.set('panel-web/src/diseno.css', css);
salidas.set('panel-web/src/controles.css', leer('design/controles.css'));
salidas.set('design/brisas.css', css + '\n' + leer('design/controles.css'));
const panelPath = 'admin-panel/panel-dispositivos.html';
const panel = leer(panelPath);
const bloque = `<style id="brisas-generado">\n${css}\n${leer('design/controles.css')}</style>`;
salidas.set(panelPath, panel.includes('<style id="brisas-generado">')
  ? panel.replace(/<style id="brisas-generado">[\s\S]*?<\/style>/, bloque)
  : panel.replace('</head>', `${bloque}\n</head>`));

const rgb = hex => hex.slice(1).match(/../g).map(v => parseInt(v, 16)).join(', ');
const rustRoles = { fondo:'background', texto:'text', muted:'muted', acento:'accent', exito:'success', advertencia:'warning', error:'danger', 'borde-fuerte':'border', 'sobre-acento':'selection_foreground' };
let rust = `// ${aviso}\nuse crate::tui::ui_kit::Theme;\nuse ratatui::style::Color;\n\n`;
for (const [nombre, tema] of Object.entries(d.temas)) {
  rust += `pub const ${nombre.toUpperCase()}: Theme = Theme {\n`;
  for (const [rol,campo] of Object.entries(rustRoles)) rust += `    ${campo}: Color::Rgb(${rgb(tema[rol])}),\n`;
  rust += `    selection_background: Color::Rgb(${rgb(tema['acento-relleno'])}),\n    navegacion_pestanas: false,\n};\n\n`;
}
salidas.set('src/diseno_generado.rs', rust.trimEnd() + '\n');

const material = { primary:'acento', onPrimary:'sobre-acento-indicador', primaryContainer:'acento-suave', onPrimaryContainer:'acento', inversePrimary:'acento', secondary:'acento', onSecondary:'sobre-acento-indicador', secondaryContainer:'acento-suave', onSecondaryContainer:'acento', tertiary:'info', onTertiary:'sobre-info', tertiaryContainer:'info-suave', onTertiaryContainer:'info', background:'fondo', onBackground:'texto', surface:'panel', onSurface:'texto', surfaceVariant:'panel-suave', onSurfaceVariant:'muted', surfaceTint:'acento', inverseSurface:'texto', inverseOnSurface:'fondo', error:'error', onError:'sobre-error', errorContainer:'error-suave', onErrorContainer:'error', outline:'borde-fuerte', outlineVariant:'borde', scrim:'fondo', surfaceBright:'elevado', surfaceDim:'fondo', surfaceContainer:'panel', surfaceContainerHigh:'panel-suave', surfaceContainerHighest:'elevado', surfaceContainerLow:'campo-fondo', surfaceContainerLowest:'fondo' };
let kt = `// ${aviso}
package com.brisas.controlacceso

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.runtime.Composable
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Typography
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp
import androidx.compose.material3.Shapes
import androidx.compose.material3.lightColorScheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp

`;
for (const [nombre, tema] of Object.entries(d.temas)) {
  kt += `internal val Brisas${nombre === 'light' ? 'Claro' : 'Oscuro'} = ${nombre}ColorScheme(\n`;
  for (const [campo,rol] of Object.entries(material)) kt += `    ${campo} = Color(0xFF${tema[rol].slice(1)}),\n`;
  kt += ')\n\n';
}
kt += `internal val FormaControlBrisas = RoundedCornerShape(${d.formas.control}.dp)
internal val FormasBrisas = Shapes(
    extraSmall = FormaControlBrisas,
    small = FormaControlBrisas,
    medium = RoundedCornerShape(${d.formas.panel}.dp),
    large = RoundedCornerShape(${d.formas.panel}.dp),
    extraLarge = RoundedCornerShape(${d.formas.panel}.dp),
)
internal val ColorRellenoBrisas: Color
    @Composable get() = if (isSystemInDarkTheme()) Color(0xFF${d.temas.dark['acento-relleno'].slice(1)}) else Color(0xFF${d.temas.light['acento-relleno'].slice(1)})
internal val ColorSobreRellenoBrisas: Color
    @Composable get() = if (isSystemInDarkTheme()) Color(0xFF${d.temas.dark['sobre-acento'].slice(1)}) else Color(0xFF${d.temas.light['sobre-acento'].slice(1)})
internal val EspacioControlBrisas = ${d.controles.horizontal}.dp
internal val AlturaControlBrisas = ${d.controles.tactil}.dp
internal val ColorExitoBrisas: Color
    @Composable get() = if (isSystemInDarkTheme()) Color(0xFF${d.temas.dark.exito.slice(1)}) else Color(0xFF${d.temas.light.exito.slice(1)})
internal val TipografiaBrisas = Typography(
    bodyLarge = TextStyle(fontFamily = FontFamily.SansSerif, fontSize = ${d.tipografia.base}.sp, lineHeight = ${d.tipografia.base * 1.5}.sp),
    bodyMedium = TextStyle(fontFamily = FontFamily.SansSerif, fontSize = ${d.tipografia.base}.sp, lineHeight = ${d.tipografia.base * 1.5}.sp),
    labelLarge = TextStyle(fontFamily = FontFamily.SansSerif, fontSize = ${d.tipografia.control}.sp, fontWeight = FontWeight(${d.tipografia.pesoControl})),
    titleLarge = TextStyle(fontFamily = FontFamily.SansSerif, fontSize = ${d.tipografia.titulo}.sp, fontWeight = FontWeight(${d.tipografia.pesoControl})),
)
`;
salidas.set('mobile/app/app/src/main/java/com/brisas/controlacceso/DisenoGenerado.kt', kt);

let errores = 0;
for (const [ruta, contenido] of salidas) {
  const destino = resolve(root, ruta);
  if (existsSync(destino) && leer(ruta) === contenido) continue;
  if (comprobar) { console.error(`Desactualizado: ${ruta}`); errores++; }
  else { mkdirSync(dirname(destino), {recursive:true}); writeFileSync(destino, contenido); console.log(`Generado: ${ruta}`); }
}
if (errores) process.exitCode = 1;
else console.log(comprobar ? 'Diseño sincronizado.' : 'Diseño generado.');
