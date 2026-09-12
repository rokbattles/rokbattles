import { isTauri } from "@tauri-apps/api/core";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./assets/globals.css";
import App from "./app.tsx";

if (isTauri() && window.location.pathname === "/") {
  window.history.replaceState(null, "", `/app${window.location.search}${window.location.hash}`);
}

// biome-ignore lint/style/noNonNullAssertion: required for vite
createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>
);
