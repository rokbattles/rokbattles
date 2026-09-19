import { NuqsAdapter } from "nuqs/adapters/react-router/v8";
import { BrowserRouter, Route, Routes } from "react-router";
import { AppLayout } from "./components/app-layout";
import { CookieConsentBanner } from "./components/cookie-consent-banner";
import { CookieConsentDrawer } from "./components/cookie-consent-drawer";
import { DocsLayout } from "./components/docs/layout";
import { MarketingLayout } from "./components/marketing-layout";
import { ScrollToHash } from "./components/scroll-to-hash";
import { installationDocs, legalDocuments } from "./content/metadata";
import CombatLabRoute from "./pages/app/combat-lab.tsx";
import AppIndexRoute from "./pages/app/index.tsx";
import LootExplorerRoute from "./pages/app/loot-explorer.tsx";
import OlympianArenaRoute from "./pages/app/olympian-arena.tsx";
import TerritoryLabRoute from "./pages/app/territory-lab.tsx";
import DocsRoute from "./pages/docs/index.tsx";
import InstallationRoute from "./pages/docs/installation/[slug].tsx";
import IndexRoute from "./pages/index.tsx";
import LegalDocumentRoute from "./pages/legal/[slug].tsx";
import LegalRoute from "./pages/legal/index.tsx";
import NotFoundRoute from "./pages/not-found.tsx";

export default function App() {
  return (
    <BrowserRouter>
      <ScrollToHash />
      <CookieConsentBanner />
      <CookieConsentDrawer />
      <NuqsAdapter>
        <Routes>
          <Route element={<MarketingLayout />}>
            <Route index element={<IndexRoute />} />
            <Route path="docs" element={<DocsLayout />}>
              <Route index element={<DocsRoute />} />
              {installationDocs.map((document) => (
                <Route
                  key={document.slug}
                  path={`installation/${document.slug}`}
                  element={<InstallationRoute document={document} />}
                />
              ))}
            </Route>
            <Route path="legal">
              <Route index element={<LegalRoute />} />
              {legalDocuments.map((document) => (
                <Route
                  key={document.id}
                  path={document.id}
                  element={<LegalDocumentRoute document={document} />}
                />
              ))}
            </Route>
          </Route>
          <Route path="app" element={<AppLayout />}>
            <Route index element={<AppIndexRoute />} />
            <Route path="olympian-arena" element={<OlympianArenaRoute />} />
            <Route path="combat-lab" element={<CombatLabRoute />} />
            <Route path="loot-explorer" element={<LootExplorerRoute />} />
            <Route path="territory-lab" element={<TerritoryLabRoute />} />
          </Route>
          <Route path="*" element={<NotFoundRoute />} />
        </Routes>
      </NuqsAdapter>
    </BrowserRouter>
  );
}
