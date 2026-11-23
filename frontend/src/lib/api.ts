// API クライアント。型は docs/03_design/api/openapi.yaml と一致させること。

export type Todo = {
  id: string;
  title: string;
  completed: boolean;
  createdAt: string;
  updatedAt: string;
};

export type Page<T> = { items: T[]; nextCursor: string | null };

export type ApiErrorBody = { code: string; message: string };

export class ApiError extends Error {
  constructor(
    public readonly status: number,
    public readonly body: ApiErrorBody | null,
  ) {
    super(body?.message ?? `HTTP ${status}`);
  }
}

const BASE = "/api/v1";

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    ...init,
    headers: { "content-type": "application/json", ...init?.headers },
  });
  if (!res.ok) {
    const body = (await res.json().catch(() => null)) as ApiErrorBody | null;
    throw new ApiError(res.status, body);
  }
  return (res.status === 204 ? undefined : await res.json()) as T;
}

export const api = {
  listTodos: (params: { limit?: number; cursor?: string | null } = {}) => {
    const q = new URLSearchParams();
    if (params.limit) q.set("limit", String(params.limit));
    if (params.cursor) q.set("cursor", params.cursor);
    const qs = q.toString();
    return request<Page<Todo>>(`/todos${qs ? `?${qs}` : ""}`);
  },
  createTodo: (title: string) =>
    request<Todo>("/todos", { method: "POST", body: JSON.stringify({ title }) }),
  updateTodo: (id: string, patch: Partial<Pick<Todo, "title" | "completed">>) =>
    request<Todo>(`/todos/${id}`, { method: "PATCH", body: JSON.stringify(patch) }),
  deleteTodo: (id: string) => request<void>(`/todos/${id}`, { method: "DELETE" }),
};
