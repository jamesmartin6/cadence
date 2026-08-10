import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import initWasm from "./wasm/crdt_engine";
import { App } from "./App";
import "./index.css";

// The WASM module must finish loading before anything tries to construct a CrdtDoc, so
// this awaits it up front rather than pushing the async dance into every component.
initWasm().then(() => {
  createRoot(document.getElementById("root")!).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
});
