import { readFile } from 'node:fs/promises';
import initWasm, { parse_kicad_pcb as parseKicadPcb } from '../src/wasm/pkg/kiview_wasm.js';

const wasmBytes = await readFile(
  new URL('../src/wasm/pkg/kiview_wasm_bg.wasm', import.meta.url),
);
await initWasm({ module_or_path: wasmBytes });

const inlineFixture = `
(kicad_pcb (version 20240108)
  (general (thickness 1.6))
  (layers (0 "F.Cu" signal) (31 "B.Cu" signal) (44 "Edge.Cuts" user))
  (gr_rect (start 0 0) (end 40 30) (stroke (width 0.05) (type default)) (fill none) (layer "Edge.Cuts"))
  (footprint "Package_QFP:LQFP-48" (layer "F.Cu") (at 20 15)
    (pad "1" smd roundrect (at 2 0) (size 0.5 1.5) (layers "F.Cu" "F.Paste" "F.Mask")))
  (segment (start 5 5) (end 20 15) (width 0.25) (layer "F.Cu") (net 1))
  (via (at 10 10) (size 0.8) (drill 0.4) (layers "F.Cu" "B.Cu") (net 1)))
`;

const inputPath = process.argv[2];
const source = inputPath ? await readFile(inputPath, 'utf8') : inlineFixture;
const result = parseKicadPcb(source);

if (result.meshes.length < 1) throw new Error('Expected at least one geometry mesh.');
if (!inputPath && result.stats.pads !== 1) throw new Error('Unexpected pad count from WASM parser.');
if (!result.meshes.every((mesh) => mesh.positions.length && mesh.indices.length)) {
  throw new Error('WASM returned an empty geometry buffer.');
}

console.log(JSON.stringify({
  input: inputPath ?? 'inline fixture',
  meshes: result.meshes.length,
  vertices: result.meshes.reduce((sum, mesh) => sum + mesh.positions.length / 3, 0),
  stats: result.stats,
  bounds: result.bounds,
}, null, 2));
