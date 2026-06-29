# Bounded Context: RepositoryIngestion

**Subdomain**: Supporting
**Crate**: `rg-repository-ingestion`

---

## Purpose

RepositoryIngestion is responsible for acquiring the raw repository, making its contents available to all downstream analysis contexts in a structured form. It handles git clone/fetch, filesystem traversal, binary filtering, language detection, and manifest extraction. Once a `Repository` aggregate is fully built and indexed, it publishes events that allow every analysis context to begin work without touching the filesystem directly.

This context is **upstream of everything**. Its output — `ManifestBuilt` — is the starting gun.

---

## Ubiquitous Language

| Term | Definition |
|---|---|
| **Repository** | The aggregate root. Represents a cloned or fetched VCS repository at a specific commit. |
| **RepoUrl** | A validated URL pointing to a public or authenticated VCS endpoint (GitHub, GitLab, Forgejo, bare HTTP). |
| **CommitSha** | An immutable 40-character hex SHA-1 identifying the exact revision being analyzed. Once set, it never changes. |
| **FileEntry** | A single file within the repository tree, including its path, size, detected language, hash, and whether it is binary. |
| **ModuleManifest** | A structured summary of the repository's files, directories, package metadata, and detected languages. Produced after full traversal. |
| **FileHash** | A BLAKE3 hash of a file's content, used for deduplication and integrity. |
| **LanguageStats** | The count of source lines per detected language across the repository (via `tokei` or equivalent). |
| **Binary filter** | The rule that excludes binary files (images, compiled objects, archives) from textual analysis. |
| **Traversal** | The depth-first walk of the repository tree that produces all `FileEntry` instances. |
| **Manifest** | Synonym for `ModuleManifest`. The structured index of repository contents. |

---

## Aggregate Root: `Repository`

The `Repository` aggregate owns the lifecycle from a raw URL through to a fully-indexed manifest. All mutations happen through `Repository` command methods; no external context modifies its internal state.

### State Transitions

```
pending → cloning → cloned → traversing → indexed → failed
```

### Entities (owned by Repository)

#### `FileEntry`

| Field | Type | Notes |
|---|---|---|
| `path` | `PathBuf` | Relative to repo root |
| `hash` | `FileHash` | BLAKE3 |
| `size_bytes` | `u64` | |
| `language` | `Option<Language>` | None for binary or unrecognised |
| `is_binary` | `bool` | |
| `is_generated` | `bool` | Detected via `.gitattributes` or path heuristics |

#### `ModuleManifest`

| Field | Type | Notes |
|---|---|---|
| `repo_id` | `RepositoryId` | FK to owning Repository |
| `total_files` | `u32` | |
| `total_loc` | `u64` | Sum across all FileEntries |
| `language_stats` | `LanguageStats` | Per-language breakdown |
| `package_files` | `Vec<PackageFileRef>` | Cargo.toml, package.json, pyproject.toml, etc. |
| `root_dirs` | `Vec<String>` | Top-level directory names |

### Value Objects

#### `RepoUrl`
- Wraps a `url::Url`. Invariant: scheme must be `https` or `ssh`. Path must have at least two segments (owner/repo). Validated on construction.
- `fn new(raw: &str) -> Result<RepoUrl, RepoUrlError>`

#### `CommitSha`
- Wraps a `[u8; 20]`. Created from a 40-char hex string. Immutable after construction.

#### `FileHash`
- Wraps a `[u8; 32]` BLAKE3 digest.

#### `LanguageStats`
- `HashMap<Language, LocCount>` where `Language` is an enum over recognised languages.

---

## Invariants

1. A `Repository` in state `indexed` must have a `CommitSha` set. (A floating `HEAD` clone is always resolved to a SHA before transitioning past `cloned`.)
2. `ModuleManifest` may only be attached to a `Repository` in state `indexed`.
3. Binary files are never included in `ModuleManifest.total_loc`.
4. A `RepoUrl` with a `file://` scheme is rejected — only remote VCS sources are accepted.
5. `FileHash` must be computed from actual file content; it cannot be null or zero for non-binary files.
6. A repository transitions to `failed` if cloning, traversal, or manifest building raises an unrecoverable error; it never silently discards the error.

---

## Domain Events

### `RepositoryCloned`
Emitted when the git clone/fetch completes and `CommitSha` is resolved.
```rust
pub struct RepositoryCloned {
    pub repo_id: RepositoryId,
    pub url: RepoUrl,
    pub commit_sha: CommitSha,
    pub cloned_at: DateTime<Utc>,
}
```

### `ManifestBuilt`
Emitted when the full traversal completes and `ModuleManifest` is ready.
```rust
pub struct ManifestBuilt {
    pub repo_id: RepositoryId,
    pub manifest: ModuleManifest,
    pub built_at: DateTime<Utc>,
}
```

### `FileIndexed`
Emitted per file during traversal (streamed, not batched). Consumers may begin processing before the full manifest is ready.
```rust
pub struct FileIndexed {
    pub repo_id: RepositoryId,
    pub entry: FileEntry,
}
```

### `IngestionFailed`
Emitted when an unrecoverable error prevents manifest completion.
```rust
pub struct IngestionFailed {
    pub repo_id: RepositoryId,
    pub reason: IngestionFailureReason,
    pub failed_at: DateTime<Utc>,
}
```

---

## Repository Interface

```rust
pub trait RepositoryStore {
    async fn save(&self, repo: &Repository) -> Result<(), StoreError>;
    async fn find_by_id(&self, id: RepositoryId) -> Result<Option<Repository>, StoreError>;
    async fn find_by_url_and_sha(
        &self,
        url: &RepoUrl,
        sha: &CommitSha,
    ) -> Result<Option<Repository>, StoreError>;
}

pub trait FileEntryStore {
    async fn save_batch(&self, entries: &[FileEntry]) -> Result<(), StoreError>;
    async fn list_for_repo(&self, repo_id: RepositoryId) -> Result<Vec<FileEntry>, StoreError>;
    async fn find_by_hash(&self, hash: &FileHash) -> Result<Option<FileEntry>, StoreError>;
}
```

---

## Integration Points

| Context | Direction | What crosses the boundary |
|---|---|---|
| AssessmentOrchestration | Downstream consumer | Issues `CloneRepository` command; subscribes to `ManifestBuilt` |
| LicenseCompliance | Downstream consumer | Subscribes to `ManifestBuilt`; reads `FileEntry` list to locate LICENSE files |
| FunctionalityDiscovery | Downstream consumer | Subscribes to `ManifestBuilt`; receives `ModuleManifest` |
| ArchitectureMapping | Downstream consumer | Subscribes to `ManifestBuilt`; receives directory structure and package files |

### Anti-Corruption Layer

There is no ACL on the downstream side — `RepositoryIngestion` publishes clean domain events. Downstream contexts are responsible for mapping the `FileEntry` and `ModuleManifest` types into their own internal representations.

The only ACL concern is **inbound**: `RepoUrl` validation acts as a lightweight ACL at the ingestion boundary, rejecting malformed, unsupported, or dangerous URLs (e.g., localhost, internal IPs) before any git operation begins.
