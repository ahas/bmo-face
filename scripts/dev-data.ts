// Makes or refreshes the development copy of the data root (docs/development.md).
// Usage: bun scripts/dev-data.ts [--from <data root>] [--to <copy>]
import { devDataRoot, realDataRoot, refreshDevData } from "../src/dev-data";

const args = process.argv.slice(2);
const value = (flag: string) => {
  const at = args.indexOf(flag);

  return at < 0 ? undefined : args[at + 1];
};
const from = value("--from") ?? realDataRoot;
const to = value("--to") ?? devDataRoot;
const report = refreshDevData(from, to);

console.log(`dev-data: ${to} ← ${report.document === "live" ? from : "the project's default"}`);
