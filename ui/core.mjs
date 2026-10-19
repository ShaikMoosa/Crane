const TAG_PATTERN = /(^|\s)#([\p{L}\p{N}_-]+(?:\/[\p{L}\p{N}_-]+)*)/gu;

export function extractTags(source = "") {
  const tags = [];
  const seen = new Set();
  for (const match of source.matchAll(TAG_PATTERN)) {
    const tag = match[2].toLocaleLowerCase();
    if (!seen.has(tag)) {
      seen.add(tag);
      tags.push(tag);
    }
  }
  return tags;
}

export function sortNotes(notes) {
  return [...notes].sort((a, b) => {
    if (Boolean(a.pinned) !== Boolean(b.pinned)) return a.pinned ? -1 : 1;
    return Number(b.updatedAt || 0) - Number(a.updatedAt || 0);
  });
}

export function filterNotes(notes, filters = {}) {
  const collection = filters.collection || "notes";
  const tag = filters.tag?.toLocaleLowerCase() || null;
  const query = filters.query?.trim().toLocaleLowerCase() || "";

  const visible = notes.filter((note) => {
    if (collection === "trash") return note.trashed;
    if (note.trashed) return false;
    if (collection === "archive" && !note.archived) return false;
    if (collection === "notes" && note.archived) return false;
    if (collection === "pinned" && (!note.pinned || note.archived)) return false;
    if (tag && !note.tags?.some((value) => value.toLocaleLowerCase() === tag)) return false;
    if (!query) return true;
    const haystack = `${note.title || ""}\n${note.body || ""}\n${(note.tags || []).join(" ")}`.toLocaleLowerCase();
    return haystack.includes(query);
  });

  return sortNotes(visible);
}

export function excerptFrom(markdown = "", maxLength = 116) {
  const text = markdown
    .replace(/```[\s\S]*?```/g, " ")
    .replace(/!\[[^\]]*]\([^)]*\)/g, " ")
    .replace(/\[([^\]]+)]\([^)]*\)/g, "$1")
    .replace(/(^|\s)#[\p{L}\p{N}_-]+(?:\/[\p{L}\p{N}_-]+)*/gu, "$1")
    .replace(/^\s{0,3}#{1,6}\s+/gm, "")
    .replace(/^\s*[-*>+]\s+/gm, "")
    .replace(/[\`*_~]/g, "")
    .replace(/\s+/g, " ")
    .trim();

  if (!text) return "No additional text";
  return text.length > maxLength ? `${text.slice(0, maxLength - 1).trimEnd()}…` : text;
}

function escapeHtml(value) {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");
}

function inlineMarkdown(value) {
  let text = escapeHtml(value);
  text = text.replace(/`([^`]+)`/g, "<code>$1</code>");
  text = text.replace(/\[([^\]]+)]\((https?:\/\/[^\s)]+)\)/g, '<a href="$2" target="_blank" rel="noreferrer">$1</a>');
  text = text.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
  text = text.replace(/(^|[^*])\*([^*\n]+)\*/g, "$1<em>$2</em>");
  return text;
}

export function renderMarkdown(markdown = "") {
  const lines = String(markdown).replace(/\r\n?/g, "\n").split("\n");
  const html = [];
  let listOpen = false;
  let codeOpen = false;

  const closeList = () => {
    if (listOpen) {
      html.push("</ul>");
      listOpen = false;
    }
  };

  for (const line of lines) {
    if (/^```/.test(line)) {
      closeList();
      html.push(codeOpen ? "</code></pre>" : "<pre><code>");
      codeOpen = !codeOpen;
      continue;
    }
    if (codeOpen) {
      html.push(`${escapeHtml(line)}\n`);
      continue;
    }

    const heading = /^(#{1,6})\s+(.+)$/.exec(line);
    if (heading) {
      closeList();
      const level = heading[1].length;
      html.push(`<h${level}>${inlineMarkdown(heading[2])}</h${level}>`);
      continue;
    }

    const task = /^\s*[-*+]\s+\[([ xX])]\s+(.+)$/.exec(line);
    const item = /^\s*[-*+]\s+(.+)$/.exec(line);
    if (task || item) {
      if (!listOpen) {
        html.push("<ul>");
        listOpen = true;
      }
      if (task) {
        const checked = task[1].toLowerCase() === "x" ? " checked" : "";
        html.push(`<li class="task"><input type="checkbox"${checked} disabled><span>${inlineMarkdown(task[2])}</span></li>`);
      } else {
        html.push(`<li>${inlineMarkdown(item[1])}</li>`);
      }
      continue;
    }

    closeList();
    if (!line.trim()) continue;
    if (/^>\s?/.test(line)) {
      html.push(`<blockquote>${inlineMarkdown(line.replace(/^>\s?/, ""))}</blockquote>`);
    } else {
      html.push(`<p>${inlineMarkdown(line)}</p>`);
    }
  }

  closeList();
  if (codeOpen) html.push("</code></pre>");
  return html.join("\n");
}
