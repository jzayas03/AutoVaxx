import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, readdirSync, realpathSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
const problems = [];

function normalizedLicense(value) {
  if (typeof value === "string") return value;
  if (Array.isArray(value)) return value.map(normalizedLicense).join(" OR ");
  if (value && typeof value === "object" && "type" in value)
    return String(value.type);
  return "";
}

function isProhibited(license) {
  const value = license.toUpperCase();
  if (value.includes("AGPL") || value.includes("SSPL")) return true;
  if (!value.includes("GPL")) return false;
  return !/(MIT|APACHE|BSD|ISC|ZLIB|UNLICENSE|CC0|MPL)/.test(value);
}

function packageRoots(base) {
  if (!existsSync(base)) return [];
  const roots = [];
  for (const entry of readdirSync(base, { withFileTypes: true })) {
    if (entry.name.startsWith(".")) continue;
    const path = join(base, entry.name);
    if (entry.name.startsWith("@")) {
      for (const child of readdirSync(path, { withFileTypes: true })) {
        roots.push(realpathSync(join(path, child.name)));
      }
    } else {
      roots.push(realpathSync(path));
    }
  }
  return roots;
}

const npmRoots = new Set([
  ...packageRoots(join(root, "node_modules")),
  ...packageRoots(join(root, "node_modules/.pnpm/node_modules")),
]);
let npmCount = 0;
for (const directory of npmRoots) {
  const manifestPath = join(directory, "package.json");
  if (!existsSync(manifestPath)) continue;
  const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
  const license = normalizedLicense(manifest.license);
  npmCount += 1;
  if (!license)
    problems.push(
      `npm ${manifest.name ?? directory}@${manifest.version ?? "unknown"}: missing license metadata`,
    );
  else if (isProhibited(license))
    problems.push(
      `npm ${manifest.name}@${manifest.version}: prohibited/unresolved license ${license}`,
    );
}

const metadata = JSON.parse(
  execFileSync(
    "cargo",
    [
      "metadata",
      "--manifest-path",
      "src-tauri/Cargo.toml",
      "--format-version",
      "1",
      "--locked",
    ],
    { cwd: root, encoding: "utf8", maxBuffer: 20 * 1024 * 1024 },
  ),
);
for (const crate of metadata.packages) {
  const license = normalizedLicense(crate.license);
  if (!license)
    problems.push(
      `crate ${crate.name}@${crate.version}: missing license metadata`,
    );
  else if (isProhibited(license))
    problems.push(
      `crate ${crate.name}@${crate.version}: prohibited/unresolved license ${license}`,
    );
}

if (problems.length) {
  console.error(problems.join("\n"));
  process.exit(1);
}

console.log(
  `License metadata gate passed for ${npmCount} installed npm packages and ${metadata.packages.length} Cargo packages.`,
);
