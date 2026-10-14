import type { ReactNode } from "react";
import { Route, Routes } from "react-router";

import { AppHeader } from "./components/AppHeader.tsx";
import { useLogEvents } from "./hooks/useLogEvents.ts";
import { HomePage } from "./pages/HomePage.tsx";
import { SettingsPage } from "./pages/SettingsPage.tsx";

export function AppRoutes(): ReactNode {
  const watcherLogs = useLogEvents("rokbattles");

  return (
    <main className="min-h-screen bg-zinc-950 text-zinc-100">
      <div className="mx-auto max-w-3xl p-6">
        <AppHeader />
        <Routes>
          <Route path="/" element={<HomePage logs={watcherLogs} />} />
          <Route path="/settings" element={<SettingsPage />} />
        </Routes>
      </div>
    </main>
  );
}
