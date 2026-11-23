import { expect, test } from "@playwright/test";

test("タスクの追加・完了・削除ができる", async ({ page }) => {
  const title = `e2e-${Date.now()}`;
  await page.goto("/");
  await page.getByLabel("新しいタスク").fill(title);
  await page.getByRole("button", { name: "追加" }).click();

  const item = page.getByRole("listitem").filter({ hasText: title });
  await expect(item).toBeVisible();

  // サーバ応答後に state が反映される制御コンポーネントのため click + 待機で検証
  await item.getByRole("checkbox").click();
  await expect(item.getByRole("checkbox")).toBeChecked();

  // リロードしても永続化されている (内部結合: Front -> API -> MySQL)
  await page.reload();
  await expect(
    page.getByRole("listitem").filter({ hasText: title }).getByRole("checkbox"),
  ).toBeChecked();

  await page.getByRole("button", { name: `${title} を削除` }).click();
  await expect(page.getByRole("listitem").filter({ hasText: title })).toHaveCount(0);
});
