```bash
#!/usr/bin/env bash

set -Eeuo pipefail

[ -f artisan ] || { echo "artisan not found"; exit 1; }
command -v php >/dev/null 2>&1 || { echo "php not found"; exit 1; }
command -v composer >/dev/null 2>&1 || { echo "composer not found"; exit 1; }

# Install PHP dependencies
composer install --no-dev --prefer-dist --no-interaction --optimize-autoloader

# Frontend build
if [ -f "./.nvmrc" ] && [ -f "$HOME/.config/nvm/nvm.sh" ]; then
  # shellcheck disable=SC1090
  source "$HOME/.config/nvm/nvm.sh"
  nvm install
fi

if [ -f "./yarn.lock" ]; then
  command -v corepack >/dev/null 2>&1 && corepack enable || true
  yarn install --frozen-lockfile
  yarn build
elif [ -f "./package-lock.json" ]; then
  npm ci
  npm run build
fi

# Enter maintenance mode only for critical section
php artisan down
trap 'php artisan up || true' EXIT

php artisan migrate --force

if php artisan list | grep -q 'wayfinder:generate'; then
  php artisan wayfinder:generate
fi

php artisan optimize:clear
php artisan config:cache
php artisan route:cache
php artisan view:cache
php artisan event:cache || true
php artisan queue:restart || true

php artisan up
trap - EXIT
```