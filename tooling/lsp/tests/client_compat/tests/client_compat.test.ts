import { describe, expect, it } from "vitest";
import { collectCodes, collectStructuredHints, readDiagnostics } from "../client-v2.js";
import { convertToV1 } from "../client-v1.js";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const currentDir = dirname(fileURLToPath(import.meta.url));
const fixturesDir = join(currentDir, "..", "fixtures");

describe("client compatibility scaffolding", () => {
  it("loads V1 diagnostics without throwing", () => {
    const diagnostics = convertToV1(fixturesDir, "diagnostic-sample.json");
    expect(diagnostics).toBeInstanceOf(Array);
  });

  it("loads V2 diagnostics and extracts codes", () => {
    const diagnostics = readDiagnostics(fixturesDir, "diagnostic-v2-sample.json");
    const codes = collectCodes(diagnostics);
    expect(codes).toBeInstanceOf(Set);
  });

  it("collects structured hints array", () => {
    const diagnostics = readDiagnostics(fixturesDir, "diagnostic-v2-sample.json");
    const hints = collectStructuredHints(diagnostics);
    expect(hints).toBeInstanceOf(Array);
  });
});
