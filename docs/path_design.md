# Path Traversal / Return Mode Design

Status: In Progress (single-hop + Phase C multi-hop node + edge sequence implemented)  
Last Updated: 2025-09-28 (phase C multi-hop edges)

## Goals
Provide a `ReturnMode::Path` that returns ordered traversal paths capturing:

* Node identifiers (and edge ids) for each hop
* Support for tail-optional repeats already modeled in the graph DSL
* Deterministic, stable JSON shape suitable for serde-based deserialization

## Non-Goals (Initial Phase)
* Arbitrary mid-path branching enumeration (only existing union + tail repeat variants)
* Mixed node schema projections inside the same path response
* Automatic cycle detection / pruning (user controls max depth)

## Result Shape
Return `Vec<PathSegment>` where:

```rust
#[derive(serde::Deserialize)]
struct PathSegment {
    nodes: Vec<String>,      // fully qualified record ids in traversal order
    edges: Option<Vec<String>>, // optional parallel edge id list (len = nodes.len()-1) when/if edge capture enabled
}
```

Rationale: uniform struct aids versioning; easy to extend with metadata (costs, weights) later.

## Current Implementation (Baseline + Phases B & C Multi-Hop)
* Single-hop: `PathSegment { nodes: [start, end], edges: [edge_id] }` (captures the traversed edge id).
* Multi-hop (Phase B): introduced full node sequence capture.
* Multi-hop (Phase C): now captures edge ids across the entire path: `PathSegment { nodes: [start, n1, ..., terminal], edges: Some([e0, e1, ...]) }` where invariant `edges.len() == nodes.len() - 1` holds.
  * Constraints (still enforced in Phases B & C):
    * All steps must be outward.
    * Exactly one edge type per step (no unions / fan-out yet).
    * No edge-level filters (node terminal filters allowed; edge filters error for now).
  * Violations emit descriptive adapter errors.
* Rationale: phased rollout ensured stable `PathSegment` shape; Phase C enriches data without breaking prior consumers that handled `edges: None`.

## Multi-Hop Compilation Strategy (Implemented for Phases B & C)
Enumerate traversal variants (tail repeat) then for each variant construct a SELECT VALUE returning the path object.

Preferred approach (LET chain + final SELECT) for clarity & control:

```sql
LET $p0 = type::thing('person', '...');
LET $e0 = (SELECT * FROM $p0->knows WHERE strength > $p1); -- edge-level filters
LET $p1 = (SELECT out AS id FROM $e0); -- collect destination node ids
-- Repeat per hop
SELECT VALUE {
  nodes: [$p0.id, (SELECT VALUE id FROM $p1)[0], ...],
  edges: (SELECT VALUE id FROM $e0)  -- or flattened array of each $e{i}.id concatenated
};
```

If multiple edges per hop (edge union) produce fan-out, each edge row yields a separate path row—natural cartesian expansion semantics.

Repetition (`repeat(min,max)`) generates multiple variant chains (0..=extra optional occurrences) UNION ALL combined.

Edge ID capture: Maintains `edges: [id0, id1, ...]` with length invariant `edges.len() == nodes.len() - 1` (enforced implicitly by compiler generation strategy & test coverage). Surreal's nested `Array` / `Thing` wrappers are adapter-normalized (single-element array unwrap) so serde receives a flat `Vec<String>`.

## Open Questions
* Performance: repeated subselects vs. staged LET bindings per hop.
* Optional edge capture toggle (API extension?)
* Error surface when path exceeds certain backend recursion limits.

## Incremental Implementation Plan
1. ✅ `PathSegment` struct introduced; `return_path()` builder present.
2. ✅ Single-hop with edge id capture implemented.
3. ✅ Multi-hop Phase A (endpoints only, constrained; edges None) without repetition.
4. ✅ Multi-hop Phase B: full node sequence capture (nodes len = hops+1) still with `edges: None`.
5. ✅ Phase C: edge id capture across multi-hop (populate `edges: Some([...])`).
6. ⏳ Tail repetition variant expansion (enumerate repeat variants via UNION ALL).
7. ⏳ Edge-level filters & unions in multi-hop (fan-out path enumeration) with tests.
8. ⏳ Optimization: consolidate node/edge collection subqueries & avoid redundant SELECT VALUE wrappers.
9. ⏳ Documentation refinement & README update (continuous) – reflect phases clearly (Phase C reflected).

## Risks
* SurrealQL limitations constructing multi-hop arrays may require nested SELECT VALUE indirection.
* Large path fan-out could inflate payload size; may revisit streaming or chunking.
* Edge property projection inside path objects (future) could complicate shape backward compatibility.

## Future Extensions
* Weighted path scoring / shortest path helpers.
* Cycle controls (simple path mode) / pruning.
* Early termination predicates / depth limits.
* Optional embedding of selected edge properties (not just ids).

---
Feedback welcome before coding begins.