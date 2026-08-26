import { BrowserRouter, Route, Routes } from "react-router";
import Index from "./pages/Index.tsx";

export default function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route index element={<Index />} />
      </Routes>
    </BrowserRouter>
  );
}
