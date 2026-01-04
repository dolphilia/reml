#!/usr/bin/env node

/**
 * Combined CI entry point:
 * 1. Runs `vitest run --reporter=default --run`.
 * 2. Generates CLI/LSP diffs via `report-fixture-diff.mjs`.
 *
 * Additional arguments (e.g. `diag-w4 20280430-w4-diag-cli-lsp`) are forwarded
 * to the diff script.
 */

import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";

const projectDir = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: projectDir,
    stdio: "inherit",
    env: process.env,
    ...options,
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

const vitestBin = path.join(
  projectDir,
  "node_modules",
  ".bin",
  process.platform === "win32" ? "vitest.cmd" : "vitest",
);

run(vitestBin, ["run", "--reporter=default", "--run"]);

run(
  process.execPath,
  ["scripts/report-fixture-diff.mjs", ...process.argv.slice(2)],
);
