#!/usr/bin/env node
// Genera un secreto de activación de dispositivo (mismo formato que
// supabase/functions/admin-provision-device/index.ts) sin llamar a Supabase
// -- útil para provisionar dispositivos de prueba contra el sandbox sin pasar
// por el panel/Google OAuth. Imprime el SQL para insertarlo a mano.
//
// Uso:
//   node scripts/generar_secreto_dispositivo.mjs --sitio "Sitio de prueba" --tipo pc --etiqueta "PC recepcion"
//
// tipo: pc | mobile | visor

import { randomUUID, createHash } from "node:crypto";

function leerArgs(argv) {
  const args = {};
  for (let i = 0; i < argv.length; i += 2) {
    const clave = argv[i]?.replace(/^--/, "");
    args[clave] = argv[i + 1];
  }
  return args;
}

const { sitio, tipo, etiqueta } = leerArgs(process.argv.slice(2));

if (!sitio || !tipo || !etiqueta || !["pc", "mobile", "visor"].includes(tipo)) {
  console.error(
    "Uso: node scripts/generar_secreto_dispositivo.mjs --sitio <nombre> --tipo pc|mobile|visor --etiqueta <texto>",
  );
  process.exit(1);
}

const secret = randomUUID() + randomUUID();
const secretHash = createHash("sha256").update(secret).digest("hex");

console.log("secret (guardalo ahora, no se puede recuperar despues):");
console.log(secret);
console.log();
console.log("secret_hash:");
console.log(secretHash);
console.log();
console.log("SQL para insertar (correlo contra el proyecto de staging, no producción):");
console.log(`
insert into public.sitios (nombre)
values ('${sitio.replace(/'/g, "''")}')
on conflict (nombre) do nothing;

insert into public.dispositivos (sitio_id, tipo, etiqueta, secret_hash)
select id, '${tipo}', '${etiqueta.replace(/'/g, "''")}', '${secretHash}'
from public.sitios
where nombre = '${sitio.replace(/'/g, "''")}';
`);
