#!/bin/bash
set -e
bun run build
bunx changeset publish
