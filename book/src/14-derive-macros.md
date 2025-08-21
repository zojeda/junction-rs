# Derive Macros Deep Dive

## Node Derive Expansion
Generates:
- `impl Node for Type { const TABLE: &'static str = "table"; type Schema = table::Schema; fn schema() -> Schema { ... } }`
- `impl Insertable` & `HasTableName`
- `table` module containing `Schema` struct with one `Col<T>` per declared field plus synthesized `id`.

### Table Name Inference
If `#[junction(table = "person")]` absent, converts type name to snake_case.

### ID Field Handling
If an `id` field is present its type is used; otherwise a `SimpleId<Type>` column is synthesized.

## Edge Derive (Overview)
Similar pattern but also handles source/target inference, generating relationship schema including `in`, `out`, and `id` columns even if omitted.

## Custom Endpoint Names
Macro records attribute-marked fields; adapter maps them to canonical backend relation keys then restores original names upon deserialization.

## Schema Columns
Each field becomes `junction_rs::schema::Col<FieldType>` bound to `TABLE` and column name string.

## Insert Helper
A convenience `create_simple_id()` method may be added for Node types to generate new `SimpleId<Type>`.

## Error Reporting
Trybuild tests ensure clear compiler diagnostics for misconfigured derives (e.g. missing source/target in Edge).

## Maintenance Tips
- Keep macro output lean to reduce compile times.
- Expose only necessary helpers in public API to avoid surface bloat.

## Future Enhancements
- Additional attributes for default values or index hints.
- Macro-driven validation of field naming conventions.
