import { NuqsAdapter } from "nuqs/adapters/react-router/v8";
import { BrowserRouter, Route, Routes } from "react-router";
import { AppLayout } from "./components/app-layout";
import Index from "./pages/index.tsx";
import NotFound from "./pages/not-found.tsx";

export default function App() {
  return (
    <BrowserRouter>
      <NuqsAdapter>
        <Routes>
          <Route element={<AppLayout />}>
            <Route index element={<Index />} />
          </Route>
          <Route path="*" element={<NotFound />} />
        </Routes>
      </NuqsAdapter>
    </BrowserRouter>
  );
}
