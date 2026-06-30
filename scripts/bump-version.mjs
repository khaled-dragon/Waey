import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

const versionInput = process.argv[2]?.trim();

if (!versionInput) {
  fail("Usage: npm run version:bump -- 1.3.7");
}

const version = versionInput.replace(/^v/i, "");

if (!/^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/.test(version)) {
  fail(`Invalid version "${versionInput}". Use a semver value like 1.3.7.`);
}

const root = process.cwd();

updateJson("package.json", (json) => {
  json.version = version;
  return json;
});

updateJson("package-lock.json", (json) => {
  json.version = version;

  if (json.packages?.[""]) {
    json.packages[""].version = version;
  }

  return json;
});

updateCargoVersion("src-tauri/Cargo.toml");

updateJson("src-tauri/tauri.conf.json", (json) => {
  json.version = version;
  return json;
});

console.log(`Waey version bumped to ${version}`);

function updateJson(filePath, transform) {
  const absolutePath = resolve(root, filePath);
  const json = JSON.parse(readFileSync(absolutePath, "utf8"));
  const nextJson = transform(json);

  writeFileSync(absolutePath, `${JSON.stringify(nextJson, null, 2)}\n`);
}

function updateCargoVersion(filePath) {
  const absolutePath = resolve(root, filePath);
  const content = readFileSync(absolutePath, "utf8");
  const newline = content.includes("\r\n") ? "\r\n" : "\n";
  const lines = content.split(/\r?\n/);
  const packageSectionStart = lines.findIndex((line) => line.trim() === "[package]");

  if (packageSectionStart < 0) {
    fail(`Could not find [package] section in ${filePath}.`);
  }

  const packageSectionEnd = lines.findIndex(
    (line, index) => index > packageSectionStart && /^\s*\[.+\]\s*$/.test(line),
  );
  const searchEnd = packageSectionEnd === -1 ? lines.length : packageSectionEnd;
  const versionLineIndex = lines.findIndex(
    (line, index) => index > packageSectionStart && index < searchEnd && /^\s*version\s*=/.test(line),
  );

  if (versionLineIndex < 0) {
    fail(`Could not find package version in ${filePath}.`);
  }

  lines[versionLineIndex] = `version = "${version}"`;

  writeFileSync(absolutePath, lines.join(newline));
}

function fail(message) {
  console.error(message);
  process.exit(1);
}
