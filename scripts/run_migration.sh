#!/bin/bash
set -euo pipefail

# Use DATABASE_URL from the environment (e.g. sourced from .env); fall back to
# the local default so the script still works standalone.
DATABASE_URL="${DATABASE_URL:-postgres://username:password@localhost:5432/icon_api}"

if ! command -v diesel >/dev/null 2>&1; then
    echo "error: the 'diesel' CLI is not installed." >&2
    echo "  install it with: cargo install diesel_cli --no-default-features --features postgres" >&2
    echo "  (the api-server also runs these migrations automatically on startup," >&2
    echo "   so running them here is optional)." >&2
    exit 1
fi

echo "Running diesel migrations..."
echo "Migration dir: ./db/migrations"
echo "Database URL: ${DATABASE_URL}"

diesel migration run \
    --migration-dir ./db/migrations \
    --database-url "$DATABASE_URL"

echo "Migrations completed successfully."
