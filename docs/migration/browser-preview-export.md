# Exporting browser-preview notes into native Crane

The retired development browser preview stored its library in the browser's
`localStorage` under the key `crane.store.v1`. That value is a version-1
Crane store (`{ "schemaVersion": 1, "notes": [...] }`), but it is **not** a
Crane backup file. Native Crane only restores version-1 backup envelopes, so
the store must be wrapped once, by hand, using the browser's own developer
tools. Crane never scrapes browser profiles or reads preview notes
automatically; this export is user-triggered and runs entirely in the
preview page you already have open.

## Before you start

- Open the browser preview one last time and confirm your notes are there.
- This export copies note IDs, titles, bodies, tags, timestamps, and
  pin/archive/trash states exactly. Do not edit them during the export;
  altered data fails validation visibly in native Crane instead of being
  guessed at.
- The preview theme (`crane.theme`) does not migrate. Re-select your theme
  once in native Crane after the restore; this is expected and one-time.

## Export steps

1. In the preview tab, open the browser developer tools console
   (usually `F12`, then the **Console** tab).
2. Paste this snippet and press Enter. It reads your preview store, wraps
   it in a version-1 backup envelope, and downloads the result:

   ```js
   const store = JSON.parse(localStorage.getItem("crane.store.v1"));
   if (!store || store.schemaVersion !== 1 || !Array.isArray(store.notes)) {
     throw new Error("No Crane preview store found in this browser profile.");
   }
   const envelope = {
     format: "crane-backup",
     backupVersion: 1,
     createdAt: Date.now(),
     appVersion: "browser-preview",
     store,
   };
   const blob = new Blob([JSON.stringify(envelope, null, 2)], {
     type: "application/json",
   });
   const link = Object.assign(document.createElement("a"), {
     href: URL.createObjectURL(blob),
     download: "crane-preview-export.crane-backup.json",
   });
   link.click();
   URL.revokeObjectURL(link.href);
   console.log(`Exported ${store.notes.length} preview note(s).`);
   ```

3. Keep the downloaded `crane-preview-export.crane-backup.json` file until
   the native restore below has been verified after a restart.

## Restore steps (native Crane)

1. Open native Crane (0.2.0 or later).
2. Open the **Data** menu and choose **Restore Backup**.
3. Select `crane-preview-export.crane-backup.json` (the picker accepts
   Crane backup `*.json` files).
4. Check the inspection summary — backup date plus total/active/archived/
   trashed counts — against the preview. Inspection never changes data.
5. Confirm **Replace library** only when the counts match. Crane first
   writes a validated safety backup of the current native library, then
   replaces it. A stale, malformed, or oversized file is refused and the
   current library is preserved.
6. Restart native Crane and spot-check a few notes, then re-select your
   theme (see above). You can now delete the export file or keep it as an
   extra backup.

## Verification evidence

The envelope shape is the one validated by `crane_domain::inspect_backup`
(`format: "crane-backup"`, `backupVersion: 1`, millisecond `createdAt`,
`store`). Synthetic coverage — no real preview data — lives in
`crates/domain/tests/backups.rs`:

- `browser_preview_store_wrapped_in_envelope_is_accepted` wraps a synthetic
  `crane.store.v1`-shaped store (active/archived/trashed notes) and asserts
  the validator's counts.
- `browser_preview_store_without_envelope_is_rejected` asserts the raw
  preview store is refused, so the wrapping step cannot be skipped
  silently.
