import { fireEvent, render, screen } from "@testing-library/react";
import { useRef } from "react";
import { afterAll, beforeAll, describe, expect, it, vi } from "vitest";
import { PingModeOverlay } from "./PingModeOverlay";

class TestPointerEvent extends MouseEvent {
  pointerId: number;

  constructor(type: string, init: PointerEventInit = {}) {
    super(type, init);
    this.pointerId = init.pointerId ?? 0;
  }
}

beforeAll(() => vi.stubGlobal("PointerEvent", TestPointerEvent));
afterAll(() => vi.unstubAllGlobals());

function Harness({ onWhip = () => {} }: { onWhip?: () => void }) {
  const rootRef = useRef<HTMLElement>(null);
  return (
    <main ref={rootRef} data-testid="root">
      <PingModeOverlay rootRef={rootRef} onWhip={onWhip} />
    </main>
  );
}

describe("PingModeOverlay", () => {
  it("enters with G and emits a basic ping on left click", () => {
    render(<Harness />);
    const root = screen.getByTestId("root");

    fireEvent.keyDown(window, { key: "g" });
    expect(root).toHaveClass("ping-mode-active");

    fireEvent.pointerDown(root, { button: 0, pointerId: 1, clientX: 80, clientY: 90 });
    fireEvent.pointerUp(window, { button: 0, pointerId: 1, clientX: 80, clientY: 90 });

    expect(root).not.toHaveClass("ping-mode-active");
    expect(document.querySelector(".ping-burst")).toBeInTheDocument();
  });

  it("selects the upward whip gesture", () => {
    const onWhip = vi.fn();
    render(<Harness onWhip={onWhip} />);
    const root = screen.getByTestId("root");

    fireEvent.keyDown(window, { key: "G" });
    fireEvent.pointerDown(root, { button: 0, pointerId: 2, clientX: 120, clientY: 160 });
    fireEvent.pointerMove(window, { pointerId: 2, clientX: 120, clientY: 90 });
    expect(screen.getByText("채찍")).toBeInTheDocument();
    fireEvent.pointerUp(window, { button: 0, pointerId: 2, clientX: 120, clientY: 90 });

    expect(onWhip).toHaveBeenCalledOnce();
    expect(root).not.toHaveClass("ping-mode-active");
  });

  it("cancels the mode with right click", () => {
    render(<Harness />);
    const root = screen.getByTestId("root");

    fireEvent.keyDown(window, { key: "g" });
    fireEvent.pointerDown(root, { button: 2, pointerId: 3 });
    fireEvent.contextMenu(root);

    expect(root).not.toHaveClass("ping-mode-active");
  });
});
