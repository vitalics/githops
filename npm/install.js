#!/usr/bin/env node
// Postinstall: download the pre-built githops binary for the current platform.

const { execFileSync } = require("child_process");
const fs = require("fs");
const https = require("https");
const path = require("path");
const { version } = require("./package.json");

const REPO = "vitalics/githops";
const tag = `v${version}`;
const binDir = path.join(__dirname, "binary");

// ── Platform → artifact name ────────────────────────────────────────────────

function getTarget() {
  const { platform, arch } = process;
  if (platform === "darwin") {
    return arch === "arm64"
      ? `aarch64-apple-darwin`
      : `x86_64-apple-darwin`;
  }
  if (platform === "linux") {
    return arch === "arm64"
      ? `aarch64-unknown-linux-gnu`
      : `x86_64-unknown-linux-gnu`;
  }
  if (platform === "win32") {
    return `x86_64-pc-windows-msvc`;
  }
  throw new Error(`Unsupported platform: ${platform}/${arch}`);
}

function getFilename(target) {
  const ext = target.includes("windows") ? "zip" : "tar.gz";
  return `githops-${tag}-${target}.${ext}`;
}

function getBinaryName() {
  return process.platform === "win32" ? "githops.exe" : "githops";
}

// ── Download helpers ─────────────────────────────────────────────────────────

function download(url, dest) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);
    const follow = (u) => {
      https.get(u, { headers: { "User-Agent": "githops-npm-installer" } }, (res) => {
        if (res.statusCode === 301 || res.statusCode === 302) {
          follow(res.headers.location);
          return;
        }
        if (res.statusCode !== 200) {
          reject(new Error(`HTTP ${res.statusCode} for ${u}`));
          return;
        }
        res.pipe(file);
        file.on("finish", () => file.close(resolve));
      }).on("error", reject);
    };
    follow(url);
  });
}

function extract(archive, target, dest) {
  fs.mkdirSync(dest, { recursive: true });
  if (archive.endsWith(".zip")) {
    // Use Node's built-in or system unzip
    try {
      execFileSync("powershell", [
        "-Command",
        `Expand-Archive -Path "${archive}" -DestinationPath "${dest}" -Force`,
      ]);
    } catch {
      execFileSync("unzip", ["-o", archive, "-d", dest]);
    }
  } else {
    execFileSync("tar", ["xzf", archive, "-C", dest]);
  }
}

// ── Main ─────────────────────────────────────────────────────────────────────

async function main() {
  const target = getTarget();
  const filename = getFilename(target);
  const url = `https://github.com/${REPO}/releases/download/${tag}/${filename}`;
  const archivePath = path.join(binDir, filename);
  const binaryName = getBinaryName();
  const binaryPath = path.join(binDir, binaryName);

  // Skip if binary already downloaded for this version
  if (fs.existsSync(binaryPath)) {
    console.log(`githops ${tag} already installed.`);
    return;
  }

  fs.mkdirSync(binDir, { recursive: true });

  process.stdout.write(`Downloading githops ${tag} for ${target}...`);
  await download(url, archivePath);
  process.stdout.write(" done.\n");

  extract(archivePath, target, binDir);
  fs.unlinkSync(archivePath);

  // Make executable on Unix
  if (process.platform !== "win32") {
    fs.chmodSync(binaryPath, 0o755);
  }

  console.log(`githops ${tag} installed to ${binaryPath}`);
}

main().catch((err) => {
  // Non-fatal: the wrapper will report a clear error on first use.
  console.warn(`githops: binary download failed — ${err.message}`);
  console.warn("You can install githops manually: https://github.com/vitalics/githops/releases");
  process.exit(0);
});
