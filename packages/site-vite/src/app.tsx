import { NuqsAdapter } from "nuqs/adapters/react-router/v8";
import { BrowserRouter, Route, Routes } from "react-router";
import { AppLayout } from "./components/app-layout";
import { MarketingLayout } from "./components/marketing-layout";
import { ScrollToHash } from "./components/scroll-to-hash";
import { legalDocuments } from "./lib/legal-documents";
import AppIndexRoute from "./pages/app/index.tsx";
import IndexRoute from "./pages/index.tsx";
import LegalDocumentRoute from "./pages/legal/[slug].tsx";
import LegalRoute from "./pages/legal/index.tsx";
import NotFoundRoute from "./pages/not-found.tsx";

export default function App() {
  return (
    <BrowserRouter>
      <ScrollToHash />
      <NuqsAdapter>
        <Routes>
          <Route element={<MarketingLayout />}>
            <Route index element={<IndexRoute />} />
            <Route path="legal" element={<LegalRoute />} />
            {legalDocuments.map((document) => (
              <Route
                key={document.id}
                path={`legal/${document.id}`}
                element={<LegalDocumentRoute document={document} />}
              />
            ))}
          </Route>
          <Route path="app" element={<AppLayout />}>
            <Route index element={<AppIndexRoute />} />
          </Route>
          <Route path="*" element={<NotFoundRoute />} />
        </Routes>
      </NuqsAdapter>
    </BrowserRouter>
  );
}
