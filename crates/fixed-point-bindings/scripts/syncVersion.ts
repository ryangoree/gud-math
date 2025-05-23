import manifest from '../package.json' with { type: 'json' };
import { dirname, resolve } from 'node:path';
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Load the Cargo.toml file.
const cargoManifestPath = resolve(__dirname, '../Cargo.toml');
const cargoTomlSrc = readFileSync(cargoManifestPath, 'utf8');

// 1. Sync the cargo.toml version with the package.json
writeFileSync(
  cargoManifestPath,
  cargoTomlSrc.replace(
    // https://regex101.com/r/PLmbXb/1
    /^version(\s*)=(\s*)"[^"]+"/m,
    `version$1=$2"${manifest.version}"`,
  ),
);