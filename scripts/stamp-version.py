#!/usr/bin/env python3
# Updates version = "..." in all Cargo.toml files and keeps path-dep constraints in sync.
# Usage: python3 scripts/stamp-version.py <version>
import re, pathlib, sys

version = sys.argv[1] if len(sys.argv) > 1 else None
if not version:
    print("Usage: stamp-version.py <version>", file=sys.stderr)
    sys.exit(1)

for p in ["Cargo.toml", "githops-core/Cargo.toml", "graphui/Cargo.toml"]:
    text = pathlib.Path(p).read_text()
    text = re.sub(r'^version = "[^"]*"', f'version = "{version}"', text, count=1, flags=re.M)
    pathlib.Path(p).write_text(text)

# Keep path-dep version constraints in sync
for p in ["Cargo.toml", "graphui/Cargo.toml"]:
    text = pathlib.Path(p).read_text()
    text = re.sub(r'(githops-core\s*=\s*\{[^}]*version\s*=\s*")[^"]*(")', rf'\g<1>{version}\g<2>', text)
    text = re.sub(r'(graphui\s*=\s*\{[^}]*version\s*=\s*")[^"]*(")', rf'\g<1>{version}\g<2>', text)
    pathlib.Path(p).write_text(text)

print(f"Cargo.toml versions set to {version}")
