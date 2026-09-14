/// <reference types="vitest/config" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  // `quietDeps` -- Bootstrap 5.3 todavía usa funciones de color de Sass que
  // Dart Sass reciente marca obsoletas (red()/green()/blue(), etc.); son
  // warnings de la librería, no de nuestro código, así que no hace sentido
  // que inunden cada build.
  css: { preprocessorOptions: { scss: { quietDeps: true } } },
  server: { port: 5174, strictPort: true },
  build: {
    sourcemap: false,
    rolldownOptions: {
      output: {
        codeSplitting: {
          groups: [
            {
              name: "react",
              test: /node_modules[\\/](react|react-dom|scheduler)[\\/]/,
            },
            { name: "supabase", test: /node_modules[\\/]@supabase[\\/]/ },
            { name: "validacion", test: /node_modules[\\/]zod[\\/]/ },
          ],
        },
      },
    },
  },
  test: {
    environment: "jsdom",
    setupFiles: ["./src/pruebas/setup.ts"],
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
