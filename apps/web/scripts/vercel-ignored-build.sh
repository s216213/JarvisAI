#!/bin/bash
# Use as Vercel "Ignored Build Step":
# bash apps/web/scripts/vercel-ignored-build.sh
# Exit 0 = skip build, 1 = proceed.

if git diff --quiet HEAD^ HEAD -- . ../../packages/protocol 2>/dev/null; then
  echo "No web-relevant changes — skipping"
  exit 0
fi

echo "Web changes detected — building"
exit 1
