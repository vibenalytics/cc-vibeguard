#!/usr/bin/env node

const { execFileSync } = require("child_process");
const path = require("path");

const PLATFORMS = {
  "darwin-arm64": "@vibenalytics/security-darwin-arm64",
  "darwin-x64": "@vibenalytics/security-darwin-x64",
  "linux-x64": "@vibenalytics/security-linux-x64",
  "linux-arm64": "@vibenalytics/security-linux-arm64",
};

const platformKey = `${process.platform}-${process.arch}`;
const pkg = PLATFORMS[platformKey];

if (!pkg) {
  console.error(
    `Unsupported platform: ${platformKey}\n` +
    `Supported: ${Object.keys(PLATFORMS).join(", ")}`
  );
  process.exit(1);
}

let binPath;
try {
  binPath = require.resolve(`${pkg}/vibenalytics-security`);
} catch {
  console.error(
    `Could not find binary package ${pkg}.\n` +
    `This usually means the optional dependency was not installed.\n` +
    `Try: npm install --force @vibenalytics/security`
  );
  process.exit(1);
}

try {
  execFileSync(binPath, process.argv.slice(2), { stdio: "inherit" });
} catch (e) {
  if (e.status !== undefined) {
    process.exit(e.status);
  }
  throw e;
}
