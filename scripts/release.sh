#!/usr/bin/env bash
# Build and publish a release from this machine (Kenny, 2026-09-29: every
# build runs locally, GitHub only receives the result). It does what the
# Release workflow did until then, step for step:
#
#   git tag vX.Y.Z && scripts/release.sh vX.Y.Z      # then sign-release.sh
#   DRY_RUN=1 scripts/release.sh vX.Y.Z              # build + verify, upload nothing
#
# 1. scripts/check.sh (what CI ran) and the performance thresholds;
# 2. bpt + bpt-tui for x86_64-unknown-linux-gnu, built in rust:1-bookworm so
#    the binary runs on an older glibc than this machine's, and for
#    x86_64-pc-windows-msvc, cross-built with cargo-xwin in its docker image
#    (the same MSVC ABI the windows-latest runner produced);
# 3. bpt-<tag>-<target>.tar.gz / .zip with README, CHANGELOG and LICENSE in
#    a folder of the same name, a .sha256 beside each, one SHA256SUMS;
# 4. push the tag, create the DRAFT release with CHANGELOG.md as its body.
#
# Needs docker, gh and zip. WSL and Garuda alike: nothing else is installed.
set -euo pipefail
tag="${1:?usage: scripts/release.sh vX.Y.Z}"
repo="kennypassenier/binary-puzzle-toolkit"
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

[ "${DRY_RUN:-0}" = 1 ] || [ "$(git rev-parse --abbrev-ref HEAD)" = main ] || { echo "release: not on main" >&2; exit 1; }
[ -z "$(git status --porcelain)" ] || { echo "release: working tree not clean" >&2; exit 1; }
# A dry run may rehearse a tag that does not exist yet; a real one may not.
if [ "${DRY_RUN:-0}" != 1 ]; then
  git rev-parse -q --verify "refs/tags/$tag" >/dev/null || { echo "release: tag $tag does not exist. What now: git tag $tag" >&2; exit 1; }
  [ "$(git rev-parse HEAD)" = "$(git rev-parse "$tag^{commit}")" ] || { echo "release: HEAD is not $tag" >&2; exit 1; }
fi
for tool in docker gh zip; do command -v "$tool" >/dev/null || { echo "release: $tool is not installed" >&2; exit 1; }; done

echo "== scripts/check.sh (gates + the suite on Windows)"
scripts/check.sh
echo "== performance thresholds"
cargo test --release -p bpt-core --test thresholds -- --nocapture
cargo test --release -p bpt-forge --test thresholds -- --nocapture

# Registry and git caches are this machine's, mounted, so a build does not
# download the world; each target has its own directory next to the tree.
uid="$(id -u):$(id -g)"
caches=(-v "$HOME/.cargo/registry:/usr/local/cargo/registry" -v "$HOME/.cargo/git:/usr/local/cargo/git")

echo "== build x86_64-unknown-linux-gnu (rust:1-bookworm)"
docker run --rm --user "$uid" -e HOME=/tmp -e CARGO_HOME=/usr/local/cargo "${caches[@]}" \
  -v "$root:/src" -w /src rust:1-bookworm \
  cargo build --release --locked --target x86_64-unknown-linux-gnu --target-dir target-release-linux

echo "== build x86_64-pc-windows-msvc (cargo-xwin)"
docker run --rm --user "$uid" -e HOME=/tmp -e XWIN_CACHE_DIR=/src/target-release-windows/xwin \
  -v "$root:/src" -w /src messense/cargo-xwin \
  sh -c 'rustup target add x86_64-pc-windows-msvc >/dev/null && cargo xwin build --release --locked --target x86_64-pc-windows-msvc --target-dir target-release-windows'

rm -rf dist && mkdir -p dist
package() { # target suffix archive
  local target=$1 suffix=$2 archive=$3 dir="target-release-linux"
  [ "$target" = x86_64-pc-windows-msvc ] && dir="target-release-windows"
  local name="bpt-$tag-$target"
  mkdir "dist/$name"
  cp "$dir/$target/release/bpt$suffix" "$dir/$target/release/bpt-tui$suffix" "dist/$name/"
  cp README.md CHANGELOG.md LICENSE "dist/$name/"
  (cd dist && if [ "$archive" = zip ]; then zip -qr "$name.zip" "$name"; else tar czf "$name.tar.gz" "$name"; fi
   sha256sum "$name.$archive" > "$name.$archive.sha256"; rm -rf "$name")
}
package x86_64-unknown-linux-gnu "" tar.gz
package x86_64-pc-windows-msvc .exe zip
(cd dist && cat ./*.sha256 > SHA256SUMS && sha256sum -c --quiet SHA256SUMS)
echo "--- manifest ---"; cat dist/SHA256SUMS

if [ "${DRY_RUN:-0}" = 1 ]; then
  echo "DRY_RUN: every asset built and verified in dist/; nothing pushed or published"
  exit 0
fi

git push origin "$tag"
gh release create "$tag" --repo "$repo" --verify-tag --draft --title "$tag" \
  --notes-file CHANGELOG.md dist/bpt-*.tar.gz dist/bpt-*.zip dist/SHA256SUMS
echo "draft release $tag created. Next: scripts/sign-release.sh $tag, then publish the draft."
