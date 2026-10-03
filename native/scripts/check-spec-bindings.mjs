import { readdirSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
// A spec carrying one of these tags binds selectors that are compiled on that
// platform only. It is deferred elsewhere, and the hosted job on its own platform
// still resolves every selector. Every other spec must resolve everywhere,
// including the many that use plain `linux` or `windows` as a topic tag.
const PLATFORM_TAGS = { 'only-macos': 'darwin', 'only-linux': 'linux', 'only-windows': 'win32' };

export function checkSpecBindings(inventory, { runtime = 'rust', directory = path.join(root, 'specs'), platform = process.platform } = {}) {
  if (!['node', 'rust'].includes(runtime)) throw new Error('Unknown spec runtime');
  if (!Object.values(PLATFORM_TAGS).includes(platform)) throw new Error('Unknown spec platform');
  const names = inventory.map((entry) => entry.name);
  const missing = [];
  let count = 0;
  const deferred = [];
  for (const file of readdirSync(directory).filter((name) => name.endsWith('.spec.md'))) {
    const content = readFileSync(path.join(directory, file), 'utf8');
    const frontmatter = content.split('\n---')[0];
    const native = /^tags:\s*\[[^\]\n]*\brust\b[^\]\n]*\]/m.test(frontmatter);
    if ((native ? 'rust' : 'node') !== runtime) { deferred.push(file); continue; }
    const tags = frontmatter.match(/^tags:\s*\[([^\]\n]*)\]/m)?.[1].split(',').map((tag) => tag.trim()) ?? [];
    const scoped = tags.filter((tag) => Object.hasOwn(PLATFORM_TAGS, tag));
    if (scoped.length && !scoped.some((tag) => PLATFORM_TAGS[tag] === platform)) { deferred.push(file); continue; }
    const lines = content.split('\n');
    for (const [index, line] of lines.entries()) {
      const selector = line.match(/^\s*(?:Test|Filter):\s*(\S.*?)\s*$/)?.[1];
      if (!selector) continue;
      count += 1;
      if (!names.some((name) => name.includes(selector))) missing.push({ file, line: index + 1, selector });
    }
  }
  return { count, missing, runtime, deferred };
}
