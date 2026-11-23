// 共通設定。閾値 (SLO) は docs/05_test/load-test-plan.md と一致させること。
export const BASE_URL = __ENV.BASE_URL || "http://localhost:8080";

export const thresholds = {
  http_req_failed: ["rate<0.01"], // エラー率 1% 未満
  http_req_duration: ["p(95)<200", "p(99)<500"], // ms
  checks: ["rate>0.99"],
};
