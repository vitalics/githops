#!/usr/bin/env node
// Updates npm/package.json version to the value passed as the first argument.
// Usage: node scripts/stamp-npm-version.js <version>
const fs = require('fs');
const path = require('path');

const version = process.argv[2];
if (!version) {
  console.error('Usage: stamp-npm-version.js <version>');
  process.exit(1);
}

const pkgPath = path.join(__dirname, '..', 'npm', 'package.json');
const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf8'));
pkg.version = version;
fs.writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + '\n');
console.log(`npm/package.json version set to ${version}`);
