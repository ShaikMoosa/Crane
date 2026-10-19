import { bridge } from "./bridge.mjs";
import { excerptFrom, extractTags, filterNotes, renderMarkdown } from "./core.mjs";

const $ = (selector) => document.querySelector(selector);
const ui = {
  list: $("#note-list"), emptyList: $("#empty-list"), editor: $(".editor-pane"), document: $("#editor-document"),
  editorEmpty: $("#editor-empty"), title: $("#title-input"), body: $("#body-input"), preview: $("#markdown-preview"),
  previewToggle: $("#preview-toggle"), pin: $("#pin-note"), more: $("#more-menu-button"), menu: $("#action-menu"),
  archive: $("#archive-note"), trash: $("#trash-note"), restore: $("#restore-note"), search: $("#search-input"),
  tagNav: $("#tag-nav"), activeFilter: $("#active-filter"), collectionTitle: $("#collection-title"),
  count: $("#notes-count"), date: $("#note-date"), words: $("#word-count"),
  tags: $("#note-tags"), saveStatus: $(".save-status"), saveLabel: $("#save-label"), toast: $("#toast"),
  saveError: $("#save-error"), shell: $(".app-shell"),
};

const state = { notes: [], selectedId: null, collection: "notes", tag: null, query: "", preview: false, saving: false };
let saveTimer;
let toastTimer;

function currentNote() { return state.notes.find((note) => note.id === state.selectedId) || null; }
function setMobilePane(pane) { ui.shell.dataset.mobilePane = pane; }
function updateNote(saved) {
  const index = state.notes.findIndex((note) => note.id === saved.id);
  if (index >= 0) state.notes[index] = saved; else state.notes.push(saved);
}
function formatDate(timestamp) {
  return new Intl.DateTimeFormat(undefined, { month: "short", day: "numeric", year: new Date(timestamp).getFullYear() !== new Date().getFullYear() ? "numeric" : undefined }).format(timestamp);
}
function wordCount(value) {
  const count = value.trim() ? value.trim().split(/\s+/).length : 0;
  return `${count} ${count === 1 ? "word" : "words"}`;
}
function showToast(message) {
  ui.toast.textContent = message;
  ui.toast.classList.add("is-visible");
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => ui.toast.classList.remove("is-visible"), 2200);
}
function makeIcon(id) {
  const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  const use = document.createElementNS("http://www.w3.org/2000/svg", "use");
  use.setAttribute("href", `#${id}`);
  svg.append(use);
  return svg;
}

function renderSidebar() {
  const activeNotes = state.notes.filter((note) => !note.trashed && !note.archived);
  ui.count.textContent = String(activeNotes.length);
  document.querySelectorAll("[data-collection]").forEach((button) => button.classList.toggle("is-active", button.dataset.collection === state.collection));

  const counts = new Map();
  activeNotes.forEach((note) => note.tags.forEach((tag) => counts.set(tag, (counts.get(tag) || 0) + 1)));
  ui.tagNav.replaceChildren();
  [...counts.entries()].sort(([a], [b]) => a.localeCompare(b)).forEach(([tag, count]) => {
    const button = document.createElement("button");
    button.className = `tag-item${state.tag === tag ? " is-active" : ""}`;
    button.dataset.tag = tag;
    button.setAttribute("aria-label", `Tag ${tag}, ${count} ${count === 1 ? "note" : "notes"}`);
    button.title = `#${tag}`;
    const label = document.createElement("span"); label.textContent = tag;
    const amount = document.createElement("span"); amount.textContent = String(count);
    button.append(label, amount);
    ui.tagNav.append(button);
  });
}

function renderList() {
  const visible = filterNotes(state.notes, state);
  const names = { notes: "Notes", pinned: "Pinned", archive: "Archive", trash: "Trash" };
  ui.collectionTitle.textContent = state.tag ? `#${state.tag}` : names[state.collection];
  ui.activeFilter.hidden = !state.tag;
  ui.activeFilter.textContent = state.tag ? `Filtering by #${state.tag}` : "";
  ui.list.replaceChildren();
  ui.emptyList.hidden = visible.length > 0;
  ui.list.hidden = visible.length === 0;

  visible.forEach((note) => {
    const button = document.createElement("button");
    button.className = `note-card${note.id === state.selectedId ? " is-selected" : ""}`;
    button.dataset.id = note.id;
    button.setAttribute("aria-current", note.id === state.selectedId ? "true" : "false");
    const heading = document.createElement("div"); heading.className = "note-card-title";
    if (note.pinned) heading.append(makeIcon("i-pin"));
    const title = document.createElement("span"); title.textContent = note.title || "Untitled note"; heading.append(title);
    const excerpt = document.createElement("p"); excerpt.textContent = excerptFrom(note.body);
    const meta = document.createElement("div"); meta.className = "note-card-meta";
    const date = document.createElement("span"); date.textContent = formatDate(note.updatedAt);
    meta.append(date);
    if (note.tags[0]) { const tag = document.createElement("span"); tag.className = "mini-tag"; tag.textContent = `#${note.tags[0]}`; meta.append(tag); }
    button.append(heading, excerpt, meta);
    ui.list.append(button);
  });
}

function renderEditor() {
  const note = currentNote();
  ui.editorEmpty.hidden = Boolean(note);
  ui.document.hidden = !note;
  if (!note) return;
  ui.title.value = note.title;
  ui.body.value = note.body;
  ui.date.textContent = `Edited ${formatDate(note.updatedAt)}`;
  ui.words.textContent = wordCount(note.body);
  ui.pin.classList.toggle("is-active", note.pinned);
  ui.pin.setAttribute("aria-label", note.pinned ? "Unpin current note" : "Pin current note");
  ui.pin.title = note.pinned ? "Unpin note" : "Pin note";
  ui.archive.hidden = note.trashed;
  ui.trash.hidden = note.trashed;
  ui.restore.hidden = !note.trashed && !note.archived;
  ui.archive.querySelector("span").textContent = note.archived ? "Move to notes" : "Move to archive";
  ui.tags.replaceChildren();
  note.tags.forEach((tag) => {
    const button = document.createElement("button"); button.textContent = `#${tag}`; button.dataset.tag = tag; ui.tags.append(button);
  });
  ui.preview.innerHTML = renderMarkdown(note.body);
  ui.preview.hidden = !state.preview;
  ui.body.hidden = state.preview;
  ui.previewToggle.classList.toggle("is-active", state.preview);
  ui.previewToggle.replaceChildren(makeIcon(state.preview ? "i-edit" : "i-eye"));
}

function render() { renderSidebar(); renderList(); renderEditor(); }

async function flushSave() {
  clearTimeout(saveTimer);
  const note = currentNote();
  if (!note || !state.saving) return true;
  try {
    const saved = await bridge.saveNote(note);
    updateNote(saved);
    state.saving = false;
    ui.saveStatus.classList.remove("is-saving");
    ui.saveStatus.classList.remove("is-saved-pulse");
    void ui.saveStatus.offsetWidth;
    ui.saveStatus.classList.add("is-saved-pulse");
    setTimeout(() => ui.saveStatus.classList.remove("is-saved-pulse"), 460);
    ui.saveLabel.textContent = "Saved locally";
    ui.saveError.hidden = true;
    renderSidebar(); renderList();
    return true;
  } catch (error) {
    ui.saveStatus.classList.remove("is-saving");
    ui.saveLabel.textContent = "Not saved";
    ui.saveError.hidden = false;
    ui.saveError.querySelector("span").textContent = error.message || "Save failed.";
    return false;
  }
}

function scheduleSave() {
  const note = currentNote();
  if (!note) return;
  note.title = ui.title.value;
  note.body = ui.body.value;
  note.tags = extractTags(note.body);
  state.saving = true;
  ui.saveError.hidden = true;
  ui.saveStatus.classList.add("is-saving");
  ui.saveLabel.textContent = "Saving…";
  ui.words.textContent = wordCount(note.body);
  ui.preview.innerHTML = renderMarkdown(note.body);
  clearTimeout(saveTimer);
  saveTimer = setTimeout(flushSave, 550);
}

async function selectNote(id) {
  if (!(await flushSave())) return;
  state.selectedId = id;
  state.preview = false;
  setMobilePane("editor");
  render();
  ui.body.focus();
}

async function createNote() {
  if (!(await flushSave())) return;
  try {
    const note = await bridge.createNote();
    updateNote(note);
    state.collection = "notes"; state.tag = null; state.query = ""; ui.search.value = "";
    state.selectedId = note.id;
    setMobilePane("editor");
    render();
    ui.title.select();
  } catch (error) { showToast(error.message || "Could not create a note"); }
}

async function changeState(field, value) {
  const note = currentNote();
  if (!note) return;
  if (!(await flushSave())) return;
  try {
    updateNote(await bridge.setNoteState(note.id, field, value));
    if ((field === "archived" || field === "trashed") && value) state.selectedId = null;
    ui.menu.hidden = true;
    render();
  } catch (error) { showToast(error.message || "Could not update the note"); }
}

document.querySelectorAll("[data-collection]").forEach((button) => button.addEventListener("click", async () => {
  if (!(await flushSave())) return; state.collection = button.dataset.collection; state.tag = null; state.selectedId = null; setMobilePane("list"); render();
}));
ui.tagNav.addEventListener("click", async (event) => { const button = event.target.closest("[data-tag]"); if (!button) return; if (!(await flushSave())) return; state.tag = button.dataset.tag; state.collection = "notes"; setMobilePane("list"); render(); });
ui.tags.addEventListener("click", (event) => { const button = event.target.closest("[data-tag]"); if (!button) return; state.tag = button.dataset.tag; state.collection = "notes"; render(); });
ui.list.addEventListener("click", (event) => { const button = event.target.closest("[data-id]"); if (button) selectNote(button.dataset.id); });
ui.search.addEventListener("input", () => { state.query = ui.search.value; renderList(); });
ui.title.addEventListener("input", scheduleSave); ui.body.addEventListener("input", scheduleSave);
ui.previewToggle.addEventListener("click", () => { state.preview = !state.preview; renderEditor(); });
ui.pin.addEventListener("click", () => { const note = currentNote(); if (note) changeState("pinned", !note.pinned); });
ui.more.addEventListener("click", () => { ui.menu.hidden = !ui.menu.hidden; ui.more.setAttribute("aria-expanded", String(!ui.menu.hidden)); });
ui.archive.addEventListener("click", () => { const note = currentNote(); if (note) changeState("archived", !note.archived); });
ui.trash.addEventListener("click", () => changeState("trashed", true));
ui.restore.addEventListener("click", () => { const note = currentNote(); if (!note) return; changeState(note.trashed ? "trashed" : "archived", false); });
$("#new-note").addEventListener("click", createNote); $("#empty-new-note").addEventListener("click", createNote);
$("#open-library").addEventListener("click", () => setMobilePane("library"));
$("#editor-back").addEventListener("click", async () => { if (await flushSave()) setMobilePane("list"); });
$("#retry-save").addEventListener("click", flushSave);
$("#clear-tag").addEventListener("click", () => { state.tag = null; render(); });
$("#theme-toggle").addEventListener("click", () => { const next = document.documentElement.dataset.theme === "dark" ? "light" : "dark"; document.documentElement.dataset.theme = next; localStorage.setItem("ember-notes.theme", next); });
document.addEventListener("click", (event) => { if (!event.target.closest(".menu-wrap")) { ui.menu.hidden = true; ui.more.setAttribute("aria-expanded", "false"); } });
document.addEventListener("keydown", (event) => {
  if (event.ctrlKey && event.key.toLowerCase() === "n") { event.preventDefault(); createNote(); }
  if (event.ctrlKey && event.key.toLowerCase() === "f") { event.preventDefault(); ui.search.focus(); }
  if (event.key === "Escape") { ui.menu.hidden = true; setMobilePane("list"); }
});
window.addEventListener("blur", flushSave);
document.addEventListener("visibilitychange", () => { if (document.visibilityState === "hidden") flushSave(); });

async function init() {
  document.documentElement.dataset.theme = localStorage.getItem("ember-notes.theme") || "light";
  try {
    const store = await bridge.bootstrap();
    state.notes = store.notes;
    state.selectedId = filterNotes(state.notes, state)[0]?.id || null;
    render();
  } catch (error) {
    showToast(error.message || "Ember could not open its local store");
  }
}

init();
