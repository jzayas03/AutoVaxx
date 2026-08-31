import { readFileSync, readdirSync, statSync } from "node:fs";
import { extname, join, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
const textExtensions = new Set([
  ".rs",
  ".ts",
  ".tsx",
  ".js",
  ".mjs",
  ".json",
  ".toml",
  ".yml",
  ".yaml",
]);
const roots = [
  "src",
  "src-tauri/src",
  "src-tauri/Cargo.toml",
  "src-tauri/tauri.conf.json",
  "src-tauri/capabilities",
  "scripts",
  ".github",
];

function filesAt(path) {
  const absolute = join(root, path);
  if (!statSync(absolute).isDirectory()) return [absolute];
  return readdirSync(absolute).flatMap((entry) => filesAt(join(path, entry)));
}

const files = roots
  .flatMap(filesAt)
  .filter((path) => textExtensions.has(extname(path)));
const failures = [];
const forbiddenNetwork = [
  "fetch(",
  "XMLHttpRequest",
  "reqwest",
  "TcpStream",
  "UdpSocket",
];
const forbiddenTauriAuthority = [
  "plugin-shell",
  "plugin-fs",
  "plugin-http",
  "shell:allow",
  "fs:allow",
  "http:allow",
];
const secretPatterns = [
  /-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----/,
  /(?:api[_-]?key|secret|token)\s*[:=]\s*["'][A-Za-z0-9+/=_-]{20,}["']/i,
];

for (const path of files) {
  const relativePath = relative(root, path).split(sep).join("/");
  const content = readFileSync(path, "utf8");
  if (relativePath !== "scripts/policy-check.mjs") {
    for (const token of forbiddenNetwork) {
      if (content.includes(token))
        failures.push(`${relativePath}: forbidden network primitive ${token}`);
    }
    for (const token of forbiddenTauriAuthority) {
      if (content.includes(token))
        failures.push(
          `${relativePath}: forbidden generic Tauri authority ${token}`,
        );
    }
  }
  for (const pattern of secretPatterns) {
    if (pattern.test(content))
      failures.push(`${relativePath}: possible embedded secret`);
  }
  if (/\b\d{3}-\d{2}-\d{4}\b/.test(content))
    failures.push(`${relativePath}: SSN-shaped value is prohibited`);
}

const rustRoot = readFileSync(join(root, "src-tauri/src/lib.rs"), "utf8");
for (const requiredGuard of [
  'all(feature = "production", feature = "synthetic-only")',
  'all(feature = "production", feature = "dev-auth")',
  'all(feature = "production", not(feature = "sqlcipher"))',
  'all(feature = "production", not(feature = "windows-secret-store"))',
  'all(feature = "production", not(feature = "production-logging"))',
  'all(feature = "production", not(target_os = "windows"))',
  'all(feature = "real-phi", not(feature = "production"))',
  'all(feature = "real-phi", not(feature = "approved-schema"))',
  'all(feature = "real-phi", not(feature = "hardened-security-config"))',
]) {
  if (!rustRoot.includes(requiredGuard))
    failures.push(
      `src-tauri/src/lib.rs: missing production guard ${requiredGuard}`,
    );
}

const capability = readFileSync(
  join(root, "src-tauri/capabilities/main.json"),
  "utf8",
);
const parsedCapability = JSON.parse(capability);
if (JSON.stringify(parsedCapability.permissions) !== JSON.stringify([])) {
  failures.push(
    "src-tauri/capabilities/main.json: Phase 1 shell must not grant core plugin permissions",
  );
}

const tauriConfig = JSON.parse(
  readFileSync(join(root, "src-tauri/tauri.conf.json"), "utf8"),
);
const expectedCsp =
  "default-src 'self' customprotocol: asset:; connect-src ipc: http://ipc.localhost; img-src 'self' asset: http://asset.localhost; style-src 'self' 'unsafe-inline'; font-src 'self'; object-src 'none'; frame-src 'none'; base-uri 'none'; form-action 'none'";
if (tauriConfig.app?.security?.csp !== expectedCsp) {
  failures.push(
    "src-tauri/tauri.conf.json: CSP changed without updating the reviewed Phase 1 policy",
  );
}
if (tauriConfig.app?.security?.freezePrototype !== true) {
  failures.push(
    "src-tauri/tauri.conf.json: freezePrototype must remain enabled",
  );
}

if (failures.length) {
  console.error(failures.join("\n"));
  process.exit(1);
}
console.log(
  `Policy checks passed for ${files.length} source/configuration files.`,
);
