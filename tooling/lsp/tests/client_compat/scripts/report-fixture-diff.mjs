#!/usr/bin/env node

/**
 * CLI/LSP diff generator.
 *
 * Usage:
 *   node scripts/report-fixture-diff.mjs <suite-label> <run-id> [cases-file]
 *
 * Example:
 *   node scripts/report-fixture-diff.mjs diag-w4 20280430-w4-diag-cli-lsp
 *
 * This script reads dual-write CLI outputs under
 * `reports/dual-write/front-end/w4-diagnostics/<run-id>/lsp_*` and compares
 * the normalized diagnostics array with the corresponding LSP fixture declared
 * in `docs/plans/rust-migration/appendix/w4-diagnostic-cases.txt`
 * (`# lsp-fixture: ...`). A unified diff is written to
 * `reports/.../<run-id>/lsp/<case>.diff`.
 */

import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const repoRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../../../../../",
);

const args = process.argv.slice(2);
if (args.length < 2) {
  console.error(
    "Usage: node scripts/report-fixture-diff.mjs <suite-label> <run-id> [cases-file]",
  );
  process.exit(1);
}

const [, runId, casesPathArg] = args;
const runDir = path.join(
  repoRoot,
  "reports",
  "dual-write",
  "front-end",
  "w4-diagnostics",
  runId,
);

if (!fs.existsSync(runDir)) {
  console.error(`[report-fixture-diff] Run directory not found: ${runDir}`);
  process.exit(1);
}

const defaultCasesPath = path.join(
  repoRoot,
  "docs",
  "plans",
  "rust-migration",
  "appendix",
  "w4-diagnostic-cases.txt",
);
const casesPath = casesPathArg
  ? path.resolve(process.cwd(), casesPathArg)
  : defaultCasesPath;

if (!fs.existsSync(casesPath)) {
  console.error(`[report-fixture-diff] Cases file not found: ${casesPath}`);
  process.exit(1);
}

const caseFixtures = parseCaseFixtures(casesPath);
const caseDirs = fs
  .readdirSync(runDir, { withFileTypes: true })
  .filter((entry) => entry.isDirectory() && entry.name.startsWith("lsp_"))
  .map((entry) => entry.name)
  .sort();

if (caseDirs.length === 0) {
  console.warn(
    `[report-fixture-diff] No lsp_* directories found under ${runDir}`,
  );
  process.exit(0);
}

const diffDir = path.join(runDir, "lsp");
fs.mkdirSync(diffDir, { recursive: true });

let errorCount = 0;
for (const caseName of caseDirs) {
  const fixtureRel = caseFixtures.get(caseName);
  if (!fixtureRel) {
    console.warn(
      `[report-fixture-diff] Missing # lsp-fixture entry for case ${caseName}`,
    );
    errorCount += 1;
    continue;
  }

  let fixturePath = path.resolve(repoRoot, fixtureRel);
  if (!fs.existsSync(fixturePath)) {
    fixturePath = path.resolve(
      repoRoot,
      "tooling",
      "lsp",
      "tests",
      "client_compat",
      "fixtures",
      fixtureRel,
    );
  }
  if (!fs.existsSync(fixturePath)) {
    console.warn(
      `[report-fixture-diff] Fixture not found for ${caseName}: ${fixturePath}`,
    );
    errorCount += 1;
    continue;
  }

  const cliPath = path.join(runDir, caseName, "diagnostics.rust.json");
  if (!fs.existsSync(cliPath)) {
    console.warn(
      `[report-fixture-diff] CLI diagnostics missing for ${caseName}: ${cliPath}`,
    );
    errorCount += 1;
    continue;
  }

  try {
    const fixtureDiagnostics = normalizeDiagnostics(fixturePath);
    const cliDiagnostics = normalizeDiagnostics(cliPath);
    const diffText = buildUnifiedDiff(
      fixtureDiagnostics,
      cliDiagnostics,
      fixturePath,
      cliPath,
    );
    const diffPath = path.join(diffDir, `${caseName}.diff`);
    fs.writeFileSync(diffPath, diffText, "utf8");
    console.log(
      `[report-fixture-diff] Wrote diff for ${caseName} -> ${diffPath}`,
    );
  } catch (error) {
    console.warn(
      `[report-fixture-diff] Failed to diff ${caseName}: ${String(error)}`,
    );
    errorCount += 1;
  }
}

if (errorCount > 0) {
  process.exitCode = 1;
}

function parseCaseFixtures(filePath) {
  const entries = new Map();
  const lines = fs.readFileSync(filePath, "utf8").split(/\r?\n/);
  let pendingFixture = null;
  for (const line of lines) {
    const caseMatch = line.match(/^([A-Za-z0-9_]+)::/);
    if (caseMatch) {
      const name = caseMatch[1];
      if (pendingFixture) {
        entries.set(name, pendingFixture);
        pendingFixture = null;
      }
      continue;
    }
    const trimmed = line.trim();
    if (trimmed.startsWith("# lsp-fixture:")) {
      const fixturePath = trimmed.split(":")[1]?.trim();
      if (fixturePath) {
        pendingFixture = fixturePath;
      }
    }
  }
  return entries;
}

function normalizeDiagnostics(filePath) {
  const raw = JSON.parse(fs.readFileSync(filePath, "utf8"));
  if (Array.isArray(raw)) {
    return raw;
  }
  if (Array.isArray(raw.diagnostics)) {
    return raw.diagnostics;
  }
  return raw;
}

function buildUnifiedDiff(fixtureData, cliData, fixturePath, cliPath) {
  const fixtureText = `${JSON.stringify(fixtureData, null, 2)}\n`;
  const cliText = `${JSON.stringify(cliData, null, 2)}\n`;

  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "cli-lsp-"));
  const fixtureTmp = path.join(tmpDir, "fixture.json");
  const cliTmp = path.join(tmpDir, "cli.json");
  fs.writeFileSync(fixtureTmp, fixtureText);
  fs.writeFileSync(cliTmp, cliText);

  const diff = spawnSync("diff", ["-u", fixtureTmp, cliTmp], {
    encoding: "utf8",
  });

  fs.rmSync(tmpDir, { recursive: true, force: true });

  if (diff.error && diff.error.code === "ENOENT") {
    throw new Error(
      "diff command not found. Please install GNU diffutils or use a system with diff available.",
    );
  }

  if (!diff.stdout?.trim()) {
    return `# ${path.basename(fixturePath)} vs ${path.basename(
      cliPath,
    )}\n# No differences detected (normalized diagnostics)\n`;
  }

  return `# ${fixturePath} vs ${cliPath}\n${diff.stdout}`;
}
