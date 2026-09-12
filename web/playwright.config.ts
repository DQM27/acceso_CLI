import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  fullyParallel: true,
  workers: 2,
  reporter: "list",
  use: { baseURL: "http://127.0.0.1:8790", trace: "retain-on-failure" },
  projects: [
    {
      name: "escritorio",
      use: {
        ...devices["Desktop Chrome"],
        viewport: { width: 1440, height: 1000 },
      },
    },
    {
      name: "movil",
      use: { ...devices["Pixel 7"], defaultBrowserType: "chromium" },
    },
  ],
  webServer: {
    command:
      "npx wrangler dev --local --ip 127.0.0.1 --port 8790 --inspector-port 0 --show-interactive-dev-session false",
    url: "http://127.0.0.1:8790",
    reuseExistingServer: !process.env.CI,
    timeout: 60_000,
  },
});
