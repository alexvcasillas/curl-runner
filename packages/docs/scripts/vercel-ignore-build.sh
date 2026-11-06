#!/bin/sh
# Vercel "Ignored Build Step" for the docs site. Runs from packages/docs.
# Exit 0 skips the build, exit 1 builds. When unsure, build.

# Release PRs only bump versions; the merge to main builds the docs.
[ "$VERCEL_GIT_COMMIT_REF" = "changeset-release/main" ] && exit 0

# Compare with the last deployed commit, so a push of several commits still
# builds when any of them touched the docs. No previous deployment, or a
# commit missing from the shallow clone: build.
base="$VERCEL_GIT_PREVIOUS_SHA"
[ -n "$base" ] || exit 1
git cat-file -e "$base^{commit}" 2>/dev/null || exit 1

# The docs, plus what they read from elsewhere: the CLI changelog and version
# they render, and the dependencies they build with.
if git diff --quiet "$base" HEAD -- \
  . \
  ../cli/CHANGELOG.md \
  ../cli/package.json \
  ../../package.json \
  ../../pnpm-lock.yaml \
  ../../pnpm-workspace.yaml; then
  exit 0
fi

exit 1
