<p align="center">
  <img src="public/kiview-logo.png" width="560" alt="KiView logo" />
</p>

# KiView

A fast, private, browser-based 3D viewer for KiCad PCB files. Board files are parsed locally with Rust and WebAssembly, then rendered with Three.js.

## Getting started

Install [Node.js](https://nodejs.org/), [Rust](https://rustup.rs/), and `wasm-pack`, then run:

```sh
cargo install wasm-pack --locked --version 0.13.1
npm ci
npm run dev
```

Create and preview a production build:

```sh
npm run build
npm run preview
```

Run the Rust parser tests and WebAssembly smoke test:

```sh
npm run test:wasm
```

## Project structure

```text
KiView/
├── public/               Brand, favicon, and web app assets
├── scripts/              WebAssembly smoke test
├── src/                  React UI and Three.js board viewer
│   ├── components/       Interactive 3D viewer
│   └── lib/              WebAssembly integration
├── wasm/                 Rust KiCad parser compiled to WebAssembly
├── .github/workflows/    GitHub Pages deployment
├── index.html            Site metadata and application entry point
└── package.json          npm scripts and project metadata
```

## License

[MIT](LICENSE)
