"use client";

import { useEffect, useState, type FormEvent } from "react";

import { api, type Todo } from "@/lib/api";
import styles from "./TodoApp.module.css";

const PAGE_SIZE = 20;

export function TodoApp() {
  const [todos, setTodos] = useState<Todo[]>([]);
  const [title, setTitle] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [nextCursor, setNextCursor] = useState<string | null>(null);

  useEffect(() => {
    api
      .listTodos({ limit: PAGE_SIZE })
      .then((page) => {
        setTodos(page.items);
        setNextCursor(page.nextCursor);
      })
      .catch((e: Error) => setError(e.message))
      .finally(() => setLoading(false));
  }, []);

  async function loadMore() {
    if (!nextCursor) return;
    try {
      const page = await api.listTodos({ limit: PAGE_SIZE, cursor: nextCursor });
      setTodos((prev) => [...prev, ...page.items]);
      setNextCursor(page.nextCursor);
    } catch (err) {
      setError((err as Error).message);
    }
  }

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    if (!title.trim()) return;
    try {
      const created = await api.createTodo(title);
      setTodos((prev) => [created, ...prev]);
      setTitle("");
      setError(null);
    } catch (err) {
      setError((err as Error).message);
    }
  }

  async function toggle(todo: Todo) {
    try {
      const updated = await api.updateTodo(todo.id, { completed: !todo.completed });
      setTodos((prev) => prev.map((t) => (t.id === updated.id ? updated : t)));
    } catch (err) {
      setError((err as Error).message);
    }
  }

  async function remove(id: string) {
    try {
      await api.deleteTodo(id);
      setTodos((prev) => prev.filter((t) => t.id !== id));
    } catch (err) {
      setError((err as Error).message);
    }
  }

  return (
    <section className={styles.container}>
      <form onSubmit={handleSubmit} className={styles.form}>
        <input
          aria-label="新しいタスク"
          placeholder="やること"
          value={title}
          maxLength={255}
          onChange={(e) => setTitle(e.target.value)}
        />
        <button type="submit" disabled={!title.trim()}>
          追加
        </button>
      </form>
      {error && (
        <p role="alert" className={styles.error}>
          {error}
        </p>
      )}
      {loading ? (
        <p>読み込み中…</p>
      ) : todos.length === 0 ? (
        <p>タスクはありません</p>
      ) : (
        <ul className={styles.list} aria-label="タスク一覧">
          {todos.map((t) => (
            <li key={t.id} className={styles.item}>
              <label>
                <input type="checkbox" checked={t.completed} onChange={() => toggle(t)} />
                <span className={t.completed ? styles.done : undefined}>{t.title}</span>
              </label>
              <button type="button" aria-label={`${t.title} を削除`} onClick={() => remove(t.id)}>
                削除
              </button>
            </li>
          ))}
        </ul>
      )}
      {nextCursor && (
        <button type="button" className={styles.more} onClick={loadMore}>
          もっと見る
        </button>
      )}
    </section>
  );
}
