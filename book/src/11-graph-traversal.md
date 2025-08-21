# Graph Traversal DSL (Experimental)

Traversal APIs enable relationship-centric queries.

## Starting Points
- `start::<NodeType>()` (all nodes)
- `start_at::<NodeType>(id)` (specific node id)

## Steps
- `.forward::<EdgeType>()`
- `.backward::<EdgeType>()`

## Filtering
- `.filter_edge(expr!(e.strength > 3))`
- `.filter_node(expr!(s.age > 18))`

## Projection
- `.project_fields::<Subset>(&["name"])` terminal node fields
- `.return_edges()` fetch edge documents
- `.return_path()` capture node sequence (+ edge ids for multi-hop)

## Path Semantics
`PathSegment { nodes: [...], edges: Some([...]) }` with invariant `edges.len() == nodes.len() - 1` when multi-hop.

## Repetition
`.repeat(min, max)` expands tail variants (safety cap). Mixed directions or unions in repeat currently restricted.

## Edge Unions
Tuple-based unions (up to N) allow multi-type traversal fan-out (limitations apply in path mode).

## Limitations
- Edge-level filters disallowed during multi-hop path capture (planned).
- Single edge type per hop in path mode today.
- Projection of edge properties inside path segments pending design.

## Example
```rust
let s = Person::schema();
let e = Knows::schema();
let paths = start_at::<Person>(alice_id)
  .forward::<Knows>()
  .filter_edge(expr!(e.strength > 3))
  .filter_node(expr!(s.age > 18))
  .return_path()
  .return_many(db.clone())
  .await?;
```

## Roadmap
Enhance unions, richer repetition, and embedding edge property subsets.
