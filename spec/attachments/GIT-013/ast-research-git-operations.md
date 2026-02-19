# AST Research: Git Operations to Replace

## Source Files Analyzed

### src/git/status.ts (11,598 bytes)

**Dependencies:**
- `isomorphic-git` - Main git library (to be replaced with gitoxide)
- `fs` (Node.js) - File system operations
- `memfs` - Mock filesystem for testing

**Exported Functions:**

| Function | Signature | Description |
|----------|-----------|-------------|
| `getStagedFiles` | `(dir: string, options?: GitStatusOptions) => Promise<string[]>` | Returns files staged for commit |
| `getUnstagedFiles` | `(dir: string, options?: GitStatusOptions) => Promise<string[]>` | Returns modified files not yet staged |
| `getUntrackedFiles` | `(dir: string, options?: GitStatusOptions) => Promise<string[]>` | Returns new files not tracked by git |
| `getCurrentBranch` | `(dir: string, options?: GitStatusOptions) => Promise<string \| undefined>` | Returns current branch name |
| `getGitStatus` | `(dir: string, options?: GitStatusOptions) => Promise<FileStatus[]>` | Returns combined status |
| `getFileStatus` | `(dir: string, filepath: string, options?: GitStatusOptions) => Promise<FileStatus \| null>` | Returns status for specific file |
| `getStagedFilesWithChangeType` | `(dir: string, options?: GitStatusOptions) => Promise<FileStatusWithChangeType[]>` | Staged files with A/M/D/R type |
| `getUnstagedFilesWithChangeType` | `(dir: string, options?: GitStatusOptions) => Promise<FileStatusWithChangeType[]>` | Unstaged files with change type |

**Exported Types:**

```typescript
interface FileStatus {
  filepath: string;
  staged: boolean;
  hasUnstagedChanges: boolean;
  untracked: boolean;
}

type ChangeType = 'A' | 'M' | 'D' | 'R';

interface FileStatusWithChangeType {
  filepath: string;
  changeType: ChangeType;
  staged: boolean;
}

interface GitStatusOptions {
  strict?: boolean;
  fs?: IFs;  // memfs compatibility - may need different approach with gitoxide
}
```

**Internal Functions:**
- `isGitRepository(dir, fs)` - Checks for .git directory
- `getStatusMatrix(dir, options)` - Wraps isomorphic-git statusMatrix
- `getChangeType(head, workdir, stage)` - Determines A/M/D/R from status values

**isomorphic-git APIs Used:**
- `git.statusMatrix({ fs, dir, cache })` - Main status operation
- `git.currentBranch({ fs, dir })` - Branch query

### src/git/diff.ts (6,924 bytes)

**Dependencies:**
- `isomorphic-git` - For reading blobs from commits
- `diff` library - Myers algorithm for line diffing
- `fs` (Node.js) - File system operations

**Exported Functions:**

| Function | Signature | Description |
|----------|-----------|-------------|
| `getFileDiff` | `(cwd: string, filepath: string) => Promise<string \| null>` | Returns unified diff for file changes |
| `getCheckpointFileDiff` | `(cwd: string, filepath: string, checkpointRef: string) => Promise<string \| null>` | Diff between checkpoint and HEAD |

**Internal Functions:**
- `generateUnifiedDiff(filepath, oldContent, newContent)` - Creates unified diff format
- `isBinaryContent(buffer)` - Detects binary files by checking for null bytes

**isomorphic-git APIs Used:**
- `git.resolveRef({ fs, dir, ref })` - Resolve ref to commit OID
- `git.readBlob({ fs, dir, oid, filepath })` - Read file content from commit

## gitoxide (gix) Mapping

### Status Operations

```rust
// gix equivalent for statusMatrix
use gix::Repository;

let repo = Repository::open(dir)?;
let status = repo.status(gix::status::Options::default())?;

for entry in status.entries() {
    let path = entry.path();
    let index_status = entry.index_status();
    let worktree_status = entry.worktree_status();
    // Map to our FileStatus struct
}
```

### Diff Operations

```rust
// gix equivalent for blob reading
let head = repo.head_commit()?;
let tree = head.tree()?;
let entry = tree.find_entry(filepath)?;
let blob = entry.object()?.into_blob();
let content = std::str::from_utf8(blob.data)?;
```

### Binary Detection

```rust
// gix has built-in binary detection via attributes
let attrs = repo.attributes(filepath)?;
let is_binary = attrs.is_binary();
```

### Branch Query

```rust
// gix equivalent for currentBranch
let head = repo.head()?;
match head.kind() {
    gix::head::Kind::Symbolic(reference) => {
        Some(reference.name().shorten().to_string())
    }
    gix::head::Kind::Detached { .. } => None,
}
```

## Migration Strategy

1. **Create codelet/git crate** with gitoxide dependency
2. **Mirror TypeScript API** in Rust with NAPI bindings
3. **Test parity** - Ensure identical output for all operations
4. **Replace imports** in src/git/status.ts and src/git/diff.ts
5. **Remove isomorphic-git** dependency from package.json

## Notes

- The `memfs` option in `GitStatusOptions` is used for testing. With gitoxide, we may need a different testing strategy (real temp directories or gitoxide's in-memory repo support).
- The `diff` library for Myers algorithm should be kept - gitoxide provides blob content but we still need the diffing logic.
- Binary detection should use gitoxide's attribute system for more accurate results.
