use super::ast;
use super::ast::{Param, PredExpr};
use crate::expr::Expr;
use crate::schema::IntoTableName;
use crate::traits::DbExecutor;

/// Builder for `DELETE` statements; returns `()` on success.
pub struct DeleteBuilder {
    table: &'static str,
    filter: Option<Expr>,
}

pub fn delete(table: impl IntoTableName) -> DeleteBuilder {
    DeleteBuilder {
        table: table.table_name(),
        filter: None,
    }
}

impl DeleteBuilder {
    pub fn where_(mut self, expr: Expr) -> Self {
        self.filter = Some(expr);
        self
    }

    pub async fn run<DB>(self, db: DB) -> Result<(), crate::traits::DbError>
    where
        DB: DbExecutor,
    {
        let filter = self.filter.map(expr_to_pred);
        let q = ast::Query::new(ast::Stmt::Delete(ast::Delete {
            table: self.table,
            filter,
        }));
        db.execute::<serde_json::Value>(q).await.map(|_| ())
    }
}

fn expr_to_pred(e: Expr) -> PredExpr {
    use crate::expr::ExprKind as K;
    match e.0 {
        K::True => PredExpr::True,
        K::Binary {
            col,
            table: _,
            op,
            val,
            ..
        } => PredExpr::Binary {
            col,
            op,
            param: Param::Inline(val),
        },
        K::And(a, b) => PredExpr::And(Box::new(expr_to_pred(*a)), Box::new(expr_to_pred(*b))),
        K::Or(a, b) => PredExpr::Or(Box::new(expr_to_pred(*a)), Box::new(expr_to_pred(*b))),
        K::Not(a) => PredExpr::Not(Box::new(expr_to_pred(*a))),
    }
}
