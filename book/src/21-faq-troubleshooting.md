# FAQ & Troubleshooting

## Feature Not Found Errors
Ensure `features = ["surreal"]` is enabled in `Cargo.toml` when using adapter types.

## Macro Derive Fails
Check that Edge structs have exactly one `#[junction(source)]` and one `#[junction(target)]` annotation.

## Unexpected Serialization Format
Verify `SimpleId<T>` values include the correct table prefix; mismatches cause deserialization errors.

## Traversal Path Missing Edges
Multi-hop path capture currently restricted; ensure you meet constraints (single edge type per hop, outward direction, no edge filters in path mode).

## Projection Errors
Field names/types in projection struct must match model. Extra fields in struct not present on backend cause serde failures.

## Empty Results
Confirm predicate logic; start by removing filters and re-adding stepwise.

## Performance Concerns
Early project stage focuses on correctness. Open an issue with reproducible scenario for optimization discussion.

## How Do I Add a New Adapter?
See Implementing a New Adapter chapter; start with basic select/insert/delete support.

## Can I Use Raw Queries?
Currently the DSL is primary; adapter-specific escape hatches may be added later with caution.

## Expression With Complex Logic
Chain `.and()` / `.or()` manually; macro does not parse nested logical operators beyond unary `!`.

## Getting Help
Open GitHub issues with minimal reproduction code and clear version info.
