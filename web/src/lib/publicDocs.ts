import { existsSync, readFileSync } from "node:fs";
import path from "node:path";

export function readPublicDoc(filename: string): string {
  const candidates = [
    path.join(process.cwd(), "docs", "public", filename),
    path.join(process.cwd(), "..", "docs", "public", filename),
  ];
  const documentPath = candidates.find((candidate) => existsSync(candidate));

  if (!documentPath) {
    throw new Error(`Public document is missing from docs/public: ${filename}`);
  }

  return readFileSync(documentPath, "utf8");
}
