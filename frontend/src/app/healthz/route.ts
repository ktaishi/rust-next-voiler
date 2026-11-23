// Liveness: Next.js サーバが応答できるか (ロードバランサ / コンテナのヘルスチェック用)。
// backend の状態は見ない (backend 障害で frontend まで入れ替えられるのを避けるため)。
export function GET() {
  return Response.json({ status: "ok" }, { headers: { "cache-control": "no-store" } });
}
