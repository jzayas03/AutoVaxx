import { Navigate, Route, Routes } from "react-router-dom";
import { AppShell } from "./components/AppShell";
import { PlaceholderPage } from "./pages/PlaceholderPage";
import { DashboardPage } from "./pages/DashboardPage";

export function App() {
  return (
    <AppShell>
      <Routes>
        <Route path="/" element={<DashboardPage />} />
        <Route
          path="/patients"
          element={<PlaceholderPage messageId="pages.patients" />}
        />
        <Route
          path="/vaccinations/new"
          element={<PlaceholderPage messageId="pages.newVaccination" />}
        />
        <Route
          path="/history"
          element={<PlaceholderPage messageId="pages.history" />}
        />
        <Route
          path="/reports"
          element={<PlaceholderPage messageId="pages.reports" />}
        />
        <Route
          path="/settings"
          element={<PlaceholderPage messageId="pages.settings" />}
        />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </AppShell>
  );
}
