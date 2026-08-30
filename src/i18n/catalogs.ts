export const catalogs = {
  en: {
    "app.name": "AutoVaxx",
    "app.subtitle": "Immunization documentation",
    "app.syntheticOnly": "Synthetic-only development build",
    "app.syntheticNotice": "Never enter real patient information.",
    "nav.primary": "Primary navigation",
    "pages.dashboard": "Dashboard",
    "pages.patients": "Patients",
    "pages.newVaccination": "New vaccination",
    "pages.history": "Vaccination history",
    "pages.reports": "Reports",
    "pages.settings": "Settings",
    "dashboard.heading": "Documentation workspace",
    "dashboard.intro":
      "Phase 1 establishes secure local foundations. The vaccination workflow is not active yet.",
    "dashboard.offline": "Offline foundation",
    "dashboard.offlineDetail":
      "Core functions do not require an internet connection.",
    "dashboard.boundary": "Trusted Rust boundary",
    "dashboard.boundaryDetail":
      "Authorization, workflow state, persistence, and audit remain outside the web interface.",
    "dashboard.scope": "Documentation-only scope",
    "dashboard.scopeDetail":
      "AutoVaxx does not determine eligibility, recommend vaccines, or claim clinical safety.",
    "dashboard.status": "Foundation status",
    "status.ready": "Ready",
    "status.foundation": "Foundation only",
    "empty.title": "Not included in Phase 1",
    "empty.detail":
      "This route is present for navigation and localization validation. Clinical workflow features begin only after Phase 1 approval.",
    "actions.changeLanguage": "Change language",
    "actions.confirm": "Confirm",
    "actions.cancel": "Cancel",
    "validation.title": "Review required items",
  },
  es: {
    "app.name": "AutoVaxx",
    "app.subtitle": "Documentación de inmunizaciones",
    "app.syntheticOnly": "Versión de desarrollo solo para datos sintéticos",
    "app.syntheticNotice": "Nunca ingrese información real de pacientes.",
    "nav.primary": "Navegación principal",
    "pages.dashboard": "Panel principal",
    "pages.patients": "Pacientes",
    "pages.newVaccination": "Nueva vacunación",
    "pages.history": "Historial de vacunación",
    "pages.reports": "Informes",
    "pages.settings": "Configuración",
    "dashboard.heading": "Área de documentación",
    "dashboard.intro":
      "La Fase 1 establece fundamentos locales seguros. El flujo de vacunación aún no está activo.",
    "dashboard.offline": "Fundamento sin conexión",
    "dashboard.offlineDetail":
      "Las funciones principales no requieren conexión a internet.",
    "dashboard.boundary": "Límite confiable en Rust",
    "dashboard.boundaryDetail":
      "La autorización, los estados, la persistencia y la auditoría permanecen fuera de la interfaz web.",
    "dashboard.scope": "Alcance solo de documentación",
    "dashboard.scopeDetail":
      "AutoVaxx no determina elegibilidad, recomienda vacunas ni declara seguridad clínica.",
    "dashboard.status": "Estado de la base técnica",
    "status.ready": "Listo",
    "status.foundation": "Solo fundamento",
    "empty.title": "No incluido en la Fase 1",
    "empty.detail":
      "Esta ruta existe para validar la navegación y localización. Las funciones clínicas comienzan solo después de aprobar la Fase 1.",
    "actions.changeLanguage": "Cambiar idioma",
    "actions.confirm": "Confirmar",
    "actions.cancel": "Cancelar",
    "validation.title": "Revise los elementos requeridos",
  },
} as const;

export type Locale = keyof typeof catalogs;
export type MessageId = keyof (typeof catalogs)["en"];

export function resolveMessage(
  locale: Locale,
  messageId: MessageId,
  catalogSet: Record<Locale, Partial<Record<MessageId, string>>> = catalogs,
): string {
  return catalogSet[locale][messageId] ?? catalogSet.en[messageId] ?? messageId;
}
