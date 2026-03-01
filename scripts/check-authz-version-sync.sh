#!/usr/bin/env bash
set -euo pipefail

changed_files="$(git diff --cached --name-only)"

model_changed=0
migration_changed=0

while IFS= read -r f; do
  [ -z "$f" ] && continue
  case "$f" in
    backend/authz/model.conf|backend/authz/VERSION)
      model_changed=1
      ;;
    backend/migrations/sqlite/*auth*sql|backend/migrations/postgres/*auth*sql|backend/migrations/sqlite/*casbin*sql|backend/migrations/postgres/*casbin*sql)
      migration_changed=1
      ;;
  esac
done <<< "$changed_files"

if [ "$model_changed" -eq 1 ] && [ "$migration_changed" -eq 0 ]; then
  echo "[pre-commit] authz sync check failed"
  echo "Changed backend/authz model/version but no auth migration SQL was staged."
  echo "Stage a corresponding file under backend/migrations/sqlite or backend/migrations/postgres."
  exit 1
fi

if [ "$migration_changed" -eq 1 ] && [ "$model_changed" -eq 0 ]; then
  echo "[pre-commit] authz sync check failed"
  echo "Changed auth migration SQL but backend/authz/model.conf or backend/authz/VERSION was not staged."
  echo "Stage a model/version update to keep authorization schema and model aligned."
  exit 1
fi

echo "[pre-commit] authz sync check passed"
