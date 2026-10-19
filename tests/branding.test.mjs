import test from "node:test";
import assert from "node:assert/strict";

class MemoryStorage {
  #values = new Map();

  getItem(key) {
    return this.#values.has(key) ? this.#values.get(key) : null;
  }

  setItem(key, value) {
    this.#values.set(key, String(value));
  }
}

test("browser preview persists Crane data under the Crane namespace", async () => {
  globalThis.window = {};
  globalThis.localStorage = new MemoryStorage();

  const { bridge } = await import("../ui/bridge.mjs");
  const store = await bridge.bootstrap();

  assert.equal(store.schemaVersion, 1);
  assert.ok(localStorage.getItem("crane.store.v1"));
  assert.equal(localStorage.getItem("ember-notes.store.v1"), null);
});
