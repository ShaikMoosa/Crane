import assert from "node:assert/strict";
import test from "node:test";

import { createTransferController } from "../ui/transfer-controller.mjs";

function setup(overrides = {}) {
  const events = [];
  const bridge = {
    importMarkdown: async () => ({ status: "cancelled", data: null }),
    exportNote: async () => ({ status: "cancelled", data: null }),
    exportLibrary: async () => ({ status: "cancelled", data: null }),
    createBackup: async () => ({ status: "cancelled", data: null }),
    inspectBackup: async () => ({ status: "cancelled", data: null }),
    restoreBackup: async () => ({ status: "cancelled", data: null }),
    ...overrides.bridge,
  };
  const dependencies = {
    bridge,
    flushSave: async () => {
      events.push("flush");
      return true;
    },
    selectedNoteId: () => "selected-note",
    replaceStore: (store) => events.push(["replace", store]),
    notify: (message, kind) => events.push(["notify", message, kind]),
    ...overrides,
    bridge,
  };
  return { controller: createTransferController(dependencies), events, bridge };
}

test("flushes_before_import_and_restore", async () => {
  const { controller, events, bridge } = setup();
  bridge.importMarkdown = async () => {
    events.push("import");
    return { status: "cancelled", data: null };
  };
  bridge.restoreBackup = async () => {
    events.push("restore");
    return { status: "cancelled", data: null };
  };

  await controller.importMarkdown();
  await controller.restoreBackup("token");

  assert.deepEqual(events, ["flush", "import", "flush", "restore"]);
});

test("aborts_when_pending_save_fails", async () => {
  let calls = 0;
  const { controller } = setup({
    flushSave: async () => false,
    bridge: {
      importMarkdown: async () => {
        calls += 1;
        return { status: "completed", data: {} };
      },
    },
  });

  const outcome = await controller.importMarkdown();

  assert.equal(calls, 0);
  assert.equal(outcome.status, "cancelled");
  assert.equal(controller.busy, false);
});

test("cancellation_is_silent", async () => {
  const { controller, events } = setup();

  await controller.createBackup();

  assert.equal(events.some((event) => Array.isArray(event) && event[0] === "notify"), false);
});

test("completed_import_replaces_store_and_reports_count", async () => {
  const store = { schemaVersion: 1, notes: [{ id: "imported" }] };
  const { controller, events } = setup({
    bridge: {
      importMarkdown: async () => ({
        status: "completed",
        data: { store, importedCount: 2 },
      }),
    },
  });

  await controller.importMarkdown();

  assert.deepEqual(events, [
    "flush",
    ["replace", store],
    ["notify", "2 notes imported", "success"],
  ]);
});

test("completed_restore_replaces_store", async () => {
  const store = { schemaVersion: 1, notes: [] };
  const { controller, events } = setup({
    bridge: {
      restoreBackup: async () => ({ status: "completed", data: store }),
    },
  });

  await controller.restoreBackup("valid-token");

  assert.deepEqual(events, [
    "flush",
    ["replace", store],
    ["notify", "Library restored", "success"],
  ]);
});

test("errors_unlock_controls", async () => {
  const { controller, events } = setup({
    bridge: {
      exportLibrary: async () => {
        throw new Error("disk unavailable");
      },
    },
  });

  const outcome = await controller.exportLibrary();

  assert.equal(outcome.status, "error");
  assert.equal(controller.busy, false);
  assert.deepEqual(events.at(-1), ["notify", "disk unavailable", "error"]);
});

test("duplicate_activation_while_busy_is_ignored", async () => {
  let release;
  let calls = 0;
  const pending = new Promise((resolve) => {
    release = resolve;
  });
  const { controller } = setup({
    bridge: {
      exportLibrary: async () => {
        calls += 1;
        await pending;
        return { status: "completed", data: { exportedCount: 1 } };
      },
    },
  });

  const first = controller.exportLibrary();
  assert.equal(controller.busy, true);
  const duplicate = await controller.exportLibrary();
  assert.equal(duplicate.status, "ignored");
  assert.equal(calls, 1);
  release();
  await first;
  assert.equal(controller.busy, false);
});
