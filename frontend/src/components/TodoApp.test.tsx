import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { TodoApp } from "./TodoApp";

const todo = (over: Partial<Record<string, unknown>> = {}) => ({
  id: "11111111-1111-1111-1111-111111111111",
  title: "牛乳を買う",
  completed: false,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
  ...over,
});

const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json" } });

describe("TodoApp", () => {
  const fetchMock = vi.fn<typeof fetch>();
  beforeEach(() => vi.stubGlobal("fetch", fetchMock));
  afterEach(() => {
    vi.unstubAllGlobals();
    fetchMock.mockReset();
  });

  it("一覧を表示する", async () => {
    fetchMock.mockResolvedValueOnce(json([todo()]));
    render(<TodoApp />);
    expect(await screen.findByText("牛乳を買う")).toBeInTheDocument();
  });

  it("空のときはメッセージを表示する", async () => {
    fetchMock.mockResolvedValueOnce(json([]));
    render(<TodoApp />);
    expect(await screen.findByText("タスクはありません")).toBeInTheDocument();
  });

  it("タスクを追加できる", async () => {
    fetchMock
      .mockResolvedValueOnce(json([]))
      .mockResolvedValueOnce(json(todo({ title: "新規" }), 201));
    render(<TodoApp />);
    await screen.findByText("タスクはありません");
    await userEvent.type(screen.getByLabelText("新しいタスク"), "新規");
    await userEvent.click(screen.getByRole("button", { name: "追加" }));
    expect(await screen.findByText("新規")).toBeInTheDocument();
    expect(fetchMock).toHaveBeenLastCalledWith(
      "/api/v1/todos",
      expect.objectContaining({ method: "POST", body: JSON.stringify({ title: "新規" }) }),
    );
  });

  it("API エラーを表示する", async () => {
    fetchMock.mockResolvedValueOnce(json({ code: "INTERNAL_ERROR", message: "boom" }, 500));
    render(<TodoApp />);
    await waitFor(() => expect(screen.getByRole("alert")).toHaveTextContent("boom"));
  });
});
