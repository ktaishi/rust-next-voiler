import http from "k6/http";
import { check, group, sleep } from "k6";

import { BASE_URL, thresholds } from "../lib/config.js";

// PROFILE=smoke|load|stress|spike で負荷形状を切り替える
const profiles = {
  smoke: { vus: 2, duration: "30s" },
  load: {
    stages: [
      { duration: "1m", target: 50 },
      { duration: "3m", target: 50 },
      { duration: "1m", target: 0 },
    ],
  },
  stress: {
    stages: [
      { duration: "2m", target: 100 },
      { duration: "3m", target: 200 },
      { duration: "2m", target: 300 },
      { duration: "1m", target: 0 },
    ],
  },
  spike: {
    stages: [
      { duration: "10s", target: 10 },
      { duration: "10s", target: 300 },
      { duration: "30s", target: 300 },
      { duration: "10s", target: 10 },
    ],
  },
};

const profile = __ENV.PROFILE || "smoke";
export const options = {
  ...profiles[profile],
  thresholds,
  summaryTrendStats: ["avg", "min", "med", "max", "p(90)", "p(95)", "p(99)"],
};

const headers = { "Content-Type": "application/json" };

export default function () {
  group("todo lifecycle", () => {
    const created = http.post(
      `${BASE_URL}/api/v1/todos`,
      JSON.stringify({ title: `k6-${__VU}-${__ITER}` }),
      { headers, tags: { name: "POST /todos" } },
    );
    check(created, { "create 201": (r) => r.status === 201 });
    const id = created.json("id");

    const list = http.get(`${BASE_URL}/api/v1/todos?limit=20`, { tags: { name: "GET /todos" } });
    check(list, { "list 200": (r) => r.status === 200 });

    if (id) {
      const upd = http.patch(`${BASE_URL}/api/v1/todos/${id}`, JSON.stringify({ completed: true }), {
        headers,
        tags: { name: "PATCH /todos/:id" },
      });
      check(upd, { "update 200": (r) => r.status === 200 });

      const del = http.del(`${BASE_URL}/api/v1/todos/${id}`, null, {
        tags: { name: "DELETE /todos/:id" },
      });
      check(del, { "delete 204": (r) => r.status === 204 });
    }
  });
  sleep(0.5);
}

export function handleSummary(data) {
  return {
    "loadtest/results/summary.json": JSON.stringify(data, null, 2),
    stdout: textSummary(data),
  };
}

function textSummary(data) {
  const d = data.metrics.http_req_duration.values;
  const f = data.metrics.http_req_failed.values;
  return [
    `profile=${profile}`,
    `requests=${data.metrics.http_reqs.values.count}`,
    `p95=${d["p(95)"].toFixed(1)}ms p99=${d["p(99)"].toFixed(1)}ms`,
    `error_rate=${(f.rate * 100).toFixed(2)}%`,
    "",
  ].join("\n");
}
