# Expression DSL

The `expr!` macro builds binary predicates with infix syntax and minimal parsing.

## Supported Operators
`==`, `!=`, `>`, `>=`, `<`, `<=` plus unary `!(...)`.

## Example
```rust
let s = Person::schema();
let predicate = expr!(s.age > 40).and(expr!(s.marketing == true));
```

## Logical Composition
Use `.and()` / `.or()` on `Expr`. Use `!expr` or `expr!( !(s.age > 40) )` for negation.

## Ordering with Expressions
Ordering is separate via `.order_by(s.age.desc())` on query builders.

## Internals
`ExprKind` enum shapes the tree. Builders avoid evaluation; adapter compilers translate to backend-specific predicates.

## Limitations
- No `&&` / `||` inside macro; explicit composition keeps parsing simple.
- Precedence is manual; compose stepwise.

## Tips
- Keep each `expr!` simple; chain for complex logic.
- Use typed columns to avoid runtime mistakes.
