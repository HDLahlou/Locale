import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { Badge, Button, StatusLabel, Tabs, tokens } from "@locale/ui";

afterEach(() => {
  cleanup();
  document.documentElement.removeAttribute("data-theme");
});

describe.each(["light", "dark"] as const)("UI kit in the desktop app, %s theme", (theme) => {
  it("renders a sample through the workspace dependency", () => {
    document.documentElement.setAttribute("data-theme", theme);
    render(
      <>
        <Button>Start event</Button>
        <StatusLabel status="ok">Live</StatusLabel>
        <Badge kind="role" role="viewer" />
        <Tabs
          label="Sections"
          value="a"
          onChange={() => {}}
          tabs={[{ id: "a", label: "Photos" }]}
        />
      </>,
    );
    expect(screen.getByRole("button", { name: "Start event" })).toBeInTheDocument();
    expect(screen.getByText("Live")).toBeInTheDocument();
    expect(screen.getByText("Guest")).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "Photos" })).toHaveAttribute("aria-selected", "true");
    expect(tokens.colors[theme].page).toMatch(/^#[0-9A-F]{6}$/);
  });
});
