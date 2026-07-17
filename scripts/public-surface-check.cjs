const fs = require('fs');
const path = require('path');

const root = path.resolve(__dirname, '..');
const roots = [
  'README.md',
  'CHANGELOG.md',
  'CODE_OF_CONDUCT.md',
  'CONTRIBUTING.md',
  'SECURITY.md',
  'crates',
  'docs',
  'examples',
  'assets',
  'packaging',
  'scripts',
  '.github',
];

function filesAt(relative) {
  const absolute = path.join(root, relative);
  const stat = fs.statSync(absolute);
  if (stat.isFile()) return [absolute];
  return fs.readdirSync(absolute, { withFileTypes: true }).flatMap((entry) => {
    const child = path.join(absolute, entry.name);
    return entry.isDirectory() ? filesAt(path.relative(root, child)) : [child];
  });
}

const scanner = path.join(root, 'scripts/public-surface-check.cjs');
const files = roots.flatMap(filesAt).filter((file) => file !== scanner);
const findings = [];

for (const file of files) {
  const buffer = fs.readFileSync(file);
  if (buffer.includes(0)) continue;
  const relative = path.relative(root, file);
  const isClaimSurface = relative === 'README.md'
    || relative.startsWith('docs/')
    || relative.startsWith('packaging/');

  for (const [index, line] of buffer.toString('utf8').split('\n').entries()) {
    if (/\/Users\/[^\s]+|@sjtu\.|sk-[A-Za-z0-9_-]{12,}/.test(line)) {
      findings.push(`${relative}:${index + 1}: private path, email, or credential-like value`);
    }
    if (/(^|[^A-Za-z0-9_-])agit([^A-Za-z0-9_-]|$)|\.agit([/-]|$)/.test(line)) {
      findings.push(`${relative}:${index + 1}: retired agit product name or metadata path`);
    }
    if (isClaimSurface
      && /10,?000 stars achieved|verified-live.*(claude|aider|openhands)|[0-9]+ testimonials/i.test(line)) {
      findings.push(`${relative}:${index + 1}: unsupported popularity or compatibility claim`);
    }
    if (isClaimSurface && /girelay-mcp|clone-shared|--create-pr|landing plans?|checkpoint evidence/i.test(line)) {
      findings.push(`${relative}:${index + 1}: removed product surface`);
    }
  }
}

if (findings.length > 0) {
  console.error(`Public surface check failed:\n${findings.map((item) => `- ${item}`).join('\n')}`);
  process.exit(1);
}

console.log('Public surface contains no detected private or inflated claims.');
