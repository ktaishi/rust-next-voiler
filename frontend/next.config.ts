import type { NextConfig } from "next";

// ブラウザからは同一オリジンの /api/* を叩き、Next.js がバックエンドへプロキシする。
// rewrites はビルド時に確定するため、Docker では build arg API_ORIGIN で指定する。
const apiOrigin = process.env.API_ORIGIN ?? "http://localhost:8080";

const nextConfig: NextConfig = {
  output: "standalone",
  cacheComponents: true,
  partialPrefetching: true,
  async rewrites() {
    return [{ source: "/api/:path*", destination: `${apiOrigin}/api/:path*` }];
  },
};

export default nextConfig;
