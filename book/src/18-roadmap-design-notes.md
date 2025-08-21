# Roadmap & Design Notes

## Roadmap Highlights
- Update & Patch semantics
- Richer aggregation DSL (group by, count, sum, avg)
- Edge unions & filters in multi-hop path capture
- Repetition semantics improvements (smart lowering)
- Additional adapters (community contributions welcome)
- Migration tooling maturation

## Path Design Evolution
Phased approach:
- Phase A: Single-hop path capture
- Phase B: Multi-hop node sequence
- Phase C: Node + edge id sequences with invariants
Future phases add unions, edge property embedding, and advanced repetition.

## Guiding Principles
- Type safety over runtime validation
- Minimal magic; explicit builder chaining
- Backend-agnostic core abstractions
- Small public API surface

## Deferred Complexity
Features like caching, query planning, advanced optimization postponed until semantics stable.

## Feedback Loop
Real usage and traversal scenarios drive prioritization; open issues with concrete examples.
