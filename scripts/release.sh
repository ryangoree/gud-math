#!/bin/bash
set -e
cp README.md packages/mantissa/README.md
bun changeset publish
