use serde_json::Value;

/// Logical expression tree used in the high-level filter DSL.
#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Binary {
        table: &'static str,
        col: &'static str,
        op: &'static str,
        val: Value,
    },
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Not(Box<Expr>),
    True,
}

/// A boolean expression.
#[derive(Debug, Clone, PartialEq)]
pub struct Expr(pub ExprKind);

/// Ordering specifier used in queries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ordering {
    Asc(&'static str, &'static str),
    Desc(&'static str, &'static str),
}

/// Wildcard marker for `SELECT *`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct All;

impl Expr {
    /// Create a binary predicate on a column.
    pub fn binary<T>(col: &crate::schema::Col<T>, op: &'static str, val: Value) -> Self {
        Expr(ExprKind::Binary {
            table: col.table,
            col: col.name,
            op,
            val,
        })
    }
    /// Conjunction (AND)
    pub fn and(self, rhs: Expr) -> Self {
        Expr(ExprKind::And(Box::new(self), Box::new(rhs)))
    }
    /// Disjunction (OR)
    pub fn or(self, rhs: Expr) -> Self {
        Expr(ExprKind::Or(Box::new(self), Box::new(rhs)))
    }
    /// Negation (NOT). Prefer using the `!expr` operator; this convenience mirrors older API.
    pub fn negate(self) -> Self {
        Expr(ExprKind::Not(Box::new(self)))
    }
}

impl std::ops::Not for Expr {
    type Output = Expr;
    fn not(self) -> Self::Output {
        Expr(ExprKind::Not(Box::new(self)))
    }
}

/// Helper to emphasize conditional expressions in builder chains.
pub fn cond(e: Expr) -> Expr {
    e
}

/// Internal helper for macros to convert values to serde_json::Value
#[doc(hidden)]
pub fn __to_value<T: serde::Serialize>(v: T) -> serde_json::Value {
    serde_json::to_value(v).unwrap()
}

/// Shorthand macro to build binary column predicates as `Expr` using an infix DSL.
///
/// Supported forms:
/// - `expr!(col == value)`
/// - `expr!(col != value)`
/// - `expr!(col > value)`
/// - `expr!(col >= value)`
/// - `expr!(col < value)`
/// - `expr!(col <= value)`
/// - `expr!(!(col == value))`
///
/// Notes:
/// - For complex logic, compose with `.and(...)`, `.or(...)`, and the `!` operator (or `.negate()`) on the resulting `Expr`.
/// - This macro intentionally does not implement `&&`/`||` parsing to keep it declarative-only.
#[macro_export]
macro_rules! expr {
	// Parenthesized expression: just re-evaluate inside
	( ( $($inner:tt)+ ) ) => { $crate::expr!($($inner)+) };

	// Unary NOT applied to a parenthesized comparison
    ( ! ( $($inner:tt)+ ) ) => { !$crate::expr!($($inner)+) };

	// Generic infix form: delegate to a parser that splits at the operator
	( $($tokens:tt)+ ) => { $crate::__lca_expr_parse!(@lhs [] $($tokens)+ ) };
}

/// Internal helper to parse the left and right sides as Rust expressions with a specific operator
#[doc(hidden)]
#[macro_export]
macro_rules! __lca_expr_build {
    ( ( $col:expr ) == ( $val:expr ) ) => {
        $crate::expr::Expr::binary(&$col, "=", $crate::expr::__to_value($val))
    };
    ( ( $col:expr ) != ( $val:expr ) ) => {
        $crate::expr::Expr::binary(&$col, "!=", $crate::expr::__to_value($val))
    };
    ( ( $col:expr ) >= ( $val:expr ) ) => {
        $crate::expr::Expr::binary(&$col, ">=", $crate::expr::__to_value($val))
    };
    ( ( $col:expr ) >  ( $val:expr ) ) => {
        $crate::expr::Expr::binary(&$col, ">", $crate::expr::__to_value($val))
    };
    ( ( $col:expr ) <= ( $val:expr ) ) => {
        $crate::expr::Expr::binary(&$col, "<=", $crate::expr::__to_value($val))
    };
    ( ( $col:expr ) <  ( $val:expr ) ) => {
        $crate::expr::Expr::binary(&$col, "<", $crate::expr::__to_value($val))
    };
}

/// Internal helper: TT muncher to split tokens at the comparison operator
#[doc(hidden)]
#[macro_export]
macro_rules! __lca_expr_parse {
	// When we find the operator, delegate to builder with expr fragments
	(@lhs [$($lhs:tt)*] == $($rhs:tt)+) => { $crate::__lca_expr_build!( ($($lhs)*) == ($($rhs)*) ) };
	(@lhs [$($lhs:tt)*] != $($rhs:tt)+) => { $crate::__lca_expr_build!( ($($lhs)*) != ($($rhs)*) ) };
	(@lhs [$($lhs:tt)*] >= $($rhs:tt)+) => { $crate::__lca_expr_build!( ($($lhs)*) >= ($($rhs)*) ) };
	(@lhs [$($lhs:tt)*] >  $($rhs:tt)+) => { $crate::__lca_expr_build!( ($($lhs)*) >  ($($rhs)*) ) };
	(@lhs [$($lhs:tt)*] <= $($rhs:tt)+) => { $crate::__lca_expr_build!( ($($lhs)*) <= ($($rhs)*) ) };
	(@lhs [$($lhs:tt)*] <  $($rhs:tt)+) => { $crate::__lca_expr_build!( ($($lhs)*) <  ($($rhs)*) ) };

	// Keep consuming tokens into the LHS until we hit an operator
	(@lhs [$($lhs:tt)*] $t:tt $($rest:tt)+) => { $crate::__lca_expr_parse!(@lhs [$($lhs)* $t] $($rest)+) };
}
