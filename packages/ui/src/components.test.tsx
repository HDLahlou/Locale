import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { useState } from "react";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { Badge, Button, Dialog, Pill, Skeleton, StatusLabel, Tabs } from "./index";
import type { ButtonVariant } from "./index";

const css = readFileSync(resolve(__dirname, "components.css"), "utf8");

afterEach(() => {
  cleanup();
  document.documentElement.removeAttribute("data-theme");
});

const THEMES = ["light", "dark"] as const;

describe.each(THEMES)("components in %s theme", (theme) => {
  const setup = () => document.documentElement.setAttribute("data-theme", theme);

  it("Button renders every variant with a role and name", () => {
    setup();
    const variants: ButtonVariant[] = ["primary", "secondary", "quiet", "destructive"];
    render(
      <>
        {variants.map((v) => (
          <Button key={v} variant={v}>
            {v}
          </Button>
        ))}
      </>,
    );
    for (const v of variants) {
      const b = screen.getByRole("button", { name: v });
      expect(b).toHaveClass(`lc-button--${v}`);
      expect(b).toHaveAttribute("type", "button");
    }
    expect(document.documentElement).toHaveAttribute("data-theme", theme);
  });

  it("Button disabled and busy states do not fire clicks", () => {
    setup();
    const onClick = vi.fn();
    render(
      <>
        <Button disabled onClick={onClick}>
          Off
        </Button>
        <Button busy onClick={onClick}>
          Working
        </Button>
        <Button onClick={onClick}>On</Button>
      </>,
    );
    expect(screen.getByRole("button", { name: "Off" })).toBeDisabled();
    const busy = screen.getByRole("button", { name: "Working" });
    expect(busy).toHaveAttribute("aria-busy", "true");
    fireEvent.click(screen.getByRole("button", { name: "Off" }));
    fireEvent.click(busy);
    expect(onClick).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "On" }));
    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it("Pill is a toggle button with aria-pressed", () => {
    setup();
    const onClick = vi.fn();
    render(
      <>
        <Pill selected onClick={onClick}>
          Everyone
        </Pill>
        <Pill>Ana</Pill>
      </>,
    );
    expect(screen.getByRole("button", { name: "Everyone" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(screen.getByRole("button", { name: "Ana" })).toHaveAttribute("aria-pressed", "false");
    fireEvent.click(screen.getByRole("button", { name: "Everyone" }));
    expect(onClick).toHaveBeenCalled();
  });

  it("Tabs has roving focus, aria-selected and arrow-key navigation", () => {
    setup();
    function Demo() {
      const [v, setV] = useState("a");
      return (
        <Tabs
          label="Sections"
          value={v}
          onChange={setV}
          tabs={[
            { id: "a", label: "Photos", content: "photos panel" },
            { id: "b", label: "People", content: "people panel" },
            { id: "c", label: "Posts", content: "posts panel" },
          ]}
        />
      );
    }
    render(<Demo />);
    expect(screen.getByRole("tablist", { name: "Sections" })).toBeInTheDocument();
    const photos = screen.getByRole("tab", { name: "Photos" });
    const people = screen.getByRole("tab", { name: "People" });
    expect(photos).toHaveAttribute("aria-selected", "true");
    expect(people).toHaveAttribute("aria-selected", "false");
    expect(photos).toHaveAttribute("tabindex", "0");
    expect(people).toHaveAttribute("tabindex", "-1");
    expect(screen.getByRole("tabpanel")).toHaveTextContent("photos panel");

    photos.focus();
    fireEvent.keyDown(photos, { key: "ArrowRight" });
    expect(people).toHaveAttribute("aria-selected", "true");
    expect(people).toHaveFocus();
    expect(screen.getByRole("tabpanel")).toHaveTextContent("people panel");
    fireEvent.keyDown(people, { key: "End" });
    expect(screen.getByRole("tab", { name: "Posts" })).toHaveFocus();
    fireEvent.keyDown(screen.getByRole("tab", { name: "Posts" }), { key: "ArrowRight" });
    expect(photos).toHaveFocus();
    fireEvent.keyDown(photos, { key: "ArrowLeft" });
    expect(screen.getByRole("tab", { name: "Posts" })).toHaveFocus();
    fireEvent.keyDown(screen.getByRole("tab", { name: "Posts" }), { key: "Home" });
    expect(photos).toHaveFocus();
  });

  it("StatusLabel always shows a word and an icon for each status", () => {
    setup();
    render(
      <>
        <StatusLabel status="ok">Live</StatusLabel>
        <StatusLabel status="warn">Waiting</StatusLabel>
        <StatusLabel status="error">Not sent</StatusLabel>
      </>,
    );
    for (const word of ["Live", "Waiting", "Not sent"]) {
      const el = screen.getByText(word).closest(".lc-status") as HTMLElement;
      expect(el).toHaveTextContent(word);
      expect(el.querySelector(".lc-status__icon")?.textContent).not.toBe("");
    }
    expect(screen.getByText("Live").closest(".lc-status")).toHaveClass("lc-status--ok");
  });

  it("Badge shows a labelled verified check and role tints from the label table", () => {
    setup();
    render(
      <>
        <Badge kind="verified" />
        <Badge kind="role" role="viewer" />
        <Badge kind="role" role="host" />
      </>,
    );
    expect(screen.getByRole("img", { name: "Verified" })).toBeInTheDocument();
    expect(screen.getByText("Guest")).toBeInTheDocument();
    expect(screen.getByText("Host")).toBeInTheDocument();
    expect(screen.queryByText(/viewer/i)).toBeNull();
  });

  it("Dialog is modal, labelled by its title, traps focus and closes on Escape", () => {
    setup();
    const onClose = vi.fn();
    function Demo() {
      const [open, setOpen] = useState(false);
      return (
        <>
          <button onClick={() => setOpen(true)}>Open</button>
          <Dialog
            open={open}
            title="End event"
            onClose={() => {
              onClose();
              setOpen(false);
            }}
            actions={
              <>
                <Button variant="secondary">Cancel</Button>
                <Button variant="destructive">End</Button>
              </>
            }
          >
            Everyone is disconnected.
          </Dialog>
        </>
      );
    }
    render(<Demo />);
    expect(screen.queryByRole("dialog")).toBeNull();
    const opener = screen.getByRole("button", { name: "Open" });
    opener.focus();
    fireEvent.click(opener);
    const dialog = screen.getByRole("dialog", { name: "End event" });
    expect(dialog).toHaveAttribute("aria-modal", "true");
    const cancel = screen.getByRole("button", { name: "Cancel" });
    const end = screen.getByRole("button", { name: "End" });
    expect(cancel).toHaveFocus();
    end.focus();
    fireEvent.keyDown(end, { key: "Tab" });
    expect(cancel).toHaveFocus();
    fireEvent.keyDown(cancel, { key: "Tab", shiftKey: true });
    expect(end).toHaveFocus();
    fireEvent.keyDown(end, { key: "Escape" });
    expect(onClose).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(opener).toHaveFocus();
  });

  it("Skeleton is hidden from assistive tech", () => {
    setup();
    render(<Skeleton width={120} height={12} />);
    expect(screen.getByTestId("skeleton")).toHaveAttribute("aria-hidden", "true");
  });
});

describe("stylesheet rules", () => {
  it("every interactive component has a visible focus style", () => {
    for (const cls of [".lc-button", ".lc-pill", ".lc-tabs__tab", ".lc-dialog"]) {
      expect(css).toMatch(new RegExp(`${cls}:focus-visible`));
    }
    expect(css).toMatch(/outline: 2px solid/);
  });
  it("Skeleton animation stops under reduced motion", () => {
    const block = css.slice(css.indexOf("@media (prefers-reduced-motion: reduce)"));
    expect(block).toMatch(/\.lc-skeleton\s*{[^}]*animation: none/);
  });
  it("destructive buttons are not styled like primary ones", () => {
    const rule = (sel: string) => css.match(new RegExp(`${sel}\\s*{([^}]*)}`))?.[1] ?? "";
    expect(rule("\\.lc-button--destructive")).not.toContain("--lc-primary");
    expect(rule("\\.lc-button--destructive")).toContain("--lc-error-ink");
  });
  it("components use tokens, not colour literals", () => {
    expect(css.replace(/rgb\(0 0 0 \/ 0\.5\)/, "")).not.toMatch(/#[0-9a-fA-F]{3,8}\b/);
  });
});
