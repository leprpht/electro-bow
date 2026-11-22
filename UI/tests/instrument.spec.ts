import { expect, test } from "@playwright/test";
import { installNativeBridge } from "./nativeBridge";

test("preview has six accessible working controls", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto("/");
  await expect(page.getByRole("slider")).toHaveCount(6);
  await expect(page.locator('input[type="range"]:enabled')).toHaveCount(6);
  await expect(page.getByRole("status")).toContainText("Preview");
  await expect(page.getByRole("img", { name: "Electro-Bow by Unholy People" })).toBeVisible();
  for (const label of [
    "Attack",
    "Bow Character",
    "Out",
    "Noise",
    "Saturation",
    "Spectral Movement",
  ]) {
    await expect(page.getByRole("slider", { name: label, exact: true })).toBeEnabled();
  }
  const attack = page.getByRole("slider", { name: "Attack" });
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
  expect(errors).toEqual([]);
});

test("rotary dragging supports fine movement, clamping and cancellation", async ({ page }) => {
  await page.goto("/");
  const knob = page.getByRole("slider", { name: "Bow Character", exact: true });
  await knob.focus();
  await page.keyboard.press("Home");
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
  await expect(knob).toHaveValue("0.5");
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
  const attack = page.getByRole("slider", { name: "Attack" });
  await attack.focus();
  await page.keyboard.press("End");
  await expect
    .poll(() => page.evaluate(() => window.__testBridge.writes.at(-1)))
    .toEqual({ id: "attackMs", value: 500 });
});

test("Attack drag follows the printed sweep and brackets one host automation gesture", async ({
  page,
}) => {
  await installNativeBridge(page);
  await page.goto("/");
  const attack = page.getByRole("slider", { name: "Attack", exact: true });
  await expect(page.getByRole("status")).toHaveText("Connected");
  await attack.press("Home");
  const box = (await attack.boundingBox())!;
  const x = box.x + box.width / 2;
  const y = box.y + box.height / 2;
  await page.mouse.move(x, y);
  await page.mouse.down();
  await page.mouse.move(x, y - 90, { steps: 3 });
  await expect(attack).toHaveValue("50");
  await page.mouse.move(x, y - 180, { steps: 3 });
  await expect(attack).toHaveValue("500");
  await page.mouse.up();
  await expect
    .poll(() => page.evaluate(() => window.__testBridge.gestures))
    .toEqual([
      { id: "attackMs", active: true },
      { id: "attackMs", active: false },
    ]);
});

test("each faceplate dial reads host automation and sends its own physical value", async ({
  page,
}) => {
  await installNativeBridge(page);
  await page.goto("/");
  await expect(page.getByRole("status")).toHaveText("Connected");
  for (const [label, id, nativeValue, max] of [
    ["Attack", "attackMs", 125, 500],
    ["Bow Character", "bowCharacter", 0.25, 1],
    ["Out", "out", 0.25, 1],
    ["Noise", "noise", 0.25, 1],
    ["Saturation", "saturation", 0.25, 1],
    ["Spectral Movement", "spectralMovement", 0.25, 1],
  ] as const) {
    const slider = page.getByRole("slider", { name: label, exact: true });
    await page.evaluate(
      ([id, value]) => {
        const state = structuredClone(window.__testBridge.state);
        state.parameters[id] = value;
        window.__testBridge.emit(state);
      },
      [id, nativeValue] as const,
    );
    await expect(slider).toHaveValue(String(nativeValue));
    await slider.press("End");
    await expect
      .poll(() => page.evaluate(() => window.__testBridge.writes.at(-1)))
      .toEqual({ id, value: max });
    if (id === "out") {
      await expect(slider).toHaveAttribute("aria-valuetext", "6.0 dB");
      await slider.press("Home");
      await expect(slider).toHaveAttribute("aria-valuetext", "−∞ dB");
    }
  }
});

test("dial faces register to the artwork at noon and rotate to the printed endpoint", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1280, height: 853 });
  await page.goto("/");
  const artwork = page.getByRole("img");
  await artwork.evaluate((image: HTMLImageElement) => image.decode());
  const screenshot = () => page.screenshot().then((bytes) => bytes.toString("base64"));
  const resting = await screenshot();
  for (const control of await page.getByRole("slider").all()) await control.press("End");
  await page.getByRole("slider").last().blur();
  const turned = await screenshot();
  const readings = await artwork.evaluate(
    async (ref: HTMLImageElement, { resting, turned }) => {
      async function pixels(encoded: string) {
        const image = new Image();
        image.src = "data:image/png;base64," + encoded;
        await image.decode();
        return raster(image);
      }
      function raster(image: HTMLImageElement) {
        const canvas = document.createElement("canvas");
        canvas.width = 1280;
        canvas.height = 853;
        const ctx = canvas.getContext("2d")!;
        ctx.drawImage(image, 0, 0);
        return ctx.getImageData(0, 0, 1280, 853).data;
      }
      const original = raster(ref);
      const initial = await pixels(resting);
      const edited = await pixels(turned);
      return [
        [235, 323, 49],
        [649, 324, 49],
        [1053, 323, 49],
        [424, 582, 44],
        [652, 582, 44],
        [880, 582, 44],
      ].map(([x, y, r]) => {
        // Compare the face to the supplied photograph, then sample the actual brass pointer.
        let error = 0;
        for (let row = y - 55; row < y + 55; row++) {
          for (let col = x - 55; col < x + 55; col++) {
            const i = 4 * (row * 1280 + col);
            error += Math.abs(original[i] - initial[i]);
          }
        }
        const angle = (127 * Math.PI) / 180;
        const tip =
          4 * (Math.round(y - r * Math.cos(angle)) * 1280 + Math.round(x + r * Math.sin(angle)));
        const noon = 4 * ((y - r) * 1280 + x);
        return {
          meanError: error / (110 * 110),
          newPointer: edited[tip],
          oldPointer: edited[noon],
        };
      });
    },
    { resting, turned },
  );
  for (const reading of readings) {
    expect(reading.meanError).toBeLessThan(3);
    expect(reading.newPointer).toBeGreaterThan(165);
    expect(reading.oldPointer).toBeLessThan(140);
  }
});

for (const size of [
  { width: 960, height: 640 },
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
          document.fonts.check('20px "Barlow Condensed"') &&
          document.querySelector("img")?.naturalWidth === 1280,
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
