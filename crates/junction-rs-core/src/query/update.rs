use serde::Serialize;

use super::ast;
use super::ast::{Param, PredExpr};
use crate::expr::Expr;
use crate::schema::IntoTableName;
use crate::traits::DbExecutor;

/// Builder for `UPDATE` statements; returns rows of type `R`.
pub struct UpdateBuilder {
    table: &'static str,
    content: Option<serde_json::Value>,
    filter: Option<Expr>,
}

pub fn update(table: impl IntoTableName) -> UpdateBuilder {
    UpdateBuilder {
        table: table.table_name(),
        content: None,
        filter: None,
    }
}

impl UpdateBuilder {
    pub fn content(mut self, value: impl Serialize) -> Self {
        self.content = Some(serde_json::to_value(value).unwrap());
        self
    }
    pub fn where_(mut self, expr: Expr) -> Self {
        self.filter = Some(expr);
        self
    }

    pub async fn return_many<DB, R>(self, db: DB) -> Result<Vec<R>, crate::traits::DbError>
    where
        DB: DbExecutor,
        R: serde::de::DeserializeOwned + Send,
    {
        let filter = self.filter.map(expr_to_pred);
        let q = ast::Query::new(ast::Stmt::Update(ast::Update {
            table: self.table,
            merge: self.content,
            filter,
        }));
        db.execute::<R>(q).await
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
