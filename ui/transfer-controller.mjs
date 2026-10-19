const CANCELLED = Object.freeze({ status: "cancelled", data: null });
const IGNORED = Object.freeze({ status: "ignored", data: null });

export function createTransferController({
  bridge,
  flushSave,
  selectedNoteId,
  replaceStore,
  notify,
}) {
  let busy = false;

  async function run(action, { completed, success } = {}) {
    if (busy) return IGNORED;
    busy = true;
    try {
      if (!(await flushSave())) return CANCELLED;
      const outcome = await action();
      if (outcome?.status !== "completed") return outcome || CANCELLED;
      completed?.(outcome.data);
      if (success) notify(typeof success === "function" ? success(outcome.data) : success, "success");
      return outcome;
    } catch (error) {
      notify(error?.message || "The transfer could not be completed.", "error");
      return { status: "error", data: null, error };
    } finally {
      busy = false;
    }
  }

  return Object.freeze({
    get busy() {
      return busy;
    },

    importMarkdown() {
      return run(() => bridge.importMarkdown(), {
        completed: ({ store }) => replaceStore(store),
        success: ({ importedCount }) =>
          `${importedCount} ${importedCount === 1 ? "note" : "notes"} imported`,
      });
    },

    exportNote() {
      return run(() => {
        const id = selectedNoteId();
        return id ? bridge.exportNote(id) : CANCELLED;
      }, { success: "Note exported" });
    },

    exportLibrary() {
      return run(() => bridge.exportLibrary(), { success: "Library exported" });
    },

    createBackup() {
      return run(() => bridge.createBackup(), { success: "Backup created" });
    },

    inspectBackup() {
      return run(() => bridge.inspectBackup());
    },

    restoreBackup(token) {
      return run(() => bridge.restoreBackup(token), {
        completed: (store) => replaceStore(store),
        success: "Library restored",
      });
    },
  });
}
