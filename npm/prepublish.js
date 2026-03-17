#!/usr/bin/env node
// Copies root-level files and docs/ into the npm package directory
// before `npm publish` so they are included in the tarball.
// Runs automatically via the `prepublishOnly` lifecycle hook.

"use strict";

const fs = require("fs");
const path = require("path");

const root = path.resolve(__dirname, "..");
const here = __dirname;

function copyFile(src, dest) {
  // Remove any existing file or symlink at the destination before copying.
  try { fs.unlinkSync(dest); } catch (_) {}
  fs.copyFileSync(src, dest);
  console.log(`  copied  ${path.relative(root, src)}`);
}

function copyDir(src, dest) {
  fs.mkdirSync(dest, { recursive: true });
  for (const entry of fs.readdirSync(src, { withFileTypes: true })) {
    const s = path.join(src, entry.name);
    const d = path.join(dest, entry.name);
    entry.isDirectory() ? copyDir(s, d) : copyFile(s, d);
  }
}

console.log("prepublish: copying files into npm/…");
copyFile(path.join(root, "LICENSE"),   path.join(here, "LICENSE"));
copyFile(path.join(root, "README.md"), path.join(here, "README.md"));
copyDir(path.join(root, "docs"),       path.join(here, "docs"));
console.log("prepublish: done.");
