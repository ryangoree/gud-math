#!/usr/bin/env node
import assert from 'node:assert';
import { execSync } from 'node:child_process';
import {
  cpSync,
  existsSync,
  mkdirSync,
  readFileSync,
  renameSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { basename, dirname, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import lockfile from 'proper-lockfile';
import TOML from 'smol-toml';
import manifest from '../package.json' with { type: 'json' };
import './syncVersion.js';

// Settings
const tempDir = 'tmp';
const packageName = '@gud/math';
const outDir = '../../../packages/';
const readmePath = '../../../README.md';
const licensePath = '../../../LICENSE';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
try {
  const cwd = process.cwd();
  const script = relative(cwd, __filename);
  const packageBaseName = basename(packageName);
  const outPath = resolve(__dirname, outDir, packageBaseName);

  // Lock the script to prevent concurrent builds
  const release = await lockfile.lock(__filename, {
    retries: 3,
    update: 3_000, // 3 seconds
    stale: 60_000 * 10, // 10 minutes
  });

  // Load the Cargo.toml file.
  const cargoManifestPath = resolve(__dirname, '../Cargo.toml');
  const cargoTomlSrc = readFileSync(cargoManifestPath, 'utf8');
  const cargoToml = TOML.parse(cargoTomlSrc) as {
    package: { name: string; version: string };
  };

  console.log(`+
	|  ${manifest.name} - ${script}
	|
	|  Generating ${packageName} package from the ${cargoToml.package.name} crate...
	|
	|  Name:        ${packageName}
	|  Version:     ${manifest.version}
	|  Out path:    ${relative(cwd, outPath)}
	+`);

  // 1. Build the wasm package.
  console.log('Building package...');
  run(`npx wasm-pkg-build --modules cjs,esm-sync --out-dir ${tempDir}`);
  assert(
    existsSync(tempDir),
    `Error: Build failed. The output directory ${tempDir} does not exist.`
  );

  // 2. Modify the package.json file.
  console.log('Modifying package.json...');
  const modifiedManifest = {
    ...manifest,
    name: packageName,
    // Add a main field for improved commonjs compatibility.
    main: `${packageBaseName}.cjs`,
    types: `${packageBaseName}.d.ts`,
    // Add exports for both ESM and CJS compatibility for modern node versions.
    exports: {
      '.': {
        default: {
          require: `./${packageBaseName}.cjs`,
          import: `./${packageBaseName}.js`,
          types: `./${packageBaseName}.d.ts`,
        },
      },
    },
    // Explicitly set the publishConfig access to public to ensure it's published by changesets.
    publishConfig: {
      access: 'public',
    },
    // Remove unnecessary fields.
    private: undefined,
    scripts: undefined,
    devDependencies: undefined,
  };

  // 3. Copy generated files to the output directory.
  const buildPrefix = cargoToml.package.name.replaceAll('-', '_');
  const manifestExports = modifiedManifest.exports['.'].default;
  mkdirSync(outPath, { recursive: true });
  writeFileSync(
    resolve(outPath, 'package.json'),
    JSON.stringify(modifiedManifest, null, 2)
  );
  renameSync(
    resolve(tempDir, `${buildPrefix}_worker.js`),
    resolve(outPath, manifestExports.import)
  );
  renameSync(
    resolve(tempDir, `${buildPrefix}.js`),
    resolve(outPath, manifestExports.require)
  );
  renameSync(
    resolve(tempDir, `${buildPrefix}.d.ts`),
    resolve(outPath, manifestExports.types)
  );
  renameSync(
    resolve(tempDir, `${buildPrefix}_bg.wasm`),
    resolve(outPath, `${packageBaseName}_bg.wasm`)
  );
  cpSync(resolve(__dirname, licensePath), resolve(outPath, 'LICENSE'));
  cpSync(resolve(__dirname, readmePath), resolve(outPath, 'README.md'));

  // 4. Remove the temporary build files.
  console.log('Removing temporary build files...');
  rmSync(tempDir, { recursive: true });

  // 5. Release the lockfile
  release();

  // Done!
  console.log(`✅ Packages generated at: ${relative(cwd, outPath)}`);

  process.exit(0);
} catch (err) {
  console.error(err);
  lockfile.unlock(__filename);
  process.exit(1);
}

function run(cmd: string, cwd?: string) {
  console.log(`▶️ Running: ${cmd}`);
  execSync(cmd, { stdio: 'inherit', cwd });
}
