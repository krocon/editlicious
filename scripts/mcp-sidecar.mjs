#!/usr/bin/env node
// Builds the MCP sidecar `editlicious-mcp` (design 016 E10) in release mode and puts it where
// Tauri's `bundle.externalBin` wants it: src-tauri/binaries/editlicious-mcp-<target triple>[.exe].
//
//   node scripts/mcp-sidecar.mjs                       host target
//   node scripts/mcp-sidecar.mjs universal-apple-darwin macOS universal (lipo of both)
//   node scripts/mcp-sidecar.mjs --dev                 debug build next to the debug app
import { execFileSync } from 'node:child_process';
import { copyFileSync, mkdirSync } from 'node:fs';
import { join } from 'node:path';

const root = join(import.meta.dirname, '..', 'src-tauri');
const run = (cmd, args) => execFileSync(cmd, args, { stdio: 'inherit', cwd: root });

/** File name of the sidecar for a target triple (exported for the test). */
export function sidecarName(triple) {
  return `editlicious-mcp-${triple}${triple.includes('windows') ? '.exe' : ''}`;
}

function hostTriple() {
  const out = execFileSync('rustc', ['-vV'], { encoding: 'utf8' });
  return /^host: (.+)$/m.exec(out)[1].trim();
}

function build(triple) {
  run('cargo', ['build', '--release', '-p', 'ed-mcp', '--bin', 'editlicious-mcp', '--target', triple]);
  const exe = triple.includes('windows') ? 'editlicious-mcp.exe' : 'editlicious-mcp';
  return join(root, 'target', triple, 'release', exe);
}

function main(argv) {
  if (argv.includes('--dev')) {
    run('cargo', ['build', '-p', 'ed-mcp', '--bin', 'editlicious-mcp']);
    return;
  }
  const triple = argv[0] ?? hostTriple();
  const dir = join(root, 'binaries');
  mkdirSync(dir, { recursive: true });
  const target = join(dir, sidecarName(triple));
  if (triple === 'universal-apple-darwin') {
    const parts = ['aarch64-apple-darwin', 'x86_64-apple-darwin'].map(build);
    run('lipo', ['-create', '-output', target, ...parts]);
  } else {
    copyFileSync(build(triple), target);
  }
  console.log(`sidecar: ${target}`);
}

if (process.argv[1] === import.meta.filename) main(process.argv.slice(2));
