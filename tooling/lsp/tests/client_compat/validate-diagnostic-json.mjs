#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import Ajv from "ajv";
import addFormats from "ajv-formats";

if (process.argv.length < 3) {
  console.error("[validate-diagnostic-json] usage: node validate-diagnostic-json.mjs <schema> <file...>");
  process.exit(1);
}

const [, , schemaPath, ...files] = process.argv;

function loadSchema(schemaFile) {
  const schemaText = fs.readFileSync(schemaFile, "utf8");
  return JSON.parse(schemaText);
}

function parseJsonLike(content, fileName) {
  const trimmed = content.trim();
  if (trimmed === "") {
    return [];
  }

  try {
    const json = JSON.parse(trimmed);
    if (Array.isArray(json)) {
      return json;
    }
    return [json];
  } catch (error) {
    const lines = trimmed.split(/\r?\n/).filter((line) => line.trim() !== "");
    return lines.map((line, index) => {
      try {
        return JSON.parse(line);
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        throw new Error(`JSONL parse error ${fileName}:${index + 1}: ${message}`);
      }
    });
  }
}

const schema = loadSchema(schemaPath);
const ajv = new Ajv({ allErrors: true, strict: false });
addFormats(ajv);
const validate = ajv.compile(schema);

let hasError = false;

const severityMap = new Map([
  ["error", 1],
  ["warning", 2],
  ["warn", 2],
  ["info", 3],
  ["information", 3],
  ["hint", 4],
]);

function normalizeDiagnostic(entry) {
  if (!entry || typeof entry !== "object") {
    return entry;
  }

  const clone = JSON.parse(JSON.stringify(entry));

  if (clone && typeof clone === "object" && Array.isArray(clone.diagnostics)) {
    return clone.diagnostics.map((item) => normalizeDiagnostic(item)).filter(Boolean);
  }

  if (typeof clone.severity === "string") {
    const mapped = severityMap.get(clone.severity.toLowerCase());
    if (mapped) {
      clone.severity = mapped;
    }
  }

  if (!clone.codes && typeof clone.code === "string") {
    clone.codes = [clone.code];
  }

  if (!clone.primary && clone.location && typeof clone.location === "object") {
    const loc = clone.location;
    const startLine = loc.start_line ?? loc.startLine ?? loc.line ?? 0;
    const startCol = loc.start_col ?? loc.startCol ?? loc.column ?? 0;
    const endLine = loc.end_line ?? loc.endLine ?? loc.line ?? startLine;
    const endCol = loc.end_col ?? loc.endCol ?? loc.column ?? startCol;
    clone.primary = {
      file: loc.file ?? loc.path ?? "<unknown>",
      start_line: Number.parseInt(startLine, 10) || 0,
      start_col: Number.parseInt(startCol, 10) || 0,
      end_line: Number.parseInt(endLine, 10) || Number.parseInt(startLine, 10) || 0,
      end_col: Number.parseInt(endCol, 10) || Number.parseInt(startCol, 10) || 0,
    };
  }

  return clone;
}

for (const file of files) {
  const absolute = path.resolve(file);
  if (!fs.existsSync(absolute)) {
    console.error(`[validate-diagnostic-json] skip (not found): ${absolute}`);
    continue;
  }
  const content = fs.readFileSync(absolute, "utf8");
  let entries;
  try {
    entries = parseJsonLike(content, absolute);
  } catch (error) {
    hasError = true;
    console.error(`[validate-diagnostic-json] ${error instanceof Error ? error.message : String(error)}`);
    continue;
  }

  const flattened = [];
  const queue = [...entries];
  while (queue.length > 0) {
    const current = queue.shift();
    const normalized = normalizeDiagnostic(current);
    if (Array.isArray(normalized)) {
      queue.push(...normalized);
    } else if (normalized && typeof normalized === "object") {
      flattened.push(normalized);
    }
  }

  flattened.forEach((entry, index) => {
    const ok = validate(entry);
    if (!ok) {
      hasError = true;
      console.error(`[validate-diagnostic-json] schema violation: ${absolute} (entry ${index + 1})`);
      for (const err of validate.errors ?? []) {
        console.error(`  - ${err.instancePath || "<root>"}: ${err.message}`);
      }
    }
  });
}

if (hasError) {
  process.exit(1);
}
