/// Builds the Tauri v2 auto-update manifest (`latest.json`) for a release.
///
/// Why this exists: tauri-action@v1 fails to collect the `.sig` artifacts
/// (tauri-apps/tauri-action#1098 — "Signature not found for the updater
/// JSON"), so it never uploads `latest.json`. Each platform build instead
/// uploads its signed updater bundle + `.sig` as a workflow artifact; this
/// script reads those signatures + maps them to the release asset URLs and
/// writes the manifest, which the workflow then uploads to the release.
///
/// Usage: bun run scripts/build-updater-manifest.ts <tag> <artifacts-dir>
///   <tag>           e.g. v0.3.3-beta.1
///   <artifacts-dir> dir containing downloaded `updater-<key>/` artifacts
///
/// Output: latest.json in the current working directory.

import { execSync } from 'node:child_process';
import { existsSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

/** Recursively find the first `*.sig` file under `dir` (upload-artifact v4 may
 * preserve directory structure, so the .sig may be nested). */
function findSig(dir: string): string | null {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const p = join(dir, entry.name);
    if (entry.isDirectory()) {
      const found = findSig(p);
      if (found) return found;
    } else if (entry.name.endsWith('.sig')) {
      return p;
    }
  }
  return null;
}

const tag = process.argv[2];
const artifactsDir = process.argv[3] ?? 'artifacts';

if (!tag) {
  console.error('Usage: bun run scripts/build-updater-manifest.ts <tag> <artifacts-dir>');
  process.exit(1);
}

const version = tag.replace(/^v/, '');

// Each Tauri updater platform key + the suffix that identifies its updater
// bundle asset in the GitHub release.
const PLATFORMS = [
  { key: 'darwin-aarch64', bundleSuffix: '_aarch64.app.tar.gz' },
  { key: 'darwin-x86_64', bundleSuffix: '_x64.app.tar.gz' },
  { key: 'windows-x86_64', bundleSuffix: '_x64-setup.exe' },
] as const;

// Release assets (name → browser download URL) via the gh CLI.
const ghAssets = JSON.parse(
  execSync(`gh release view ${tag} --json assets --jq '.assets'`, { encoding: 'utf-8' }),
) as Array<{ name: string; url: string }>;

// Release notes (fall back to a simple label).
let notes = `CorIn ${tag}`;
try {
  const body = execSync(`gh release view ${tag} --json body --jq '.body'`, { encoding: 'utf-8' }).trim();
  if (body) notes = body;
} catch {
  // keep default notes
}

const platforms: Record<string, { signature: string; url: string }> = {};

for (const p of PLATFORMS) {
  const dir = join(artifactsDir, `updater-${p.key}`);
  if (!existsSync(dir)) {
    console.warn(`⚠ no artifact dir for ${p.key} — skipping (no auto-update for this platform)`);
    continue;
  }
  // The downloaded artifact may preserve directory structure; search recursively.
  const sigPath = findSig(dir);
  if (!sigPath) {
    console.warn(`⚠ no .sig in ${dir} — skipping ${p.key}`);
    continue;
  }
  const signature = readFileSync(sigPath, 'utf-8');

  // Match the unsigned updater bundle in the release assets.
  const bundle = ghAssets.find(
    (a) => a.name.endsWith(p.bundleSuffix) && !a.name.endsWith('.sig'),
  );
  if (!bundle) {
    console.warn(`⚠ no release asset matching "${p.bundleSuffix}" — skipping ${p.key}`);
    continue;
  }

  platforms[p.key] = { signature, url: bundle.url };
  console.log(`✓ ${p.key} ← ${bundle.name}`);
}

if (Object.keys(platforms).length === 0) {
  console.error(
    '✖ No platforms assembled — no .sig artifacts matched. Aborting ' +
      '(an empty manifest would break auto-update). Check the upload-step globs.',
  );
  process.exit(1);
}

const manifest = {
  version,
  notes,
  pub_date: new Date().toISOString(),
  platforms,
};

writeFileSync('latest.json', JSON.stringify(manifest, null, 2) + '\n');
console.log(`\nWrote latest.json (version ${version}, ${Object.keys(platforms).length} platform(s))`);
