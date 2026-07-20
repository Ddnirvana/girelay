const fs = require('fs');
const path = require('path');
const { chromium } = require('playwright');

async function main() {
  const root = path.resolve(__dirname, '..');
  const transcriptPath = process.env.GIRELAY_TRANSCRIPT
    ? path.resolve(process.env.GIRELAY_TRANSCRIPT)
    : path.join(root, 'assets/demo/girelay-run-demo-transcript.txt');
  const frameDir = process.env.GIRELAY_FRAME_DIR
    ? path.resolve(process.env.GIRELAY_FRAME_DIR)
    : path.join(root, 'target/demo-frames');
  const label = process.env.GIRELAY_DEMO_LABEL || 'deterministic lifecycle';
  fs.rmSync(frameDir, { recursive: true, force: true });
  fs.mkdirSync(frameDir, { recursive: true });
  const lines = fs.readFileSync(transcriptPath, 'utf8').trimEnd().split('\n');
  const chunks = [];
  const finalStart = Math.max(0, lines.length - 18);
  for (let start = 0; start < finalStart; start += 14) {
    chunks.push(lines.slice(start, start + 18));
  }
  chunks.push(lines.slice(finalStart));

  const browser = await chromium.launch({ headless: true, executablePath: findChromium() });
  const page = await browser.newPage({ viewport: { width: 1280, height: 720 }, deviceScaleFactor: 1 });
  for (let index = 0; index < chunks.length; index += 1) {
    const escaped = chunks[index]
      .join('\n')
      .replaceAll('&', '&amp;')
      .replaceAll('<', '&lt;')
      .replaceAll('>', '&gt;');
    await page.setContent(`<!doctype html><style>
      * { box-sizing: border-box; }
      body { margin: 0; background: #101412; color: #e6ece8; font-family: Menlo, Monaco, monospace; }
      header { height: 74px; display: flex; align-items: center; padding: 0 42px; color: #72d68a; font-size: 28px; border-bottom: 1px solid #344039; }
      pre { margin: 0; padding: 34px 42px; white-space: pre-wrap; font-size: 19px; line-height: 1.48; letter-spacing: 0; }
      .step { margin-left: auto; color: #91a198; font-size: 16px; }
    </style><header>girelay <span class="step">${label} ${index + 1}/${chunks.length}</span></header><pre>${escaped}</pre>`);
    await page.screenshot({ path: path.join(frameDir, `frame-${String(index).padStart(2, '0')}.png`) });
  }

  if (process.env.GIRELAY_SKIP_SOCIAL !== '1') {
    await page.setViewportSize({ width: 1280, height: 640 });
    await page.goto(`file://${path.join(root, 'assets/brand/social-preview.svg')}`);
    await page.screenshot({ path: path.join(root, 'assets/brand/social-preview.png') });
  }
  await browser.close();
  process.stdout.write(`${chunks.length}\n`);
}

function findChromium() {
  if (process.env.GIRELAY_CHROMIUM) return process.env.GIRELAY_CHROMIUM;
  const cache = path.join(process.env.HOME || '', 'Library/Caches/ms-playwright');
  if (!fs.existsSync(cache)) return undefined;
  const candidates = fs.readdirSync(cache)
    .filter((name) => name.startsWith('chromium_headless_shell-'))
    .sort()
    .reverse()
    .map((name) => path.join(cache, name, 'chrome-headless-shell-mac-arm64/chrome-headless-shell'));
  return candidates.find((candidate) => fs.existsSync(candidate));
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
