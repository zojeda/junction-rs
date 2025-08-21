use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table<T> {
    pub name: &'static str,
    _p: std::marker::PhantomData<T>,
}
impl<T> Table<T> {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            _p: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeTable<E, From, To> {
    pub name: &'static str,
    _p: std::marker::PhantomData<(E, From, To)>,
}
impl<E, From, To> EdgeTable<E, From, To> {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            _p: Default::default(),
        }
    }
}

/// Trait to extract a static table name from table handle types
pub trait IntoTableName {
    fn table_name(&self) -> &'static str;
}

impl<T> IntoTableName for Table<T> {
    fn table_name(&self) -> &'static str {
        self.name
    }
}
impl<T> IntoTableName for &Table<T> {
    fn table_name(&self) -> &'static str {
        self.name
    }
}
impl<E, F, To> IntoTableName for EdgeTable<E, F, To> {
    fn table_name(&self) -> &'static str {
        self.name
    }
}
impl<E, F, To> IntoTableName for &EdgeTable<E, F, To> {
    fn table_name(&self) -> &'static str {
        self.name
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Col<T> {
    pub table: &'static str,
    pub name: &'static str,
    _p: std::marker::PhantomData<T>,
}
impl<T> Col<T> {
    pub fn new(table: &'static str, name: &'static str) -> Self {
        Self {
            table,
            name,
            _p: Default::default(),
        }
    }
}

impl<T: Serialize> Col<T> {
    /// Equality predicate
    pub fn eq<V: Into<T>>(self, v: V) -> crate::expr::Expr {
        crate::expr::Expr::binary(&self, "=", serde_json::to_value(v.into()).unwrap())
    }
    /// Alias for eq
    pub fn equal<V: Into<T>>(self, v: V) -> crate::expr::Expr {
        self.eq(v)
    }
    /// Less-than predicate
    pub fn lt<V: Into<T>>(self, v: V) -> crate::expr::Expr {
        crate::expr::Expr::binary(&self, "<", serde_json::to_value(v.into()).unwrap())
    }
    /// Less-than-or-equal predicate
    pub fn lte<V: Into<T>>(self, v: V) -> crate::expr::Expr {
        crate::expr::Expr::binary(&self, "<=", serde_json::to_value(v.into()).unwrap())
    }
    /// Greater-than predicate
    pub fn gt<V: Into<T>>(self, v: V) -> crate::expr::Expr {
        crate::expr::Expr::binary(&self, ">", serde_json::to_value(v.into()).unwrap())
    }
    /// Greater-than-or-equal predicate
    pub fn gte<V: Into<T>>(self, v: V) -> crate::expr::Expr {
        crate::expr::Expr::binary(&self, ">=", serde_json::to_value(v.into()).unwrap())
    }
    /// IN predicate (value contained in list/array)
    pub fn isin<I: Serialize>(self, items: I) -> crate::expr::Expr {
        crate::expr::Expr::binary(&self, "IN", serde_json::to_value(items).unwrap())
    }
    /// LIKE predicate (pattern match)
    pub fn like<V: Into<T>>(self, pattern: V) -> crate::expr::Expr {
        crate::expr::Expr::binary(&self, "LIKE", serde_json::to_value(pattern.into()).unwrap())
    }
    /// BETWEEN predicate inclusive (synthesizes (col >= low AND col <= high))
    pub fn between<V: Into<T>>(self, low: V, high: V) -> crate::expr::Expr {
        let l = crate::expr::Expr::binary(&self, ">=", serde_json::to_value(low.into()).unwrap());
        let r = crate::expr::Expr::binary(&self, "<=", serde_json::to_value(high.into()).unwrap());
        l.and(r)
    }
    /// Descending ordering specifier
    pub fn desc(self) -> crate::expr::Ordering {
        crate::expr::Ordering::Desc(self.table, self.name)
    }
    /// Ascending ordering specifier
    pub fn asc(self) -> crate::expr::Ordering {
        crate::expr::Ordering::Asc(self.table, self.name)
    }
}
