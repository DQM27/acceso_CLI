/// <reference types="vitest/config" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [react(), tailwindcss()],
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
