import initWasm, { parse_kicad_pcb as parseKicadPcb } from '../wasm/pkg/kiview_wasm.js';

let wasmInitialization;

function initializeWasm() {
  if (!wasmInitialization) {
    wasmInitialization = initWasm();
  }
  return wasmInitialization;
}

/**
 * Rust parses KiCad directly into mesh-ready arrays. JavaScript only attaches
 * those arrays to THREE.BufferGeometry in KicadBoardViewer.
 */
export async function convertKicadBoard(file) {
  const source = await file.text();
  await new Promise((resolve) => requestAnimationFrame(resolve));
  await initializeWasm();

  try {
    const geometry = parseKicadPcb(source);
    return {
      name: file.name,
      size: file.size,
      ...geometry,
    };
  } catch (error) {
    const message = typeof error === 'string' ? error : error?.message;
    throw new Error(message || 'Rust/WASM could not parse this KiCad board.');
  }
}
