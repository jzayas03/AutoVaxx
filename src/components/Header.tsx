import { useI18n } from "../i18n/I18nProvider";

export function Header() {
  const { locale, setLocale, t } = useI18n();
  return (
    <header className="app-header">
      <div>
        <div className="brand">{t("app.name")}</div>
        <div className="brand-subtitle">{t("app.subtitle")}</div>
      </div>
      <label className="language-control">
        <span>{t("actions.changeLanguage")}</span>
        <select
          value={locale}
          onChange={(event) => setLocale(event.target.value as "en" | "es")}
        >
          <option value="en">English</option>
          <option value="es">Español</option>
        </select>
      </label>
    </header>
  );
}
