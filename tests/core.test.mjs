import test from "node:test";
import assert from "node:assert/strict";

import {
  extractTags,
  excerptFrom,
  filterNotes,
  renderMarkdown,
  sortNotes,
} from "../ui/core.mjs";

const notes = [
  {
    id: "a",
    title: "Quiet systems",
    body: "A note about #Design and #work/notes.",
    tags: ["design", "work/notes"],
    updatedAt: 30,
    pinned: false,
    archived: false,
    trashed: false,
  },
  {
    id: "b",
    title: "Pinned idea",
    body: "Keep this close. #Ideas",
    tags: ["ideas"],
    updatedAt: 10,
    pinned: true,
    archived: false,
    trashed: false,
  },
  {
    id: "c",
    title: "Old draft",
    body: "Archived material",
    tags: [],
    updatedAt: 50,
    pinned: false,
    archived: true,
    trashed: false,
  },
  {
    id: "d",
    title: "Discarded",
    body: "Recover me",
    tags: [],
    updatedAt: 60,
    pinned: false,
    archived: false,
    trashed: true,
  },
];

test("extractTags normalizes and deduplicates nested tags", () => {
  assert.deepEqual(
    extractTags("#Design work #work/Notes and #design, but not email#hash"),
    ["design", "work/notes"],
  );
});

test("filterNotes combines collection, tag, and text query", () => {
  const result = filterNotes(notes, {
    collection: "notes",
    tag: "work/notes",
    query: "systems",
  });
  assert.deepEqual(result.map((note) => note.id), ["a"]);
});

test("filterNotes keeps trash separate from active notes", () => {
  assert.deepEqual(
    filterNotes(notes, { collection: "trash", tag: null, query: "" }).map((note) => note.id),
    ["d"],
  );
});

test("sortNotes keeps pinned notes first then orders by recency", () => {
  assert.deepEqual(sortNotes(notes.slice(0, 2)).map((note) => note.id), ["b", "a"]);
});

test("excerptFrom removes markdown and tags without returning an empty excerpt", () => {
  assert.equal(excerptFrom("## Hello\n\n**Strong** words with #tag"), "Hello Strong words with");
  assert.equal(excerptFrom("#only-tag"), "No additional text");
});

test("renderMarkdown escapes HTML and rejects unsafe link protocols", () => {
  const rendered = renderMarkdown("# Hello\n<script>alert(1)</script>\n[bad](javascript:alert(1))");
  assert.match(rendered, /<h1>Hello<\/h1>/);
  assert.doesNotMatch(rendered, /<script>/);
  assert.doesNotMatch(rendered, /href="javascript:/);
});

test("renderMarkdown supports lists, tasks, emphasis, and safe links", () => {
  const rendered = renderMarkdown(
    "- [x] shipped\n- **bold** and *calm*\n\n[site](https://example.com)",
  );
  assert.match(rendered, /type="checkbox" checked disabled/);
  assert.match(rendered, /<strong>bold<\/strong>/);
  assert.match(rendered, /<em>calm<\/em>/);
  assert.match(rendered, /href="https:\/\/example.com"/);
});

