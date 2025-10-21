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

  entries.forEach((entry, index) => {
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
