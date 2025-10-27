use junction_rs::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Node)]
#[junction(table = "me_person")]
struct MePerson {
    id: SimpleId<MePerson>,
    name: String,
}

// Edge with default endpoint field names (r#in / r#out)
#[derive(Debug, Clone, Serialize, Deserialize, Edge)]
#[junction(table = "me_follows_default")]
struct FollowsDefault {
    #[junction(source)]
    r#in: SimpleId<MePerson>,
    #[junction(target)]
    r#out: SimpleId<MePerson>,
    since: i32,
}

// Edge with custom endpoint names (src / dst)
#[derive(Debug, Clone, Serialize, Deserialize, Edge)]
#[junction(table = "me_follows_custom")]
struct FollowsCustom {
    #[junction(source)]
    src: SimpleId<MePerson>,
    #[junction(target)]
    dst: SimpleId<MePerson>,
    strength: i32,
}

async fn setup_adapter() -> SurrealDbAdapter {
    use surrealdb::engine::any::connect;
    let db = connect("mem://").await.unwrap();
    db.use_ns("test").use_db("test").await.unwrap();
    SurrealDbAdapter::new(db)
}

#[tokio::test(flavor = "current_thread")]
async fn multi_edge_insert_with_default_and_custom_endpoints() {
    let db = setup_adapter().await;
    // Create people
    let p1 = MePerson {
        id: MePerson::create_simple_id(),
        name: "Alice".into(),
    };
    let p2 = MePerson {
        id: MePerson::create_simple_id(),
        name: "Bob".into(),
    };
    let p3 = MePerson {
        id: MePerson::create_simple_id(),
        name: "Carol".into(),
    };
    let p4 = MePerson {
        id: MePerson::create_simple_id(),
        name: "Dave".into(),
    };
    insert(vec![p1.clone(), p2.clone(), p3.clone(), p4.clone()])
        .return_many::<_>(db.clone())
        .await
        .unwrap();

    // Insert multiple default-endpoint edges using generic insert (no metadata needed)
    let def_edges = vec![
        FollowsDefault {
            r#in: p1.id.clone(),
            r#out: p2.id.clone(),
            since: 2021,
        },
        FollowsDefault {
            r#in: p2.id.clone(),
            r#out: p3.id.clone(),
            since: 2022,
        },
        FollowsDefault {
            r#in: p3.id.clone(),
            r#out: p4.id.clone(),
            since: 2023,
        },
    ];
    let def_inserted = insert(def_edges)
        .return_many::<_>(db.clone())
        .await
        .unwrap();
    assert_eq!(def_inserted.len(), 3);

    // Insert multiple custom-endpoint edges via unified insert so metadata drives mapping
    let custom_edges = vec![
        FollowsCustom {
            src: p1.id.clone(),
            dst: p3.id.clone(),
            strength: 5,
        },
        FollowsCustom {
            src: p2.id.clone(),
            dst: p4.id.clone(),
            strength: 7,
        },
    ];
    let custom_inserted = insert(custom_edges.clone())
        .return_many::<_>(db.clone())
        .await
        .unwrap();
    assert_eq!(custom_inserted.len(), 2);

    // Raw query to ensure Surreal stored in/out and that they match src/dst semantics
    let raw = db
        .debug_select_json(&format!(
            "SELECT * FROM {} ORDER BY strength ASC",
            FollowsCustom::TABLE
        ))
        .await
        .unwrap();
    // Flatten similar to adapter logic (minimal subset for this test)
    fn flat(v: &serde_json::Value) -> serde_json::Value {
        if let Some(obj) = v.as_object() {
            if let Some(arr) = obj.get("Array").and_then(|a| a.as_array()) {
                return serde_json::Value::Array(arr.iter().map(flat).collect());
            }
            if let Some(thing) = obj.get("Thing").and_then(|t| t.as_object()) {
                if let (Some(tb), Some(id)) = (thing.get("tb"), thing.get("id")) {
                    if let (Some(tb_s), Some(id_obj)) = (tb.as_str(), id.as_object()) {
                        if let Some(id_s) = id_obj.get("String").and_then(|s| s.as_str()) {
                            return serde_json::json!(format!("{}:{}", tb_s, id_s));
                        }
                    }
                }
            }
            if let Some(strand) = obj.get("Strand").and_then(|s| s.as_str()) {
                return serde_json::Value::String(strand.to_string());
            }
            if let Some(num) = obj.get("Number").and_then(|n| n.as_object()) {
                if let Some(int_v) = num.get("Int").and_then(|i| i.as_i64()) {
                    return serde_json::Value::Number(int_v.into());
                }
            }
            // Object wrapper
            if let Some(inner) = obj.get("Object").and_then(|o| o.as_object()) {
                let mut m = serde_json::Map::new();
                for (k, v) in inner.iter() {
                    m.insert(k.clone(), flat(v));
                }
                return serde_json::Value::Object(m);
            }
        }
        v.clone()
    }
    let flattened = flat(&raw);
    let arr = flattened
        .as_array()
        .unwrap_or_else(|| panic!("expected array after flatten, got {:?}", flattened));
    assert_eq!(arr.len(), 2);
    // Strength ascending: first edge strength 5 (p1 -> p3)
    let first = &arr[0];
    let second = &arr[1];
    let first_in = first.get("in").and_then(|v| v.as_str()).unwrap_or("");
    let first_out = first.get("out").and_then(|v| v.as_str()).unwrap_or("");
    fn last_segment(s: &str) -> &str {
        s.split(':').last().unwrap_or(s)
    }
    assert_eq!(last_segment(first_in), p1.id.as_uuid_str());
    assert_eq!(last_segment(first_out), p3.id.as_uuid_str());
    let second_in = second.get("in").and_then(|v| v.as_str()).unwrap_or("");
    let second_out = second.get("out").and_then(|v| v.as_str()).unwrap_or("");
    assert_eq!(last_segment(second_in), p2.id.as_uuid_str());
    assert_eq!(last_segment(second_out), p4.id.as_uuid_str());

    // Traversal check: from p1 outward along custom edge should reach p3
    let hop: Vec<MePerson> = select::<MePerson>(All)
        .record_id(&p1.id)
        .forward::<FollowsCustom, MePerson>()
        .return_many(db.clone())
        .await
        .unwrap();
    assert!(hop
        .iter()
        .any(|n| n.id.as_uuid_str() == p3.id.as_uuid_str()));
}
