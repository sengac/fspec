# BUG-151: `add-attachment` truncates the source file to 0 bytes when it already lives in `spec/attachments/<ID>/`

## Summary

When `fspec add-attachment <ID> <filePath>` is called with a `filePath` that already resides in
the destination directory `spec/attachments/<workUnitId>/`, the command copies the file **onto
itself** and truncates it to **0 bytes**. This is a common, natural workflow — write the
research/markdown doc into the work unit's attachments directory first, then register it — and
it silently destroys the document.

**Severity: data loss.** Reproduced twice on 2026-07-10:

1. **~14:06** — CONT-006 / CONT-007 / CONT-008 / CONT-009 research attachments (four markdown
   docs) truncated to 0 bytes during epic card preparation.
2. **~22:0x** — `spec/attachments/CONT-005/review-findings.md` truncated during the review-cycle
   wrap-up.

All five files had to be manually reconstructed and restored via the `Write` tool.

## Root cause

File: `src/commands/add-attachment.ts`

```ts
// line 64-70
const attachmentsDir = join(cwd, 'spec', 'attachments', options.workUnitId);
await mkdir(attachmentsDir, { recursive: true });

const fileName = basename(resolvedPath);
const destPath = join(attachmentsDir, fileName);
await copyFile(resolvedPath, destPath);        // ← no source === dest guard
```

When the source already lives in `spec/attachments/<workUnitId>/`,
`destPath === resolvedPath` (same inode). Node's `fs.promises.copyFile` (libuv
`uv_fs_copyfile`) opens the **destination** with `O_CREAT | O_WRONLY | O_TRUNC` — truncating
the file — and then reads from the (now empty) source. Result: the file is destroyed before a
single byte is copied. No error is raised; the command reports success.

### Compounding order-of-operations bug

The duplicate-registration check runs **after** the copy (line ~92):

```ts
await copyFile(resolvedPath, destPath);              // file already truncated here...
...
if (workUnit.attachments.includes(relativePath)) {
  throw new Error(`Attachment '${fileName}' already exists ...`);  // ...then we throw
}
```

So re-registering an existing attachment destroys the file **and** exits with an error,
leaving the work unit pointing at a 0-byte attachment.

### Prior art: BUG-055

Lines 72–81 already special-case a source in the `spec/attachments/` **root** (delete source
after copy to prevent duplication). That guard compares `sourceDir === attachmentsRootDir` only
and never considers the per-work-unit directory — the exact case that self-truncates was left
unhandled.

## Reproduction

```bash
mkdir -p spec/attachments/BUG-151
printf 'important research\n' > spec/attachments/BUG-151/notes.md
fspec add-attachment BUG-151 spec/attachments/BUG-151/notes.md
wc -c spec/attachments/BUG-151/notes.md   # → 0 bytes; command reported success
```

## Proposed fix

In `addAttachment()` (`src/commands/add-attachment.ts`):

1. **Guard source === destination (primary fix).** Canonicalize both paths
   (`resolve()`, plus `realpath` to defeat symlink aliasing) and compare:

   ```ts
   const destPath = join(attachmentsDir, fileName);
   if (resolve(resolvedPath) !== resolve(destPath)) {
     await copyFile(resolvedPath, destPath);
   }
   // else: file is already in place — register-only, no copy, no unlink
   ```

2. **Reorder the duplicate-registration check before any filesystem mutation** so a duplicate
   `add-attachment` call throws without touching the file.

3. **Defense in depth (optional):** copy via a temp file in the destination directory and
   `rename()` into place, or pass `fs.constants.COPYFILE_EXCL` when the destination must not
   pre-exist. Either makes an accidental self-copy impossible even if the guard regresses.

4. Ensure the BUG-055 root-directory unlink (lines 72–81) is **not** triggered in the
   register-only path (source inside the per-work-unit dir must never be unlinked).

## Regression tests required

Add to `src/commands/__tests__/` (Vitest, TypeScript; patterns exist in
`attachment-support.test.ts` and `bug-055-attachment-duplication.test.ts`):

1. **Self-copy preserves content:** create `spec/attachments/<ID>/doc.md` with known content,
   call `addAttachment`, assert (a) file content is byte-identical afterwards, (b) the
   attachment path is registered on the work unit, (c) command succeeds.
2. **Duplicate registration is non-destructive:** register a file, call `addAttachment` again
   with the same path, assert it throws `already exists` **and** the file content is intact.
3. **BUG-055 behavior preserved:** source in `spec/attachments/` root is still copied into the
   per-work-unit dir and the root copy removed.
4. **Symlink alias (if realpath used):** a symlink pointing at the destination file must not
   truncate it.

## Workaround (until fixed)

Write the source document **outside** `spec/attachments/<ID>/` — e.g. into the
`spec/attachments/` root (auto-moved by the BUG-055 path) or any other directory — and let
`add-attachment` perform the copy into the per-work-unit directory.

## Impact history

| When (2026-07-10) | What was lost | Recovery |
|---|---|---|
| ~14:06 | CONT-006/007/008/009 research markdown attachments (4 files) | Rewritten via `Write` |
| ~22:0x | `spec/attachments/CONT-005/review-findings.md` | Rewritten via `Write` |
