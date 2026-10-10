import { expect, test } from "@playwright/test";

test("dropdown supports keyboard dismissal and outside clicks", async ({
  page,
}) => {
  await page.goto("/");
  const more = page.getByRole("button", { name: "More" });
  await more.focus();
  await page.keyboard.press("Enter");
  await expect(more).toHaveAttribute("aria-expanded", "true");
  await page.keyboard.press("Tab");
  await expect(
    page.getByRole("link", { name: "Build notes", exact: true }).first(),
  ).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(more).toHaveAttribute("aria-expanded", "false");
  await expect(more).toBeFocused();
  await more.click();
  await page.getByRole("heading", { level: 1 }).click();
  await expect(more).toHaveAttribute("aria-expanded", "false");
});

test("responsive page keeps content inside the viewport", async ({ page }, testInfo) => {
  for (const width of [320, 390, 768, 1024, 1440]) {
    await page.setViewportSize({ width, height: 900 });
    await page.goto("/");
    await page.evaluate(() => document.fonts.ready);
    expect(
      await page.evaluate(() => document.documentElement.scrollWidth),
    ).toBeLessThanOrEqual(width);
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
    await expect(page.getByRole("button", { name: "More" })).toBeInViewport();
    if (width === 390 || width === 1440) {
      await page.screenshot({ path: testInfo.outputPath(`homepage-${width}.png`), fullPage: true });
    }
    await page.getByRole("button", { name: "More" }).click();
    await expect(page.locator("#more-links")).toBeInViewport();
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
  }
});

test("logo looks up, left, right, forward, then blinks; reduced motion is respected", async ({
  page,
}) => {
  await page.goto("/");
  const transforms = await page.locator(".brand .logo-eyes").evaluate((el) => {
    const animation = el.getAnimations()[0];
    animation.pause();
    return [0, 1800, 3150, 4500, 6000, 6930].map((time) => {
      animation.currentTime = time;
      const matrix = new DOMMatrix(getComputedStyle(el).transform);
      return { x: matrix.e, y: matrix.f, scaleY: matrix.d };
    });
  });
  expect(transforms[1].y).toBeLessThan(0);
  expect(transforms[2].x).toBeLessThan(0);
  expect(transforms[3].x).toBeGreaterThan(0);
  expect(transforms[4]).toEqual({ x: 0, y: 0, scaleY: 1 });
  expect(transforms[5].scaleY).toBeLessThan(0.1);
  await page.emulateMedia({ reducedMotion: "reduce" });
  await expect(page.locator(".brand .logo-eyes")).toHaveCSS(
    "animation-name",
    "none",
  );
});
