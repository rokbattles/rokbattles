import type { ReactNode } from "react";
import { HashRouter } from "react-router";

import { AppRoutes } from "./AppRoutes.tsx";

function App(): ReactNode {
  return (
    <HashRouter>
      <AppRoutes />
    </HashRouter>
  );
}

export default App;
