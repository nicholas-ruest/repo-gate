# Bounded Context: ArchitectureMapping

**Subdomain**: Supporting
**Crate**: `rg-architecture-mapping`

---

## Purpose

ArchitectureMapping answers the question: **how is this codebase structured?** It detects module boundaries, builds a dependency graph, and classifies each module into a functional layer. The output is an `ArchitectureMap` that tells CommercialValuation and GatingStrategy which modules are central (hard to gate), which are peripheral (easy to gate), and how modules depend on each other.

This context does not opine on commercial value — it produces structural facts. The interpretation of structural relationships is left to CommercialValuation.

---

## Ubiquitous Language

| Term | Definition |
|---|---|
| **ArchitectureMap** | The aggregate root. The complete structural picture of the repository. |
| **ModuleNode** | A single logical unit within the repository (a crate, package, directory, or significant file cluster) with a name, path, and layer classification. |
| **DependencyEdge** | A directed relationship between two `ModuleNode`s indicating that the source depends on the target. |
| **Layer** | The functional role of a module: `core`, `api`, `sdk`, `cli`, `connector`, `integration`, `deployment`, `test`, `documentation`. |
| **DependencyGraph** | The directed graph formed by all `ModuleNode`s and `DependencyEdge`s. |
| **CycleDetection** | The process of finding circular dependencies within the `DependencyGraph`. |
| **Centrality** | A measure of how many other modules depend on a given module (in-degree in the dependency graph). High-centrality modules are risky to gate. |
| **Coupling** | The degree to which a module depends on internal implementation details of another module (tight vs. loose coupling). |
| **BoundaryStrength** | How well-defined a module's boundary is: `strong` (clear interface), `weak` (implementation details exposed), `unclear` (no apparent boundary). |
| **LayerViolation** | A `DependencyEdge` where a lower-layer module depends on a higher-layer module (e.g., `core` depending on `cli`). |

---

## Aggregate Root: `ArchitectureMap`

`ArchitectureMap` accumulates `ModuleNode`s and `DependencyEdge`s as module detection runs. It is complete when all modules have been mapped and all edges resolved.

### State Transitions

```
pending → detecting_modules → building_graph → classifying_layers → complete → failed
```

### Entity: `ModuleNode`

| Field | Type | Notes |
|---|---|---|
| `id` | `ModuleNodeId` | |
| `name` | `String` | Human-readable module name |
| `path` | `PathBuf` | Root directory or file of this module |
| `layer` | `Layer` | Classified functional role |
| `centrality` | `u32` | Number of modules that depend on this one |
| `boundary_strength` | `BoundaryStrength` | How well the module's interface is defined |
| `file_count` | `u32` | Files in this module |
| `loc` | `u64` | Lines of code |
| `language` | `Option<Language>` | Primary language |
| `has_public_interface` | `bool` | Exports a documented public API |
| `cycle_member` | `bool` | Is part of a dependency cycle |

### Value Objects

#### `DependencyEdge`
```rust
pub struct DependencyEdge {
    pub from: ModuleNodeId,
    pub to: ModuleNodeId,
    pub kind: DependencyKind, // Compile | Runtime | Optional | Dev | Build
    pub is_layer_violation: bool,
}
```

#### `Layer`
```rust
pub enum Layer {
    Core,          // fundamental runtime logic, algorithms, data structures
    Api,           // HTTP/gRPC/WebSocket interface layer
    Sdk,           // client SDK for consuming the API
    Cli,           // command-line interface
    Connector,     // third-party system integrations (databases, SaaS)
    Integration,   // workflow or data pipeline integrations
    Deployment,    // Dockerfile, Helm, Terraform, CI config
    Test,          // test suites and test utilities
    Documentation, // docs, examples, tutorials
}
```

#### `BoundaryStrength`
```rust
pub enum BoundaryStrength {
    Strong,   // clear exported interface; internals not exposed
    Weak,     // some internal details leaked through public API
    Unclear,  // cannot determine interface boundaries from code structure
}
```

---

## Invariants

1. Every `DependencyEdge` must reference existing `ModuleNodeId`s — dangling edges are rejected.
2. `ModuleNode.centrality` is always derived from the live `DependencyGraph`; it may not be set manually.
3. A `LayerViolation` is flagged whenever a `Layer::Core` module imports from `Layer::Api`, `Layer::Cli`, or `Layer::Integration`. The violation is recorded but does not block mapping completion.
4. Circular dependencies are detected and flagged; `ModuleNode.cycle_member` is set to `true` for all nodes in a cycle.
5. The `ArchitectureMap` must contain at least one `Layer::Core` module; if none is detected, the map completes with a warning.
6. `ModuleNode` boundaries are determined by package manifest files (Cargo.toml, package.json, etc.) first, then by directory structure heuristics. LLM inference is a last resort and must be marked as such.

---

## Domain Events

### `ModuleMapped`
Emitted when a `ModuleNode` is added to the map.
```rust
pub struct ModuleMapped {
    pub repo_id: RepositoryId,
    pub map_id: ArchitectureMapId,
    pub module: ModuleNode,
    pub mapped_at: DateTime<Utc>,
}
```

### `DependencyGraphBuilt`
Emitted when all modules and edges have been resolved.
```rust
pub struct DependencyGraphBuilt {
    pub repo_id: RepositoryId,
    pub map_id: ArchitectureMapId,
    pub node_count: u32,
    pub edge_count: u32,
    pub cycle_count: u32,
    pub layer_violation_count: u32,
    pub built_at: DateTime<Utc>,
}
```

### `LayerViolationDetected`
Emitted per layer-violating edge.
```rust
pub struct LayerViolationDetected {
    pub repo_id: RepositoryId,
    pub map_id: ArchitectureMapId,
    pub edge: DependencyEdge,
    pub description: String,
}
```

### `CycleDetected`
Emitted when a circular dependency is found.
```rust
pub struct CycleDetected {
    pub repo_id: RepositoryId,
    pub map_id: ArchitectureMapId,
    pub cycle_nodes: Vec<ModuleNodeId>,
}
```

---

## Repository Interface

```rust
pub trait ArchitectureMapStore {
    async fn save(&self, map: &ArchitectureMap) -> Result<(), StoreError>;
    async fn find_by_repo_id(&self, repo_id: RepositoryId) -> Result<Option<ArchitectureMap>, StoreError>;
    async fn add_node(&self, map_id: ArchitectureMapId, node: ModuleNode) -> Result<(), StoreError>;
    async fn add_edge(&self, map_id: ArchitectureMapId, edge: DependencyEdge) -> Result<(), StoreError>;
    async fn list_nodes(&self, map_id: ArchitectureMapId) -> Result<Vec<ModuleNode>, StoreError>;
    async fn list_edges(&self, map_id: ArchitectureMapId) -> Result<Vec<DependencyEdge>, StoreError>;
    async fn nodes_by_layer(&self, map_id: ArchitectureMapId, layer: Layer) -> Result<Vec<ModuleNode>, StoreError>;
}
```

---

## Integration Points

| Context | Direction | What crosses the boundary |
|---|---|---|
| RepositoryIngestion | Upstream | Subscribes to `ManifestBuilt`; uses directory structure and package files |
| AssessmentOrchestration | Coordinator | Issues `MapArchitecture` command; subscribes to `DependencyGraphBuilt` |
| FunctionalityDiscovery | Downstream | Subscribes to `ModuleMapped` to organise per-module discovery scans |
| CommercialValuation | Downstream | Reads `ModuleNode` list (layer, centrality, boundary strength) for scoring |
| GatingStrategy | Downstream | Uses centrality and layer to inform tier boundary placement |
| ReportDelivery | Downstream | Reads the full `ArchitectureMap` for the architecture section of the report |

### Anti-Corruption Layer

No formal ACL is needed between ArchitectureMapping and its direct upstream (RepositoryIngestion). The infrastructure adapter for parsing Cargo.toml, package.json, and similar manifests is internal to this context and translates external formats into `ModuleNode` structures.
