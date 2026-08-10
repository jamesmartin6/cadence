import { test, expect, type Page } from "@playwright/test";

async function waitForEditorText(page: Page, text: string) {
  await page.waitForFunction(
    (expected) => document.querySelector<HTMLTextAreaElement>("textarea.editor")?.value === expected,
    text,
    { timeout: 5_000 },
  );
}

// These tests exercise the whole stack end to end -- real WASM CRDT engine in the
// browser, a real relay server, a real Postgres -- because that's the only way to
// actually prove the thing the build plan cares about most: two independent browser
// tabs converging on the same document, including through an offline/reconnect cycle.

test("creating a document opens a connected editor", async ({ page }) => {
  await page.goto("/");
  await expect(page.locator(".doc-list-page")).toBeVisible();

  await page.click(".primary-button");
  await expect(page).toHaveURL(/\/doc\//);
  await expect(page.locator("textarea.editor")).toBeVisible();
  await expect(page.locator(".connection-status--connected")).toBeVisible({ timeout: 5_000 });
});

test("two tabs sync edits live and show presence cursors", async ({ browser }) => {
  const context = await browser.newContext();
  const pageA = await context.newPage();
  await pageA.goto("/");
  await pageA.click(".primary-button");
  await expect(pageA.locator("textarea.editor")).toBeVisible();
  const docUrl = pageA.url();

  const pageB = await context.newPage();
  await pageB.goto(docUrl);
  await expect(pageB.locator(".connection-status--connected")).toBeVisible({ timeout: 5_000 });

  await pageA.click("textarea.editor");
  await pageA.type("textarea.editor", "hello from A", { delay: 20 });
  await waitForEditorText(pageB, "hello from A");

  await pageB.click("textarea.editor");
  await pageB.press("textarea.editor", "End");
  await pageB.type("textarea.editor", " and B", { delay: 20 });
  await waitForEditorText(pageA, "hello from A and B");

  await expect(pageA.locator(".presence-cursor")).toBeVisible({ timeout: 5_000 });

  await context.close();
});

test("offline edits merge correctly with no duplication on reconnect", async ({ browser }) => {
  const context = await browser.newContext();
  const pageA = await context.newPage();
  await pageA.goto("/");
  await pageA.click(".primary-button");
  await expect(pageA.locator("textarea.editor")).toBeVisible();
  const docUrl = pageA.url();

  const pageB = await context.newPage();
  await pageB.goto(docUrl);
  await expect(pageB.locator(".connection-status--connected")).toBeVisible({ timeout: 5_000 });

  await context.setOffline(true);
  await expect(pageB.locator(".connection-status--disconnected")).toBeVisible({ timeout: 5_000 });

  await pageB.click("textarea.editor");
  await pageB.press("textarea.editor", "Home");
  await pageB.type("textarea.editor", "[B-OFFLINE]", { delay: 10 });
  await expect(pageB.locator("textarea.editor")).toHaveValue(/^\[B-OFFLINE\]/);

  // A stays online and edits concurrently while B is offline.
  await pageA.click("textarea.editor");
  await pageA.press("textarea.editor", "End");
  await pageA.type("textarea.editor", "[A-ONLINE]", { delay: 10 });

  await context.setOffline(false);
  await expect(pageB.locator(".connection-status--connected")).toBeVisible({ timeout: 10_000 });

  // Let the reconnect + op exchange settle.
  await pageA.waitForTimeout(1_500);

  const finalA = await pageA.inputValue("textarea.editor");
  const finalB = await pageB.inputValue("textarea.editor");
  expect(finalA).toBe(finalB);
  expect(finalA).toContain("[B-OFFLINE]");
  expect(finalA).toContain("[A-ONLINE]");
  // The regression this guards against: re-delivering already-known ops on reconnect
  // must not duplicate content.
  expect(finalA.match(/\[B-OFFLINE\]/g)).toHaveLength(1);

  await context.close();
});

test("the document picker lists previously created documents", async ({ page }) => {
  await page.goto("/");
  await page.click(".primary-button");
  await expect(page.locator("textarea.editor")).toBeVisible();
  const docId = page.url().split("/doc/")[1];

  await page.click(".link-button"); // "<- All documents"
  await expect(page.locator(".doc-list-page")).toBeVisible();
  await expect(page.locator(`.doc-list__item:has-text("${docId.slice(0, 8)}")`)).toBeVisible();
});

test("reopening a document reconstructs its content from the server (proves durability, not just live sync)", async ({
  page,
}) => {
  await page.goto("/");
  await page.click(".primary-button");
  await expect(page.locator("textarea.editor")).toBeVisible();
  const docUrl = page.url();

  await page.click("textarea.editor");
  await page.type("textarea.editor", "durable content", { delay: 10 });
  await waitForEditorText(page, "durable content");

  // A fresh page load is a fresh WASM doc + fresh WebSocket connection with zero prior
  // state -- the only way it can end up with the right text is by correctly replaying
  // the persisted op history from Postgres via the relay server, exactly what a real
  // relay-server restart (already covered by relay-server's own integration test) would
  // also require of a reconnecting client.
  await page.goto(docUrl);
  await expect(page.locator("textarea.editor")).toBeVisible();
  await waitForEditorText(page, "durable content");
});
