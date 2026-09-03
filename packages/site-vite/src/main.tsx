import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./assets/globals.css";
import App from "./App.tsx";

// biome-ignore lint/style/noNonNullAssertion: required for vite
createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>
);
