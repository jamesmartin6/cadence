import { defineConfig, devices } from "@playwright/test";

// Runs against the real stack, not mocks: a real WASM CRDT engine, a real relay server,
// and a real Postgres. Playwright only manages the frontend dev server (below) --
// relay-server (+ its database) must already be running at VITE_RELAY_HTTP_URL
// (defaults to http://localhost:3001; see README "Running the tests" section).
export default defineConfig({
  testDir: "./e2e",
  timeout: 30_000,
  fullyParallel: false,
  retries: 0,
  reporter: "list",
  use: {
    baseURL: "http://localhost:5173",
    trace: "retain-on-failure",
  },
  webServer: {
    command: "npm run dev",
    url: "http://localhost:5173",
    reuseExistingServer: !process.env.CI,
    timeout: 30_000,
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
});
