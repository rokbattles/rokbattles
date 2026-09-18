import { isTauri } from "@tauri-apps/api/core";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./assets/globals.css";
import App from "./app.tsx";

if (isTauri() && import.meta.env.PROD) {
  document.addEventListener("contextmenu", (event) => {
    event.preventDefault();
  });
}

// biome-ignore lint/style/noNonNullAssertion: required for vite
createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>
);
