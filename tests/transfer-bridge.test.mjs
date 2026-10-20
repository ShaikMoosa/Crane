import assert from "node:assert/strict";
import test from "node:test";

async function loadBridge({ invoke, adapter } = {}) {
  globalThis.window = invoke ? { __TAURI__: { core: { invoke } } } : {};
  if (adapter) globalThis.__CRANE_PREVIEW_TRANSFER__ = adapter;
  else delete globalThis.__CRANE_PREVIEW_TRANSFER__;
  return (await import(`../ui/bridge.mjs?transfer-test=${Date.now()}-${Math.random()}`)).bridge;
}

test.afterEach(() => {
  delete globalThis.window;
  delete globalThis.__CRANE_PREVIEW_TRANSFER__;
});

test("native transfer methods dispatch narrow command arguments", async () => {
  const calls = [];
  const bridge = await loadBridge({
    invoke: async (command, payload) => {
      calls.push([command, payload]);
      return { status: "completed", data: command };
    },
  });

  await bridge.importMarkdown();
  await bridge.exportNote("note-7");
  await bridge.exportLibrary();
  await bridge.createBackup();
  await bridge.inspectBackup();
  await bridge.restoreBackup("restore-token");
  await bridge.cancelRestore("cancel-token");

  assert.deepEqual(calls, [
    ["import_markdown", undefined],
    ["export_note", { id: "note-7" }],
    ["export_library", undefined],
    ["create_backup", undefined],
    ["inspect_backup", undefined],
    ["restore_backup", { confirmationToken: "restore-token" }],
    ["cancel_restore", { confirmationToken: "cancel-token" }],
  ]);
});

test("preview transfer methods delegate to the injected adapter", async () => {
  const calls = [];
  const adapter = Object.fromEntries(
    ["importMarkdown", "exportNote", "exportLibrary", "createBackup", "inspectBackup", "restoreBackup", "cancelRestore"].map(
      (name) => [
        name,
        async (...args) => {
          calls.push([name, ...args]);
          return { status: "completed", data: name };
        },
      ],
    ),
  );
  const bridge = await loadBridge({ adapter });

  await bridge.importMarkdown();
  await bridge.exportNote("note-3");
  await bridge.exportLibrary();
  await bridge.createBackup();
  await bridge.inspectBackup();
  await bridge.restoreBackup("token-3");
  await bridge.cancelRestore("token-4");

  assert.deepEqual(calls, [
    ["importMarkdown"],
    ["exportNote", "note-3"],
    ["exportLibrary"],
    ["createBackup"],
    ["inspectBackup"],
    ["restoreBackup", "token-3"],
    ["cancelRestore", "token-4"],
  ]);
});

test("missing preview adapter returns a normalized cancelled outcome", async () => {
  const bridge = await loadBridge();

  assert.deepEqual(await bridge.importMarkdown(), { status: "cancelled", data: null });
  assert.deepEqual(await bridge.createBackup(), { status: "cancelled", data: null });
});

test("preview adapter failures are not hidden", async () => {
  const bridge = await loadBridge({
    adapter: {
      async importMarkdown() {
        throw new Error("preview import failed");
      },
    },
  });

  await assert.rejects(bridge.importMarkdown(), /preview import failed/);
});
