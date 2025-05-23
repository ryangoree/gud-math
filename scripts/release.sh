#!/bin/bash
set -e
cp README.md packages/fixed/README.md
bun changeset publish
