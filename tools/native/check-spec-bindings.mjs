import { readdirSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
// A spec carrying one of these tags binds selectors that are compiled on that
// platform only. It is deferred elsewhere, and the hosted job on its own platform
// still resolves every selector. Unscoped bindings must resolve everywhere,
// including specs that use plain `linux` or `windows` as a topic tag.
const PLATFORM_TAGS = { 'only-macos': 'darwin', 'only-linux': 'linux', 'only-windows': 'win32' };
const TEST_PLATFORMS = { macos: 'darwin', linux: 'linux', windows: 'win32' };

// Mixed-platform specs keep their portable scenarios checked everywhere. A
// Test-Platforms declaration applies only to the immediately following binding.
function testPlatforms(lines, file) {
  const scopes = new Map();
  for (const [index, line] of lines.entries()) {
    if (!/^\s*Test-Platforms\b/.test(line)) continue;
    const match = line.match(/^\s*Test-Platforms:\s*\[([^\]]*)\]\s*$/);
    const platforms = match?.[1].split(',').map(value => value.trim()) ?? [];
    if (!platforms.length || platforms.some(value => !Object.hasOwn(TEST_PLATFORMS, value)) || new Set(platforms).size !== platforms.length
      || !/^\s*(?:Test|Filter):\s*\S/.test(lines[index + 1] ?? '')) {
      throw new Error(`Invalid Test-Platforms binding in ${file}:${index + 1}`);
    }
    scopes.set(index + 1, platforms);
  }
  return scopes;
}

export function checkSpecBindings(inventory, { runtime = 'rust', directory = path.join(root, 'specs'), platform = process.platform } = {}) {
  if (!['node', 'rust'].includes(runtime)) throw new Error('Unknown spec runtime');
  if (!Object.values(PLATFORM_TAGS).includes(platform)) throw new Error('Unknown spec platform');
  const names = inventory.map((entry) => entry.name);
  const missing = [];
  let count = 0;
  const deferred = [];
  const platformDeferred = [];
  const superseded = [];
  const retired = [];
  for (const file of readdirSync(directory).filter((name) => name.endsWith('.spec.md'))) {
    const content = readFileSync(path.join(directory, file), 'utf8');
    const frontmatter = content.split('\n---')[0];
    const target = frontmatter.match(/^superseded-by:\s*(\S+)\s*$/m)?.[1];
    if (target) {
      if (!/^[a-zA-Z0-9_-]+\.spec\.md$/.test(target) || target === file) throw new Error(`Invalid superseded-by target in ${file}`);
      let replacement;
      try { replacement = readFileSync(path.join(directory, target), 'utf8'); } catch { throw new Error(`Missing superseded-by target in ${file}`); }
      if (/^superseded-by:/m.test(replacement.split('\n---')[0])) throw new Error(`Superseded-by chains are forbidden in ${file}`);
      if (!/^\s*(?:Test|Filter):\s*\S/m.test(replacement)) throw new Error(`Empty superseded-by replacement in ${file}`);
      superseded.push({ file, target });
      continue;
    }

    const native = /^tags:\s*\[[^\]\n]*\brust\b[^\]\n]*\]/m.test(frontmatter);
    if ((native ? 'rust' : 'node') !== runtime) { deferred.push(file); continue; }
    const tags = frontmatter.match(/^tags:\s*\[([^\]\n]*)\]/m)?.[1].split(',').map((tag) => tag.trim()) ?? [];
    const scoped = tags.filter((tag) => Object.hasOwn(PLATFORM_TAGS, tag));
    if (scoped.length && !scoped.some((tag) => PLATFORM_TAGS[tag] === platform)) { deferred.push(file); continue; }
    const lines = content.split('\n');
    const platforms = testPlatforms(lines, file);
    for (const [index, line] of lines.entries()) {
      const retiredSelector = line.match(/^\s*Retired-Test:\s*(\S.*?)\s*$/)?.[1];
      if (retiredSelector) retired.push({ file, line: index + 1, selector: retiredSelector });

      const selector = line.match(/^\s*(?:Test|Filter):\s*(\S.*?)\s*$/)?.[1];
      if (!selector) continue;
      const scope = platforms.get(index);
      if (scope && !scope.some(value => TEST_PLATFORMS[value] === platform)) {
        platformDeferred.push({ file, line: index + 1, selector, platforms: scope });
        continue;
      }
      count += 1;
      if (!names.some((name) => name.includes(selector))) missing.push({ file, line: index + 1, selector });
    }
  }
  return { count, missing, runtime, deferred, platformDeferred, superseded, retired };
}
