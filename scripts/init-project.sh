#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
CONFIG_FILE="${1:-${PROJECT_ROOT}/project.conf}"

if [ ! -f "$CONFIG_FILE" ]; then
  echo "Config not found: $CONFIG_FILE"
  echo "Create it from project.conf.template"
  exit 1
fi

# shellcheck disable=SC1090
source "$CONFIG_FILE"

required_vars=(PROJECT_NAME PROJECT_TITLE)
for v in "${required_vars[@]}"; do
  if [ -z "${!v:-}" ]; then
    echo "Missing required config key: $v"
    exit 1
  fi
done

DB_KIND="${DB_KIND:-sqlite}"
if [ "$DB_KIND" != "sqlite" ] && [ "$DB_KIND" != "postgres" ]; then
  echo "Invalid DB_KIND: $DB_KIND (expected: sqlite or postgres)"
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

echo "Initialized project naming from project.conf"
echo "backend crate: ${BACKEND_BIN}"
echo "backend db feature: ${BACKEND_DB_FEATURE}"
echo "gui package: ${GUI_PACKAGE}"
echo "admin-gui package: ${ADMIN_GUI_PACKAGE}"
