#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
CONFIG_FILE="${1:-${PROJECT_ROOT}/project.toml}"

if [ ! -f "$CONFIG_FILE" ]; then
  echo "Config not found: $CONFIG_FILE"
  echo "Create it from project.toml.template"
  exit 1
fi

# Read a value from a TOML file: toml_get <file> <section> <key>
toml_get() {
  python3 - "$1" "$2" "$3" <<'PYEOF'
import sys

file, section, key = sys.argv[1], sys.argv[2], sys.argv[3]

try:
    import tomllib
    with open(file, "rb") as f:
        data = tomllib.load(f)
    val = data.get(section, {}).get(key)
    if val is not None:
        print(val)
    sys.exit(0)
except ImportError:
    pass

# Fallback for Python < 3.11
in_section = False
with open(file) as f:
    for line in f:
        line = line.strip()
        if not line or line.startswith('#'):
            continue
        if line.startswith('[') and line.endswith(']'):
            in_section = (line[1:-1].strip() == section)
            continue
        if in_section and '=' in line:
            k, _, v = line.partition('=')
            if k.strip() == key:
                v = v.strip().strip('"').strip("'").split('#')[0].strip()
                print(v)
                break
PYEOF
}

PROJECT_NAME="$(toml_get "$CONFIG_FILE" project name)"
PROJECT_TITLE="$(toml_get "$CONFIG_FILE" project title)"
DB_KIND="$(toml_get "$CONFIG_FILE" database kind)"

for v in PROJECT_NAME PROJECT_TITLE; do
  if [ -z "${!v:-}" ]; then
    echo "Missing required config key: $v"
    exit 1
  fi
done

DB_KIND="${DB_KIND:-sqlite}"
if [ "$DB_KIND" != "sqlite" ] && [ "$DB_KIND" != "postgres" ]; then
  echo "Invalid database.kind: $DB_KIND (expected: sqlite or postgres)"
  exit 1
fi

if [ "$DB_KIND" = "sqlite" ]; then
  BACKEND_DB_FEATURE="db-sqlite"
else
  BACKEND_DB_FEATURE="db-postgres"
fi

BACKEND_BIN="${PROJECT_NAME}-backend"
GUI_PACKAGE="${PROJECT_NAME}-gui"
ADMIN_GUI_PACKAGE="${PROJECT_NAME}-admin-gui"

perl -0777 -i -pe "s/name = \"[^\"]+\"/name = \"${BACKEND_BIN}\"/" "$PROJECT_ROOT/backend/Cargo.toml"
perl -0777 -i -pe "s/default = \\[[^\\]]+\\]/default = [\"${BACKEND_DB_FEATURE}\"]/s" "$PROJECT_ROOT/backend/Cargo.toml"
perl -0777 -i -pe "s/\"name\": \"[^\"]+\"/\"name\": \"${GUI_PACKAGE}\"/" "$PROJECT_ROOT/gui/package.json"
perl -0777 -i -pe "s/\"name\": \"[^\"]+\"/\"name\": \"${ADMIN_GUI_PACKAGE}\"/" "$PROJECT_ROOT/admin-gui/package.json"
perl -0777 -i -pe "s#<title>.*?</title>#<title>${PROJECT_TITLE}</title>#s" "$PROJECT_ROOT/gui/index.html"
perl -0777 -i -pe "s#<title>.*?</title>#<title>${PROJECT_TITLE} Admin</title>#s" "$PROJECT_ROOT/admin-gui/index.html"
perl -0777 -i -pe "s/^# .* Template/# ${PROJECT_TITLE} Template/m" "$PROJECT_ROOT/README.md"
perl -0777 -i -pe "s/cargo run -p [A-Za-z0-9_-]+-backend/cargo run -p ${BACKEND_BIN}/g" "$PROJECT_ROOT/README.md"

echo "Initialized project naming from project.toml"
echo "backend crate: ${BACKEND_BIN}"
echo "backend db feature: ${BACKEND_DB_FEATURE}"
echo "gui package: ${GUI_PACKAGE}"
echo "admin-gui package: ${ADMIN_GUI_PACKAGE}"
