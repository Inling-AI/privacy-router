#!/usr/bin/env bash
# Build and publish compiled demo assets; requires push access to the demo repo.
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
flutter_bin="${FLUTTER_BIN:-$repo_root/.fvm/flutter_sdk/bin/flutter}"
site_dir="$(mktemp -d)"
trap 'rm -rf "$site_dir"' EXIT
cd "$repo_root/apps/console"
"$flutter_bin" pub get --enforce-lockfile
"$flutter_bin" build web --wasm --release --no-pub \
  --dart-define=PRIVACY_ROUTER_DEMO=true \
  --base-href /privacy-router-demo/ --output build/demo
bash "$repo_root/scripts/prune_web_release.sh" build/demo
touch build/demo/.nojekyll
git clone --depth 1 https://github.com/Inling-AI/privacy-router-demo.git "$site_dir"
# Preserve the separately published introduction video when refreshing the app.
rsync -a --delete --exclude=.git --exclude=video/ build/demo/ "$site_dir/"
git -C "$site_dir" add --all
if ! git -C "$site_dir" diff --cached --quiet; then
  git -C "$site_dir" commit -m 'Update static demo'
  git -C "$site_dir" push origin HEAD:main
fi
