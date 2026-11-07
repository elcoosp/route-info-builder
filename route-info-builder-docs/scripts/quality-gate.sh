#!/usr/bin/env bash
# Zero-warning quality gate (astro-doc-architect Phase 8.2).
#
# Runs build, check and lint, then fails on any warning line.
#
# Narrow, documented exception (docs/decisions/0005-known-warnings.md):
# Starlight 0.42.10 unconditionally injects the @astrojs/sitemap integration,
# which prints exactly one line when no `site` URL is configured. The
# route-info-builder repository declares no deploy URL and inventing one is
# not acceptable, so only that exact string is filtered — any other warning
# fails the gate.
set -uo pipefail
cd "$(dirname "$0")/.."

pnpm build 2>&1 | tee .qa/build.log
pnpm check 2>&1 | tee .qa/check.log
pnpm lint 2>&1 | tee .qa/lint.log

SITEMAP_LINE='The Sitemap integration requires the `site` astro.config option\. Skipping\.'

if grep -Eiv "$SITEMAP_LINE" .qa/build.log | grep -Eiq '\b(warn|warning)\b'; then
	echo "FAIL: build log contains warning lines:"
	grep -Eiv "$SITEMAP_LINE" .qa/build.log | grep -Ei '\b(warn|warning)\b'
	exit 1
fi
for log in .qa/check.log .qa/lint.log; do
	if grep -Eiq '\b(warn|warning)\b' "$log"; then
		echo "FAIL: $log contains warning lines:"
		grep -Ei '\b(warn|warning)\b' "$log"
		exit 1
	fi
done
echo "ZERO-WARNING GATE PASSED (build + check + lint)"
