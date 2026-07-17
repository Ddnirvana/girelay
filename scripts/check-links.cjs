const fs = require('fs');
const path = require('path');

const root = path.resolve(__dirname, '..');
const roots = ['README.md', 'CHANGELOG.md', 'CODE_OF_CONDUCT.md', 'CONTRIBUTING.md', 'SECURITY.md', 'docs'];
const failures = [];

function filesAt(relative) {
  const absolute = path.join(root, relative);
  const stat = fs.statSync(absolute);
  if (stat.isFile()) return [absolute];
  return fs.readdirSync(absolute, { withFileTypes: true }).flatMap((entry) => {
    const child = path.join(absolute, entry.name);
    return entry.isDirectory() ? filesAt(path.relative(root, child)) : [child];
  });
}

for (const file of roots.flatMap(filesAt).filter((file) => file.endsWith('.md') || file.endsWith('.html'))) {
  const body = fs.readFileSync(file, 'utf8');
  const targets = [
    ...body.matchAll(/\[[^\]]*\]\(([^)]+)\)/g),
    ...body.matchAll(/href=["']([^"']+)["']/g),
  ].map((match) => match[1].trim().replace(/^<|>$/g, ''));
  for (const target of targets) {
    if (!target || /^(https?:|mailto:|#)/.test(target) || target.includes('[PUBLIC_')) continue;
    const clean = decodeURIComponent(target.split('#')[0]);
    if (!clean) continue;
    const resolved = path.resolve(path.dirname(file), clean);
    if (!fs.existsSync(resolved)) failures.push(`${path.relative(root, file)} -> ${target}`);
  }
}

if (failures.length) {
  console.error(`Broken local links:\n${failures.map((failure) => `- ${failure}`).join('\n')}`);
  process.exit(1);
}
console.log('Documentation links resolve locally.');
