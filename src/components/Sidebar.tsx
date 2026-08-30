import { NavLink } from "react-router-dom";
import { useI18n } from "../i18n/I18nProvider";
import type { MessageId } from "../i18n/catalogs";

const items: Array<{ to: string; label: MessageId }> = [
  { to: "/", label: "pages.dashboard" },
  { to: "/patients", label: "pages.patients" },
  { to: "/vaccinations/new", label: "pages.newVaccination" },
  { to: "/history", label: "pages.history" },
  { to: "/reports", label: "pages.reports" },
  { to: "/settings", label: "pages.settings" },
];

export function Sidebar() {
  const { t } = useI18n();
  return (
    <aside className="sidebar">
      <nav aria-label={t("nav.primary")}>
        {items.map((item) => (
          <NavLink key={item.to} to={item.to} end={item.to === "/"}>
            {t(item.label)}
          </NavLink>
        ))}
      </nav>
    </aside>
  );
}
