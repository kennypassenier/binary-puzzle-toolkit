#!/usr/bin/env bash
# Sign a release's checksum manifest with Kenny's ecosystem minisign key
# and upload the signature, the same scheme the chassis-based projects
# use. The key never leaves the machine that holds it, so this runs by
# hand after the Release workflow has published SHA256SUMS:
#
#   scripts/sign-release.sh v1.1.0
#
# bpt has no self-updater, so unlike those projects no VERSION asset is
# uploaded: the signature is for a person verifying a download.
set -euo pipefail

tag="${1:?usage: sign-release.sh vX.Y.Z}"
repo="${REPO:-kennypassenier/binary-puzzle-toolkit}"
key="${MINISIGN_KEY:-$HOME/.minisign/minisign.key}"
# The ecosystem public key, as baked into chassis (its rule R3).
pubkey="RWQWCzzUBquIHGkS3YERMkuqEm4C3vBArnlb9rySbr8z5ytgVYuji3bS"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

for tool in gh minisign; do
  command -v "$tool" >/dev/null || { echo "$tool is not installed. What now: install it, then rerun." >&2; exit 1; }
done
[ -f "$key" ] || { echo "no minisign secret key at $key. What now: run this on the machine that holds the key, or set MINISIGN_KEY to the key file." >&2; exit 1; }

echo "downloading SHA256SUMS of $tag from $repo"
gh release download "$tag" --repo "$repo" -p SHA256SUMS -D "$work"
[ -s "$work/SHA256SUMS" ] || { echo "the release has no SHA256SUMS yet. What now: wait for the Release workflow to finish." >&2; exit 1; }

echo "signing (minisign will ask for the key password)"
minisign -S -s "$key" -m "$work/SHA256SUMS" -x "$work/SHA256SUMS.minisig" -t "$repo $tag"

echo "verifying the signature against the ecosystem public key"
minisign -V -P "$pubkey" -m "$work/SHA256SUMS" -x "$work/SHA256SUMS.minisig" >/dev/null

echo "uploading SHA256SUMS.minisig"
gh release upload "$tag" --repo "$repo" --clobber "$work/SHA256SUMS.minisig"
echo "done: $repo $tag is signed. Verify a download with:"
echo "  minisign -V -P $pubkey -m SHA256SUMS -x SHA256SUMS.minisig && sha256sum -c SHA256SUMS --ignore-missing"
