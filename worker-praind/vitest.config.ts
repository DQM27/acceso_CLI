import { cloudflareTest } from "@cloudflare/vitest-plugin";
import { defineConfig } from "vitest/config";

// Las pruebas corren en el mismo motor que Cloudflare, con un R2 local.
// Las salidas a Supabase y a la página del QR se reemplazan en cada prueba.
export default defineConfig({
  plugins: [cloudflareTest({ wrangler: { configPath: "./wrangler.jsonc" } })],
});
