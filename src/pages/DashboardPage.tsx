import { FormSection } from "../components/FormSection";
import { PageHeader } from "../components/PageHeader";
import { StatusBadge } from "../components/StatusBadge";
import { useI18n } from "../i18n/I18nProvider";

export function DashboardPage() {
  const { t } = useI18n();
  const cards = [
    ["dashboard.offline", "dashboard.offlineDetail"],
    ["dashboard.boundary", "dashboard.boundaryDetail"],
    ["dashboard.scope", "dashboard.scopeDetail"],
  ] as const;
  return (
    <>
      <PageHeader
        title={t("dashboard.heading")}
        description={t("dashboard.intro")}
      />
      <FormSection title={t("dashboard.status")}>
        <div className="foundation-grid">
          {cards.map(([title, detail]) => (
            <article className="foundation-card" key={title}>
              <StatusBadge label={t("status.foundation")} />
              <h3>{t(title)}</h3>
              <p>{t(detail)}</p>
            </article>
          ))}
        </div>
      </FormSection>
    </>
  );
}
