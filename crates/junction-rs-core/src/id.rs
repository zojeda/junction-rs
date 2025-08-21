use std::fmt::{Display, Formatter};
use std::marker::PhantomData;
use uuid::Uuid;

pub enum PrimaryKey {
    String(String),
    Integer(i64),
    Uuid(uuid::Uuid),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SimpleId<T> {
    pub value: Uuid,
    _marker: PhantomData<T>,
}

impl<T> SimpleId<T> {
    pub fn new() -> Self {
        Self {
            value: Uuid::new_v4(),
            _marker: PhantomData,
        }
    }
    pub fn from_uuid(value: Uuid) -> Self {
        Self {
            value,
            _marker: PhantomData,
        }
    }
    /// Return the underlying UUID as a hyphenated string.
    pub fn as_uuid_str(&self) -> String {
        self.value.to_string()
    }
    /// Return the compact token part used as the record id (prefix 'u' + 32 hex chars)
    pub fn token_part(&self) -> String {
        format!("u{}", self.value.simple())
    }
}

impl<T> Default for SimpleId<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Display for SimpleId<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

// Serde implementation notes:
// - SimpleId serializes/deserializes as "table:uuid" for compatibility with SurrealDB and other DBs.

mod serde_impls {
    use super::*;
    use crate::traits::HasTableName;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    impl<T: HasTableName> Serialize for SimpleId<T> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let table = T::table_name();
            serializer.serialize_str(&format!("{}:{}", table, self.value))
        }
    }

    impl<'de, T> Deserialize<'de> for SimpleId<T> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct SimpleIdVisitor<T>(PhantomData<T>);

            impl<'de, T> serde::de::Visitor<'de> for SimpleIdVisitor<T> {
                type Value = SimpleId<T>;

                fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    formatter
                        .write_str("a string in 'table:uuid' format or a SurrealDB Thing object")
                }

                fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    let parts: Vec<&str> = value.split(':').collect();
                    if parts.len() == 2 {
                        let uuid =
                            Uuid::parse_str(parts[1]).map_err(|e| E::custom(e.to_string()))?;
                        Ok(SimpleId {
                            value: uuid,
                            _marker: PhantomData,
                        })
                    } else {
                        let uuid = Uuid::parse_str(value).map_err(|_| {
                            E::custom(format!("Invalid SimpleId format: {}", value))
                        })?;
                        Ok(SimpleId {
                            value: uuid,
                            _marker: PhantomData,
                        })
                    }
                }

                fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
                where
                    A: serde::de::MapAccess<'de>,
                {
                    let mut id = None;
                    while let Some(key) = map.next_key::<String>()? {
                        if key == "id" {
                            id = Some(map.next_value::<String>()?);
                        } else {
                            let _ = map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                    let id =
                        id.ok_or_else(|| <A::Error as serde::de::Error>::missing_field("id"))?;
                    let uuid = Uuid::parse_str(&id)
                        .map_err(|e| <A::Error as serde::de::Error>::custom(e.to_string()))?;
                    Ok(SimpleId {
                        value: uuid,
                        _marker: PhantomData,
                    })
                }
            }

            deserializer.deserialize_any(SimpleIdVisitor(PhantomData))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct Dummy;

    impl crate::traits::HasTableName for Dummy {
        fn table_name() -> &'static str {
            "dummy"
        }
    }

    #[test]
    fn roundtrip_uuid_default() {
        let id = SimpleId::<Dummy>::from_uuid(Uuid::nil());
        let s = serde_json::to_string(&id).unwrap();
        let id2: SimpleId<Dummy> = serde_json::from_str(&s).unwrap();
        assert_eq!(id, id2);
    }
}
