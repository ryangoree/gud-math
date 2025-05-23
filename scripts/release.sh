#!/bin/bash
set -e
bun run build
bun changeset publish
