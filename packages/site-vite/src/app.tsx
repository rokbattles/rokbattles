import { NuqsAdapter } from "nuqs/adapters/react-router/v8";
import { BrowserRouter, Route, Routes } from "react-router";
import { AppLayout } from "./components/app-layout";
import AppIndexRoute from "./pages/app/index.tsx";
import IndexRoute from "./pages/index.tsx";
import NotFoundRoute from "./pages/not-found.tsx";

export default function App() {
  return (
    <BrowserRouter>
      <NuqsAdapter>
        <Routes>
          <Route index element={<IndexRoute />} />
          <Route path="app" element={<AppLayout />}>
            <Route index element={<AppIndexRoute />} />
          </Route>
          <Route path="*" element={<NotFoundRoute />} />
        </Routes>
      </NuqsAdapter>
    </BrowserRouter>
  );
}
