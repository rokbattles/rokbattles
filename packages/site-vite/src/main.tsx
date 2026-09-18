import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./assets/globals.css";
import App from "./app.tsx";
import { CookieConsentProvider } from "./providers/cookie-consent-context";

// biome-ignore lint/style/noNonNullAssertion: required for vite
createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <CookieConsentProvider>
      <App />
    </CookieConsentProvider>
  </StrictMode>
);
