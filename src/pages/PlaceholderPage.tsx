import { EmptyState } from "../components/EmptyState";
import { PageHeader } from "../components/PageHeader";
import { useI18n } from "../i18n/I18nProvider";
import type { MessageId } from "../i18n/catalogs";

export function PlaceholderPage({ messageId }: { messageId: MessageId }) {
  const { t } = useI18n();
  return (
    <>
      <PageHeader title={t(messageId)} />
      <EmptyState title={t("empty.title")} detail={t("empty.detail")} />
    </>
  );
}
