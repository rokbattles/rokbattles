import { NuqsAdapter } from "nuqs/adapters/react-router/v8";
import { BrowserRouter, Route, Routes } from "react-router";
import Index from "./pages/Index.tsx";
import NotFound from "./pages/NotFound.tsx";

export default function App() {
  return (
    <BrowserRouter>
      <NuqsAdapter>
        <Routes>
          <Route index element={<Index />} />
          <Route path="*" element={<NotFound />} />
        </Routes>
      </NuqsAdapter>
    </BrowserRouter>
  );
}
