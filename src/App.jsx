import { useCallback, useRef, useState } from 'react';
import BoardViewer from './components/KicadBoardViewer.jsx';
import { convertKicadBoard } from './lib/boardModel.js';
import {
  CheckIcon,
  GithubIcon,
  LayersIcon,
  ResetIcon,
  UploadIcon,
} from './icons.jsx';

const MAX_FILE_SIZE = 50 * 1024 * 1024;
const LOGO_URL = `${import.meta.env.BASE_URL}kiview-logo.png`;

function App() {
  const inputRef = useRef(null);
  const [board, setBoard] = useState(null);
  const [status, setStatus] = useState('idle');
  const [error, setError] = useState('');
  const [dragging, setDragging] = useState(false);
  const [resetSignal, setResetSignal] = useState(0);
  const [showLayers, setShowLayers] = useState(false);

  const handleFile = useCallback(async (file) => {
    if (!file) return;
    if (!file.name.toLowerCase().endsWith('.kicad_pcb')) {
      setError('Choose a .kicad_pcb file to continue.');
      return;
    }
    if (file.size > MAX_FILE_SIZE) {
      setError('That file is over the 50 MB limit.');
      return;
    }

    setError('');
    setStatus('processing');
    setBoard(null);
    try {
      const nextBoard = await convertKicadBoard(file);
      setBoard(nextBoard);
      setStatus('ready');
    } catch (reason) {
      console.error(reason);
      setStatus('idle');
      setError(reason instanceof Error ? reason.message : 'We could not read that KiCad board.');
    }
  }, []);

  const onDrop = (event) => {
    event.preventDefault();
    setDragging(false);
    handleFile(event.dataTransfer.files?.[0]);
  };

  return (
    <div className="app-shell">
      <header className="topbar">
        <div className="brand">
          <img className="brand-logo" src={LOGO_URL} alt="KiView" />
          <span className="brand-tagline">*.kicad_pcb Viewer</span>
        </div>
        <div className="header-actions">
          <input
            ref={inputRef}
            className="file-input"
            type="file"
            accept=".kicad_pcb"
            onChange={(event) => {
              const file = event.target.files?.[0];
              event.target.value = '';
              handleFile(file);
            }}
          />
          <button
            className="upload-button"
            type="button"
            onClick={() => inputRef.current?.click()}
            disabled={status === 'processing'}
          >
            <UploadIcon size={17} />
            <span>{status === 'processing' ? 'Opening…' : 'Upload'}</span>
          </button>
          <a className="github-link" href="https://github.com/shishir-dey/KiView" target="_blank" rel="noreferrer" aria-label="View KiView on GitHub">
            <GithubIcon />
          </a>
        </div>
      </header>

      <main>
        <section className="viewer-section">
          <div
            className={`workspace-card ${dragging ? 'dragging' : ''}`}
            onDragEnter={(event) => { event.preventDefault(); setDragging(true); }}
            onDragOver={(event) => event.preventDefault()}
            onDragLeave={() => setDragging(false)}
            onDrop={onDrop}
          >
            <div className="viewer-pane">
              <div className="viewer-surface">
                {board && (
                  <>
                    <BoardViewer board={board} resetSignal={resetSignal} />
                    <div className="viewer-toolbar">
                      <button type="button" onClick={() => setResetSignal((value) => value + 1)}>
                        <ResetIcon /> Reset view
                      </button>
                      <span className="toolbar-divider" />
                      <button type="button" className={showLayers ? 'active' : ''} onClick={() => setShowLayers((value) => !value)}>
                        <LayersIcon /> Layers
                      </button>
                    </div>
                    <div className="viewer-hint">
                      <span className="mouse-icon" />
                      Drag to rotate <i>·</i> Scroll to zoom
                    </div>
                    {showLayers && (
                      <div className="layers-popover">
                        <p>Visible layers</p>
                        <span><i className="layer-dot copper" /> F.Cu <CheckIcon /></span>
                        <span><i className="layer-dot mask" /> F.Mask <CheckIcon /></span>
                        <span><i className="layer-dot board" /> Board <CheckIcon /></span>
                      </div>
                    )}
                  </>
                )}
                {error && <div className="error-message" role="alert">{error}</div>}
                {dragging && <div className="drop-overlay">Drop the board to open it</div>}
              </div>
            </div>
          </div>
        </section>
      </main>
    </div>
  );
}

export default App;
