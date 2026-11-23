import type { NextRequest } from "next/server";

import { proxyToBackend } from "@/lib/proxy";

// ブラウザ → 同一オリジン /api/* → (このハンドラ) → バックエンド。
// 転送先は実行時の環境変数 API_ORIGIN で決まるため、同じイメージを環境間で昇格できる。
const handler = (req: NextRequest, ctx: RouteContext<"/api/[...path]">) =>
  ctx.params.then(({ path }) => proxyToBackend(req, path));

export { handler as DELETE, handler as GET, handler as PATCH, handler as POST, handler as PUT };
