#!/usr/bin/env python3
"""One-time script: convert testfiles-creation/*.json to *.yaml.

The manifest field is preserved as a raw JSON block scalar.
All other fields are written as native YAML scalars.
Run from the repository root: python3 scripts/convert_to_yaml.py
"""
import json
import os

FIELD_ORDER = [
    "testId", "title", "description", "alg", "inputAsset",
    "signingCert", "signingKey", "tsaUrl",
]


def needs_quoting(s: str) -> bool:
    """Return True if a bare YAML scalar would be ambiguous."""
    specials = set(':#{}[]|>&*!,\'"?')
    return (
        s.startswith("http")
        or any(c in s for c in specials)
        or s.lower() in ("true", "false", "null", "yes", "no")
    )


def scalar(value) -> str:
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, (int, float)):
        return str(value)
    s = str(value)
    if needs_quoting(s):
        escaped = s.replace("\\", "\\\\").replace('"', '\\"')
        return f'"{escaped}"'
    return s


def convert(json_path: str) -> None:
    with open(json_path) as f:
        data = json.load(f)

    manifest = data.pop("manifest")
    manifest_json = json.dumps(manifest, indent=2)

    lines = []
    # Write fields in preferred order
    for key in FIELD_ORDER:
        if key in data:
            lines.append(f"{key}: {scalar(data[key])}")
    # Any remaining fields not in FIELD_ORDER
    for key, val in data.items():
        if key not in FIELD_ORDER:
            lines.append(f"{key}: {scalar(val)}")
    # Manifest as indented JSON block scalar
    lines.append("manifest: |")
    for line in manifest_json.split("\n"):
        lines.append("  " + line if line.strip() else "")

    yaml_path = json_path[:-5] + ".yaml"
    with open(yaml_path, "w") as f:
        f.write("\n".join(lines) + "\n")

    os.remove(json_path)
    print(f"  {os.path.basename(json_path)} → {os.path.basename(yaml_path)}")


folder = "testfiles-creation"
count = 0
for fname in sorted(os.listdir(folder)):
    if fname.endswith(".json"):
        convert(os.path.join(folder, fname))
        count += 1

print(f"\nConverted {count} files.")
