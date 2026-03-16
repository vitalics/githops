#!/usr/bin/env node
// Thin wrapper: resolves the downloaded native binary and spawns it.

const { spawnSync } = require("child_process");
const fs = require("fs");
const path = require("path");

const binaryName = process.platform === "win32" ? "githops.exe" : "githops";
const binaryPath = path.join(__dirname, "..", "binary", binaryName);

if (!fs.existsSync(binaryPath)) {
  console.error(
    "githops binary not found.\n" +
    "Try reinstalling: npm install githops\n" +
    "Or download manually: https://github.com/vitalics/githops/releases"
  );
  process.exit(1);
}

const result = spawnSync(binaryPath, process.argv.slice(2), { stdio: "inherit" });

if (result.error) {
  console.error(`Failed to run githops: ${result.error.message}`);
  process.exit(1);
}

process.exit(result.status ?? 0);
