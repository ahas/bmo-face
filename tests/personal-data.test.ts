import { expect, test } from "bun:test";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { join } from "node:path";

// No tracked file holds personal data: a mail address outside the reserved example domains and the
// development address, a person's home path, a telephone number, a token or a private network address
// (docs/PERSONAL-DATA.md). The check carries the patterns only; no list of addresses, paths or values
// is kept in the repository.

const ROOT = join(import.meta.dir, "..");

// The home folders a neutral example writes, and the owner's own handle, so a person's own is not
// among them.
const NEUTRAL_HOME = ["user", "tester", "qa-admin", "placeholder", "example", "ahas"];

// The mail addresses the repository allows by name (docs/PERSONAL-DATA.md, 「Allowed values」).
const ALLOWED_MAIL = ["dev@ahas.io"];

// The compiled core's bytes written as base64, where a run of letters can take any pattern's shape
// (docs/PERSONAL-DATA.md, 「The check」).
const ENCODED_BYTES = ["src/wasm/bytes.js"];

// Every file git tracks or would track: the committed ones and the new ones it does not ignore.
function tracked(): string[] {
  const output = execFileSync("git", ["ls-files", "--cached", "--others", "--exclude-standard", "-z"], {
    cwd: ROOT,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });

  return output.split("\0").filter((path) => path !== "");
}

// The tracked files that read as text, with their text.
function trackedTexts(): [string, string][] {
  const texts: [string, string][] = [];

  for (const path of tracked().filter((path) => !ENCODED_BYTES.includes(path))) {
    const bytes = readFileSync(join(ROOT, path));

    if (!bytes.includes(0)) texts.push([path, bytes.toString("utf8")]);
  }

  return texts;
}

// Whether a home path's own folder is a neutral example or the owner's rather than someone else's.
function isNeutralHome(folder: string): boolean {
  return NEUTRAL_HOME.includes(folder) || folder.startsWith("agent-");
}

// Whether a domain is the reserved example space, where a neutral example address stands.
function isExampleDomain(domain: string): boolean {
  const lower = domain.toLowerCase();

  return (
    ["example.com", "example.net", "example.org"].some((example) => lower === example || lower.endsWith(`.${example}`)) ||
    [".example", ".test", ".invalid", ".localhost"].some((suffix) => lower.endsWith(suffix))
  );
}

// Whether a mail address is the development address or stands in the reserved example space.
function isAllowedMail(address: string): boolean {
  const lower = address.toLowerCase();
  const domain = lower.split("@")[1] ?? "";

  return ALLOWED_MAIL.includes(lower) || isExampleDomain(domain);
}

test("no tracked file holds personal data", () => {
  const mail = /\b[A-Za-z0-9._%+-]+@[A-Za-z][A-Za-z0-9.-]*\.[A-Za-z]{2,}\b/g;
  const home = /(?:\/home\/|\/Users\/|[A-Za-z]:\\Users\\)([A-Za-z0-9._-]+)/g;
  const telephone = /\b01[016789][-. ]?\d{3,4}[-. ]?\d{4}\b|\+\d{1,3}[-. ]?\d{2,4}[-. ]?\d{3,4}[-. ]?\d{4}\b|\+\d{9,15}\b/g;
  const token = /gh[pousr]_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,}|sk-[A-Za-z0-9_-]{20,}|xox[baprs]-[A-Za-z0-9-]{10,}|AKIA[0-9A-Z]{16}|AIza[0-9A-Za-z_-]{35}/g;
  const privateAddress = /\b(?:10\.\d{1,3}\.\d{1,3}\.\d{1,3}|192\.168\.\d{1,3}\.\d{1,3}|172\.(?:1[6-9]|2\d|3[01])\.\d{1,3}\.\d{1,3})\b|\b(?:f[cd][0-9a-f]{2}|fe80):/gi;
  const found: string[] = [];

  for (const [path, text] of trackedTexts()) {
    for (const match of text.matchAll(mail)) {
      if (!isAllowedMail(match[0])) found.push(`${path}: mail address ${match[0]}`);
    }

    for (const match of text.matchAll(home)) {
      if (!isNeutralHome(match[1] ?? "")) found.push(`${path}: home path ${match[0]}`);
    }

    for (const match of text.matchAll(telephone)) found.push(`${path}: telephone number ${match[0]}`);
    for (const match of text.matchAll(token)) found.push(`${path}: token ${match[0]}`);
    for (const match of text.matchAll(privateAddress)) found.push(`${path}: private address ${match[0]}`);
  }

  expect(found).toEqual([]);
});
