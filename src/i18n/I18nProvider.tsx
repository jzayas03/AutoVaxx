import {
  createContext,
  useContext,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { resolveMessage, type Locale, type MessageId } from "./catalogs";

type I18nContextValue = {
  locale: Locale;
  setLocale: (locale: Locale) => void;
  t: (messageId: MessageId) => string;
};

const I18nContext = createContext<I18nContextValue | null>(null);

export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocale] = useState<Locale>("en");
  const value = useMemo<I18nContextValue>(
    () => ({
      locale,
      setLocale,
      t: (messageId) => resolveMessage(locale, messageId),
    }),
    [locale],
  );

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n() {
  const context = useContext(I18nContext);
  if (!context) throw new Error("useI18n must be used within I18nProvider");
  return context;
}
