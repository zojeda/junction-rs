#![allow(dead_code)]
use std::marker::PhantomData;

extern crate uuid;

pub mod id {
    use super::PhantomData;
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct SimpleId<T> { _marker: PhantomData<T> }
    impl<T> SimpleId<T> { pub(crate) fn new() -> Self { Self { _marker: PhantomData } } }
}

pub mod traits {
    use super::id::SimpleId;
    pub trait HasTableName { fn table_name() -> &'static str; }

    pub trait Node: HasTableName {
        const TABLE: &'static str;
        type Schema;
        fn schema() -> Self::Schema;
    }

    pub trait Edge: HasTableName {
        const TABLE: &'static str;
        type Schema;
        type From: Node;
        type To: Node;
        const SOURCE_FIELD_NAME: &'static str = "in";
        const TARGET_FIELD_NAME: &'static str = "out";
        fn schema() -> Self::Schema;
    }

    pub trait Insertable: Sized + 'static {
        fn table_name() -> &'static str;
        fn endpoint_fields() -> Option<(&'static str, &'static str)> {
            None
        }
    }

    pub trait WithId: HasTableName + Sized {
        fn create_simple_id() -> SimpleId<Self> { SimpleId::new() }
        fn from_uuid(_id: uuid::Uuid) -> SimpleId<Self> { SimpleId::new() }
    }

    impl<T: HasTableName> WithId for T {}
}

pub mod schema {
    use super::PhantomData;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Col<T> {
        pub table: &'static str,
        pub column: &'static str,
        _marker: PhantomData<T>,
    }

    impl<T> Col<T> {
        pub fn new(table: &'static str, column: &'static str) -> Self {
            Self {
                table,
                column,
                _marker: PhantomData,
            }
        }
    }
}

// (id module moved earlier)
