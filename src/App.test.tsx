import { fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { App } from "./App";
import { I18nProvider } from "./i18n/I18nProvider";
import { resolveMessage } from "./i18n/catalogs";

function renderApp() {
  return render(
    <I18nProvider>
      <MemoryRouter>
        <App />
      </MemoryRouter>
    </I18nProvider>,
  );
}

describe("Phase 1 desktop shell", () => {
  it("shows an unavoidable synthetic-only warning", () => {
    renderApp();
    expect(screen.getByText("Synthetic-only development build")).toBeVisible();
    expect(
      screen.getByText("Never enter real patient information."),
    ).toBeVisible();
  });

  it("switches all visible shell content to Spanish", () => {
    renderApp();
    fireEvent.change(screen.getByLabelText("Change language"), {
      target: { value: "es" },
    });
    expect(
      screen.getByRole("heading", { name: "Área de documentación" }),
    ).toBeVisible();
    expect(
      screen.getByText("Nunca ingrese información real de pacientes."),
    ).toBeVisible();
  });

  it("falls back unknown routes to the dashboard", () => {
    render(
      <I18nProvider>
        <MemoryRouter initialEntries={["/not-real"]}>
          <App />
        </MemoryRouter>
      </I18nProvider>,
    );
    expect(
      screen.getByRole("heading", { name: "Documentation workspace" }),
    ).toBeVisible();
  });

  it("falls back to English when a locale catalog entry is unavailable", () => {
    expect(
      resolveMessage("es", "pages.reports", {
        en: { "pages.reports": "Reports" },
        es: {},
      }),
    ).toBe("Reports");
  });
});
