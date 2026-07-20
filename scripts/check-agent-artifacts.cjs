#!/usr/bin/env node

const fs = require("node:fs");
const path = require("node:path");

const root = path.resolve(__dirname, "..");
const files = {
  transcript: "assets/demo/multi-agent-relay-transcript.txt",
  tests: "assets/demo/multi-agent-relay-tests.txt",
  gif: "assets/demo/multi-agent-relay.gif",
  mp4: "assets/demo/multi-agent-relay.mp4",
  piJson: "docs/evidence/pi-v2-live-2026-07-20.json",
  piMarkdown: "docs/evidence/pi-v2-live-2026-07-20.md",
  relayJson: "docs/evidence/multi-agent-relay-2026-07-20.json",
  relayMarkdown: "docs/evidence/multi-agent-relay-2026-07-20.md",
};

function read(relativePath) {
  const absolutePath = path.join(root, relativePath);
  let contents;

  try {
    contents = fs.readFileSync(absolutePath);
  } catch (error) {
    throw new Error(`cannot read ${relativePath}: ${error.message}`);
  }

  if (contents.length === 0) {
    throw new Error(`required artifact is empty: ${relativePath}`);
  }

  return contents;
}

function requireText(contents, expected, relativePath) {
  if (!contents.includes(expected)) {
    throw new Error(`${relativePath} is missing expected text: ${expected}`);
  }
}

const contents = Object.fromEntries(
  Object.entries(files).map(([name, relativePath]) => [name, read(relativePath)]),
);
const transcript = contents.transcript.toString("utf8");
const tests = contents.tests.toString("utf8");

requireText(
  transcript,
  "Codex codex-cli 0.144.3 -> Claude Code 2.1.215",
  files.transcript,
);
requireText(
  transcript,
  "Restored source branch main to <baseline-commit>",
  files.transcript,
);
requireText(tests, "Ran 3 tests", files.tests);

for (const name of ["piJson", "relayJson"]) {
  try {
    JSON.parse(contents[name].toString("utf8"));
  } catch (error) {
    throw new Error(`${files[name]} is not valid JSON: ${error.message}`);
  }
}

const privatePattern = new RegExp([
  "/" + "Users/",
  "/private/" + "var/",
  "/" + "tmp/",
  "s" + "k-[A-Za-z0-9_-]{12,}",
].join("|"));
for (const name of [
  "transcript",
  "tests",
  "piJson",
  "piMarkdown",
  "relayJson",
  "relayMarkdown",
]) {
  if (privatePattern.test(contents[name].toString("utf8"))) {
    throw new Error(`publishable agent evidence contains a private path or credential pattern: ${files[name]}`);
  }
}

console.log("Agent integration artifacts are complete, parseable, and sanitized.");
