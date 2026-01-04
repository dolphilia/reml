import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    globals: true,
    reporters: ["default"],
    include: ["tests/**/*.test.ts"],
    environment: "node",
    coverage: {
      enabled: false
    }
  },
});
