import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const root = new URL("../", import.meta.url);
const html = await readFile(new URL("ui/index.html", root), "utf8");
const css = await readFile(new URL("ui/styles.css", root), "utf8");
const app = await readFile(new URL("ui/app.js", root), "utf8");

test("Data menu lives in the library footer and uses real buttons", () => {
  const footer = html.match(/<div class="library-footer">([\s\S]*?)<\/div>\s*<\/aside>/)?.[1] || "";
  assert.match(footer, /id="data-menu-button"/);
  assert.match(footer, /id="data-menu"/);
  for (const id of ["import-markdown", "export-library", "create-backup", "inspect-backup"]) {
    assert.match(footer, new RegExp(`<button[^>]*id="${id}"`));
  }
  assert.match(html, /<button[^>]*id="export-note"/);
});

test("Data trigger exposes expanded state and controlled menu", () => {
  const trigger = html.match(/<button[^>]*id="data-menu-button"[^>]*>/)?.[0] || "";
  assert.match(trigger, /aria-expanded="false"/);
  assert.match(trigger, /aria-controls="data-menu"/);
});

test("restore confirmation is a labelled semantic dialog", () => {
  const dialog = html.match(/<dialog[^>]*id="restore-dialog"[^>]*>/)?.[0] || "";
  assert.match(dialog, /aria-labelledby="restore-dialog-title"/);
  assert.match(dialog, /aria-describedby="restore-dialog-description"/);
  assert.match(html, /id="restore-dialog-title"/);
  assert.match(html, /id="restore-dialog-description"/);
  assert.match(html, /<button[^>]*id="confirm-restore"[^>]*class="[^"]*danger[^"]*"/);
  assert.match(html, /<button[^>]*id="cancel-restore"/);
});

test("compact transfer controls preserve 44 pixel targets", () => {
  const compact = css.match(/@media \(max-width: 680px\) \{([\s\S]*?)\n\}/)?.[1] || "";
  assert.match(compact, /(?:\.data-menu button|\.data-menu-button)[^{]*\{[^}]*min-height:\s*44px/);
});

test("busy transfers block note mutations until their store snapshot completes", () => {
  assert.match(app, /ui\.shell\.inert\s*=\s*transfers\.busy/);
});

test("restore replacement cannot be dismissed after commit starts", () => {
  assert.match(app, /ui\.cancelRestore\.disabled\s*=\s*transfers\.busy/);
  assert.match(app, /restoreDialog\.addEventListener\("cancel",[\s\S]*?if \(transfers\.busy\) event\.preventDefault\(\)/);
});

test("closing restore confirmation revokes its one-use token", () => {
  assert.match(app, /restoreDialog\.returnValue\s*=\s*""/);
  assert.match(app, /bridge\.cancelRestore\(token\)/);
});
