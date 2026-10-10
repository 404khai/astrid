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

test("responsive page keeps content inside the viewport", async ({
  page,
}, testInfo) => {
  for (const width of [320, 390, 768, 1024, 1440]) {
    await page.setViewportSize({ width, height: 900 });
    await page.goto("/");
    await page.evaluate(() => document.fonts.ready);
    expect(
      await page.evaluate(() => document.documentElement.scrollWidth),
    ).toBeLessThanOrEqual(width);
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
    await expect(
      page.getByRole("button", { name: width <= 650 ? "Menu" : "More" }),
    ).toBeInViewport();
    if (width === 390 || width === 1440) {
      await page.screenshot({
        path: testInfo.outputPath(`homepage-${width}.png`),
        fullPage: true,
      });
    }
    await page
      .getByRole("button", { name: width <= 650 ? "Menu" : "More" })
      .click();
    await expect(
      page.locator(width <= 650 ? "#mobile-links" : "#more-links"),
    ).toBeInViewport();
    expect(
      await page.evaluate(() => document.documentElement.scrollWidth),
    ).toBeLessThanOrEqual(width);
  }
});

test("logo looks up, left, right, forward, then blinks; reduced motion is respected", async ({
  page,
}) => {
  await page.goto("/");
  const transforms = await page.locator(".brand .logo-gaze").evaluate((el) => {
    const animation = el.getAnimations()[0];
    animation.pause();
    return [0, 1800, 3150, 4500, 6000].map((time) => {
      animation.currentTime = time;
      const matrix = new DOMMatrix(getComputedStyle(el).transform);
      return { x: matrix.e, y: matrix.f, scaleY: matrix.d };
    });
  });
  expect(transforms[1].y).toBeLessThan(0);
  expect(transforms[2].x).toBeLessThan(0);
  expect(transforms[3].x).toBeGreaterThan(0);
  expect(transforms[4]).toEqual({ x: 0, y: 0, scaleY: 1 });
  const blinks = await page.locator(".brand .logo-eyes").evaluate((el) => {
    const animation = el.getAnimations()[0];
    animation.pause();
    return [6930, 7200, 7650, 7920].map((time) => {
      animation.currentTime = time;
      return new DOMMatrix(getComputedStyle(el).transform).d;
    });
  });
  expect(blinks[0]).toBeLessThan(0.1);
  expect(blinks[1]).toBe(1);
  expect(blinks[2]).toBeLessThan(0.1);
  expect(blinks[3]).toBe(1);
  await page.emulateMedia({ reducedMotion: "reduce" });
  await expect(page.locator(".brand .logo-eyes")).toHaveCSS(
    "animation-name",
    "none",
  );
});

test("sticky navbar and cursor tracking stay functional after scrolling", async ({
  page,
}) => {
  await page.goto("/");
  await expect(page.getByRole("link", { name: "Astrid home" })).toBeVisible();
  await page.mouse.move(10, 10, { steps: 3 });
  await expect
    .poll(async () =>
      page
        .locator(".brand .logo-gaze")
        .evaluate((el) => new DOMMatrix(getComputedStyle(el).transform).e),
    )
    .toBeLessThan(0);
  await page.mouse.move(1200, 800);
  await expect
    .poll(async () =>
      page
        .locator(".brand .logo-gaze")
        .evaluate((el) => new DOMMatrix(getComputedStyle(el).transform).e),
    )
    .toBeGreaterThan(0);
  await page.evaluate(() => window.scrollTo(0, 1200));
  await expect
    .poll(async () => (await page.locator(".navbar").boundingBox())?.y)
    .toBe(0);
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.mouse.move(10, 10);
  await expect(page.locator(".brand .logo-gaze")).not.toHaveClass(/tracking/);
});

test("mobile Menu dismisses with Escape and closes after navigation", async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 900 });
  await page.goto("/");
  const menu = page.getByRole("button", { name: "Menu" });
  await expect(page.getByRole("button", { name: "More" })).toBeHidden();
  await menu.click();
  await page.keyboard.press("Escape");
  await expect(menu).toBeFocused();
  await expect(menu).toHaveAttribute("aria-expanded", "false");
  await menu.click();
  await page
    .locator("#mobile-links")
    .getByRole("link", { name: "Philosophy" })
    .click();
  await expect(menu).toHaveAttribute("aria-expanded", "false");
  await expect(page).toHaveURL(/#philosophy$/);
});
