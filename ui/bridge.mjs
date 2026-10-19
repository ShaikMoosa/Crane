const STORAGE_KEY = "ember-notes.store.v1";

const now = Date.now();
const seedNotes = [
  {
    id: "welcome-to-ember",
    title: "Welcome to Ember",
    body: "# A quiet place for bright ideas\n\nEmber keeps your notes on this computer. Write in Markdown, use #tags to connect ideas, and let the interface stay out of your way.\n\n- [x] Create a first note\n- [ ] Shape it into something useful\n- [ ] Keep the spark",
    tags: ["tags"],
    createdAt: now - 172800000,
    updatedAt: now - 3600000,
    pinned: true,
    archived: false,
    trashed: false,
  },
  {
    id: "weekend-reading",
    title: "Weekend reading",
    body: "## On the desk\n\n- The Creative Act\n- A Pattern Language\n- Ways of Seeing\n\nNotes belong under #reading/list.",
    tags: ["reading/list"],
    createdAt: now - 259200000,
    updatedAt: now - 86400000,
    pinned: false,
    archived: false,
    trashed: false,
  },
  {
    id: "product-ritual",
    title: "A small product ritual",
    body: "Before adding a feature, write down the behavior in plain language. Then make the smallest test fail. #work/process\n\n> Clarity is a form of speed.",
    tags: ["work/process"],
    createdAt: now - 432000000,
    updatedAt: now - 172800000,
    pinned: false,
    archived: false,
    trashed: false,
  },
];

function isNative() {
  return Boolean(window.__TAURI__?.core?.invoke);
}

function loadLocal() {
  try {
    const stored = JSON.parse(localStorage.getItem(STORAGE_KEY));
    if (stored?.schemaVersion === 1 && Array.isArray(stored.notes)) return stored;
  } catch {
    // A corrupt preview store is replaced with a safe seed below.
  }
  const store = { schemaVersion: 1, notes: seedNotes };
  localStorage.setItem(STORAGE_KEY, JSON.stringify(store));
  return structuredClone(store);
}

function saveLocal(store) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(store));
}

function makeId() {
  return globalThis.crypto?.randomUUID?.() || `note-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

export const bridge = {
  async bootstrap() {
    if (isNative()) return window.__TAURI__.core.invoke("bootstrap");
    return loadLocal();
  },

  async createNote() {
    if (isNative()) return window.__TAURI__.core.invoke("create_note");
    const store = loadLocal();
    const timestamp = Date.now();
    const note = {
      id: makeId(),
      title: "Untitled note",
      body: "",
      tags: [],
      createdAt: timestamp,
      updatedAt: timestamp,
      pinned: false,
      archived: false,
      trashed: false,
    };
    store.notes.push(note);
    saveLocal(store);
    return structuredClone(note);
  },

  async saveNote(note) {
    if (isNative()) return window.__TAURI__.core.invoke("save_note", { note });
    const store = loadLocal();
    const index = store.notes.findIndex((candidate) => candidate.id === note.id);
    if (index < 0) throw new Error("That note no longer exists.");
    const saved = { ...note, updatedAt: Date.now() };
    store.notes[index] = saved;
    saveLocal(store);
    return structuredClone(saved);
  },

  async setNoteState(id, field, value) {
    if (isNative()) return window.__TAURI__.core.invoke("set_note_state", { id, field, value });
    const store = loadLocal();
    const index = store.notes.findIndex((candidate) => candidate.id === id);
    if (index < 0) throw new Error("That note no longer exists.");
    if (!["pinned", "archived", "trashed"].includes(field)) throw new Error("Unsupported note state.");
    store.notes[index] = { ...store.notes[index], [field]: Boolean(value), updatedAt: Date.now() };
    saveLocal(store);
    return structuredClone(store.notes[index]);
  },
};

