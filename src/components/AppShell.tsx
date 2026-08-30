import type { ReactNode } from "react";
import { Header } from "./Header";
import { Sidebar } from "./Sidebar";
import { useI18n } from "../i18n/I18nProvider";

export function AppShell({ children }: { children: ReactNode }) {
  const { t } = useI18n();
  return (
    <div className="app-frame">
      <div className="synthetic-banner" role="status">
        <strong>{t("app.syntheticOnly")}</strong>
        <span>{t("app.syntheticNotice")}</span>
      </div>
      <Header />
      <div className="workspace">
        <Sidebar />
        <main className="page-content" id="main-content">
          {children}
        </main>
      </div>
    </div>
  );
}
