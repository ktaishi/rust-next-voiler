import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { proxyToBackend } from "./proxy";

describe("proxyToBackend", () => {
  const fetchMock = vi.fn<typeof fetch>();
  beforeEach(() => {
    vi.stubGlobal("fetch", fetchMock);
    vi.stubEnv("API_ORIGIN", "http://backend:8080");
  });
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.unstubAllEnvs();
    fetchMock.mockReset();
  });

  it("実行時の API_ORIGIN へパス・クエリ・メソッド・ボディを転送する", async () => {
    fetchMock.mockResolvedValueOnce(Response.json({ ok: true }, { status: 201 }));
    const req = new Request("http://localhost:3000/api/v1/todos?limit=2", {
      method: "POST",
      headers: { "content-type": "application/json", host: "localhost:3000" },
      body: JSON.stringify({ title: "a" }),
    });

    const res = await proxyToBackend(req, ["v1", "todos"]);

    expect(res.status).toBe(201);
    const [url, init] = fetchMock.mock.calls[0];
    expect(String(url)).toBe("http://backend:8080/api/v1/todos?limit=2");
    expect(init?.method).toBe("POST");
    expect(new TextDecoder().decode(init?.body as ArrayBuffer)).toBe('{"title":"a"}');
    expect((init?.headers as Headers).get("host")).toBeNull();
  });

  it("204 はボディなしで返す", async () => {
    fetchMock.mockResolvedValueOnce(new Response(null, { status: 204 }));
    const res = await proxyToBackend(
      new Request("http://localhost:3000/api/v1/todos/x", { method: "DELETE" }),
      ["v1", "todos", "x"],
    );
    expect(res.status).toBe(204);
    expect(await res.text()).toBe("");
  });

  it("バックエンドに到達できなければ 502", async () => {
    fetchMock.mockRejectedValueOnce(new TypeError("fetch failed"));
    vi.spyOn(console, "error").mockImplementation(() => {});
    const res = await proxyToBackend(new Request("http://localhost:3000/api/v1/todos"), [
      "v1",
      "todos",
    ]);
    expect(res.status).toBe(502);
    expect((await res.json()).code).toBe("BAD_GATEWAY");
  });
});
