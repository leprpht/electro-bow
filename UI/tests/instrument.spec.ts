import { expect, test } from "@playwright/test";
import { installNativeBridge } from "./nativeBridge";

test("preview has working accessible controls and honest inactive states", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto("/");
  await expect(page.getByRole("slider")).toHaveCount(7);
  await expect(page.locator('input[type="range"]:enabled')).toHaveCount(2);
  await expect(page.getByRole("status")).toContainText("Preview");
  await expect(page.getByRole("img", { name: /Tuner idle/ })).toBeVisible();
  for (const label of ["Sustain", "Resonance", "Bow Pressure", "Bow Speed", "Bow Friction"]) {
    await expect(page.getByRole("slider", { name: label, exact: true })).toBeDisabled();
  }
  const attack = page.getByRole("slider", { name: "Attack Reduction" });
  const value = attack.locator("../..").locator("output");
  await expect(value).toHaveCSS("opacity", "0");
  await attack.hover();
  await expect(value).toHaveCSS("opacity", "1");
  await page.mouse.move(0, 0);
  await expect(value).toHaveCSS("opacity", "0");
  await attack.focus();
  await expect(value).toHaveCSS("opacity", "1");
  await expect(attack.locator("..")).toHaveCSS("outline-style", "solid");
  await page.keyboard.press("End");
  await expect(attack).toHaveValue("500");
  await page.keyboard.press("Home");
  await expect(attack).toHaveValue("0");
  await page.keyboard.press("ArrowUp");
  await expect(attack).toHaveValue("1");
  await attack.dblclick();
  await expect(attack).toHaveValue("50");
  await page.emulateMedia({ reducedMotion: "reduce" });
  await expect(value).toHaveCSS("transition-duration", "0s");
  await expect(page.locator(".tuner-needle")).toHaveCSS("transition-duration", "0s");
  expect(errors).toEqual([]);
});

test("rotary dragging supports fine movement, clamping and cancellation", async ({ page }) => {
  await page.goto("/");
  const knob = page.getByRole("slider", { name: "Bow Character", exact: true });
  const box = (await knob.boundingBox())!;
  const x = box.x + box.width / 2;
  const y = box.y + box.height / 2;
  await page.mouse.move(x, y);
  await page.mouse.down();
  await page.mouse.move(x, y - 90, { steps: 5 });
  await expect(knob).toHaveValue("0.5");
  await page.keyboard.down("Shift");
  await page.mouse.move(x, y - 180, { steps: 5 });
  await expect(knob).toHaveValue("0.55");
  await page.keyboard.up("Shift");
  await page.mouse.move(x, y - 400);
  await expect(knob).toHaveValue("1");
  await page.mouse.up();
  await knob.dblclick();
  await expect(knob).toHaveValue("0");
  await knob.evaluate((element) =>
    element.addEventListener(
      "pointerdown",
      (event) => {
        element.dataset.testPointerId = String((event as PointerEvent).pointerId);
      },
      { once: true },
    ),
  );
  await page.mouse.move(x, y);
  await page.mouse.down();
  await expect(page.locator('[data-dragging="true"]')).toHaveCount(1);
  await knob.dispatchEvent("pointercancel", {
    pointerId: Number(await knob.getAttribute("data-test-pointer-id")),
  });
  await expect(page.locator('[data-dragging="true"]')).toHaveCount(0);
  await page.mouse.up();
});

test("native values load, writes reach JUCE, and external updates resume after drag", async ({
  page,
}) => {
  await installNativeBridge(page);
  await page.goto("/");
  const knob = page.getByRole("slider", { name: "Bow Character", exact: true });
  await expect(knob).toHaveValue("0.73");
  await expect(page.getByRole("status")).toContainText("Connected");
  await knob.focus();
  await page.keyboard.press("ArrowUp");
  await expect
    .poll(() => page.evaluate(() => window.__testBridge.writes.at(-1)))
    .toEqual({ id: "bowCharacter", value: 0.731 });
  const box = (await knob.boundingBox())!;
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  await page.evaluate(() => {
    const state = structuredClone(window.__testBridge.state);
    state.parameters.bowCharacter = 0.2;
    window.__testBridge.emit(state);
  });
  await expect(knob).toHaveValue("0.731");
  await page.mouse.up();
  await page.evaluate(() => window.__testBridge.emit(window.__testBridge.state));
  await expect(knob).toHaveValue("0.2");
  const attack = page.getByRole("slider", { name: "Attack Reduction" });
  await attack.focus();
  await page.keyboard.press("End");
  await expect
    .poll(() => page.evaluate(() => window.__testBridge.writes.at(-1)))
    .toEqual({ id: "attackMs", value: 500 });
});

test("tuner responds to measured pitch and returns to idle without stale telemetry", async ({
  page,
}) => {
  await installNativeBridge(page);
  await page.goto("/");
  await expect(page.getByRole("status")).toContainText("Connected");
  for (const cents of [-25, 0, 25]) {
    await page.evaluate((cents) => {
      const state = structuredClone(window.__testBridge.state);
      state.pitch = { note: "A4", frequencyHz: 440 * 2 ** (cents / 1200), confidence: 0.95 };
      window.__testBridge.emit(state);
    }, cents);
    await expect(page.getByRole("img")).toHaveAttribute("aria-label", new RegExp(`${cents} cents`));
    await expect
      .poll(() => page.locator(".tuner-needle").evaluate((el) => el.getAttribute("style")))
      .toContain(`rotate(${cents * 0.96}`);
  }
  await page.evaluate(() => {
    const state = structuredClone(window.__testBridge.state);
    state.pitch = { note: "—", frequencyHz: 0, confidence: 0 };
    window.__testBridge.emit(state);
  });
  await expect(page.getByRole("img", { name: /Tuner idle/ })).toBeVisible();
  await expect(page.locator(".tuner-frequency")).toHaveText("No signal");
});

for (const size of [
  { width: 960, height: 700 },
  { width: 760, height: 540 },
  { width: 1280, height: 960 },
  { width: 390, height: 844 },
  { width: 320, height: 568 },
]) {
  test(`offline assets and unclipped panel at ${size.width} × ${size.height}`, async ({
    page,
  }, testInfo) => {
    await page.setViewportSize(size);
    await page.goto("/");
    await page.evaluate(() => document.fonts.ready);
    expect(
      await page.evaluate(
        () =>
          document.fonts.check('20px "Lobster"') && document.fonts.check('20px "Barlow Condensed"'),
      ),
    ).toBe(true);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
    const shell = (await page.locator(".app-shell").boundingBox())!;
    expect(shell.y + shell.height).toBeLessThanOrEqual(size.height);
    expect(await page.evaluate(() => document.documentElement.scrollHeight <= innerHeight)).toBe(
      true,
    );
    for (const control of await page.getByRole("slider").all()) {
      const box = (await control.boundingBox())!;
      expect(box.x).toBeGreaterThanOrEqual(shell.x);
      expect(box.x + box.width).toBeLessThanOrEqual(shell.x + shell.width);
      expect(box.y + box.height).toBeLessThanOrEqual(shell.y + shell.height);
      expect(box.width).toBeGreaterThanOrEqual(44);
    }
    await page.screenshot({ path: testInfo.outputPath("instrument.png") });
    const requests: string[] = [];
    page.on("request", (request) => requests.push(request.url()));
    await page.reload();
    await page.evaluate(() => document.fonts.ready);
    expect(
      requests.filter(
        (url) => !url.startsWith("http://127.0.0.1:4173/") && !url.startsWith("data:"),
      ),
    ).toEqual([]);
  });
}
