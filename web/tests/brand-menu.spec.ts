import { expect, test } from "@playwright/test";

test("logo right-click opens the brand menu with three actions", async ({
  page,
  context,
}) => {
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  await page.goto("/");
  const logo = page.getByRole("link", { name: "Astrid home" });
  await logo.click({ button: "right" });
  const menu = page.getByRole("menu", { name: "Astrid brand" });
  await expect(menu).toBeInViewport();
  await expect(menu.getByRole("menuitem")).toHaveText([
    "Copy logo as SVG",
    "Go to brand guidelines",
    "Open link in new tab",
  ]);
  await expect(menu.getByRole("menuitem").first()).toBeFocused();

  await menu.getByRole("menuitem", { name: "Copy logo as SVG" }).click();
  await expect(menu.getByRole("menuitem").first()).toHaveText("Copied");
  const svg = await page.evaluate(() => navigator.clipboard.readText());
  expect(svg).toMatch(/^<svg[^>]*viewBox="0 0 20 16"/);
  await expect(menu).toBeHidden();
  await expect(logo).toBeFocused();

  await logo.click({ button: "right" });
  const popup = context.waitForEvent("page");
  await menu.getByRole("menuitem", { name: "Open link in new tab" }).click();
  expect(new URL((await popup).url()).pathname).toBe("/");
  await expect(menu).toBeHidden();
});

test("brand menu supports keyboard navigation and dismissal", async ({
  page,
}) => {
  await page.goto("/");
  const logo = page.getByRole("link", { name: "Astrid home" });
  await logo.click({ button: "right" });
  const menu = page.getByRole("menu", { name: "Astrid brand" });
  const items = menu.getByRole("menuitem");
  await page.keyboard.press("ArrowDown");
  await expect(items.nth(1)).toBeFocused();
  await page.keyboard.press("ArrowUp");
  await page.keyboard.press("ArrowUp");
  await expect(items.nth(2)).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(menu).toBeHidden();
  await expect(logo).toBeFocused();

  await logo.click({ button: "right" });
  await page.mouse.click(5, 600);
  await expect(menu).toBeHidden();

  await logo.click({ button: "right" });
  await menu.getByRole("menuitem", { name: "Go to brand guidelines" }).click();
  await expect(page).toHaveURL(/\/docs\.html#brand$/);
  await expect(page.locator(".brand-marks")).toBeVisible();
});

test("docs logo opens the same brand menu", async ({ page }) => {
  await page.goto("/docs.html");
  await page
    .getByRole("link", { name: "Astrid home" })
    .click({ button: "right" });
  const menu = page.getByRole("menu", { name: "Astrid brand" });
  await menu.getByRole("menuitem", { name: "Go to brand guidelines" }).click();
  await expect(page).toHaveURL(/\/docs\.html#brand$/);
  await expect(page.locator(".brand-marks")).toBeVisible();
});
