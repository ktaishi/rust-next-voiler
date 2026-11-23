// バックエンドへのリバースプロキシ。ルートハンドラから分離して単体テスト可能にしている。

const HOP_BY_HOP = new Set([
  "connection",
  "keep-alive",
  "transfer-encoding",
  "upgrade",
  "host",
  "content-length",
]);

export function backendOrigin(): string {
  return process.env.API_ORIGIN ?? "http://localhost:8080";
}

export async function proxyToBackend(req: Request, path: string[]): Promise<Response> {
  const src = new URL(req.url);
  const target = new URL(`/api/${path.map(encodeURIComponent).join("/")}`, backendOrigin());
  target.search = src.search;

  const headers = new Headers();
  req.headers.forEach((v, k) => {
    if (!HOP_BY_HOP.has(k.toLowerCase())) headers.set(k, v);
  });

  const hasBody = !["GET", "HEAD"].includes(req.method);
  try {
    const res = await fetch(target, {
      method: req.method,
      headers,
      body: hasBody ? await req.arrayBuffer() : undefined,
      cache: "no-store",
      redirect: "manual",
    });
    const out = new Headers(res.headers);
    for (const h of HOP_BY_HOP) out.delete(h);
    out.delete("content-encoding"); // fetch が展開済みのため
    return new Response(res.status === 204 ? null : res.body, {
      status: res.status,
      headers: out,
    });
  } catch (e) {
    console.error("proxy error", target.toString(), e);
    return Response.json(
      { code: "BAD_GATEWAY", message: "backend is unavailable" },
      { status: 502 },
    );
  }
}
