#!/bin/bash
set -e
bun run build
cp README.md packages/fixed/README.md
# bun changeset publish
