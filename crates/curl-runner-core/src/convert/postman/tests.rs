use super::*;
use std::path::Path;

fn fixture(name: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/postman")
            .join(name),
    )
    .unwrap()
}

#[test]
fn parses_the_sample() {
    let c = parse_collection(&fixture("sample.postman_collection.json")).unwrap();
    assert_eq!(c.item.len(), 2);
    assert!(c.variable.iter().any(|v| v.key == "apiKey" && v.secret));
    assert_eq!(c.auth.as_ref().unwrap().kind, "apikey");
}

#[test]
fn parses_edge_shapes() {
    parse_collection(&fixture("edge.postman_collection.json")).unwrap();
}

#[test]
fn rejects_what_isnt_a_v2_collection() {
    assert!(
        parse_collection("nope")
            .unwrap_err()
            .0
            .starts_with("Not a Postman collection: ")
    );
    assert_eq!(
        parse_collection("{}").unwrap_err().0,
        "Not a Postman collection: missing \"info\" or \"item\""
    );
    let v1 = r#"{"info":{"name":"x","schema":"https://schema.getpostman.com/json/collection/v1.0.0/collection.json"},"item":[]}"#;
    assert!(
        parse_collection(v1)
            .unwrap_err()
            .0
            .starts_with("Unsupported Postman collection schema: ")
    );
}

#[test]
fn variable_debug_hides_the_value() {
    let c = parse_collection(&fixture("sample.postman_collection.json")).unwrap();
    assert!(!format!("{:?}", c.variable).contains("test-key-123"));
}

#[test]
fn reads_items_bodies_and_auth_params() {
    let c = parse_collection(&fixture("sample.postman_collection.json")).unwrap();
    let Item::Folder(f) = &c.item[1] else {
        panic!("folder")
    };
    let Item::Request(r) = &f.item[0] else {
        panic!("request")
    };
    let body = r.request.body.as_ref().unwrap();
    assert_eq!(body.mode, "formdata");
    assert_eq!(body.formdata.len(), 5);
    assert_eq!(body.formdata[0].kind.as_deref(), Some("file"));
    assert!(body.formdata[1].src.is_some());
    assert!(body.formdata[3].disabled);
    assert_eq!(
        body.formdata[3].description.as_ref().unwrap().text(),
        "Stamp text across each page."
    );
    let params = c.auth.as_ref().unwrap().params();
    assert!(params.contains(&("key".to_string(), "X-Api-Key".to_string())));
}

#[test]
fn reads_edge_specifics() {
    let c = parse_collection(&fixture("edge.postman_collection.json")).unwrap();
    let Item::Folder(reqs) = &c.item[1] else {
        panic!("folder")
    };
    let Item::Request(plain) = &reqs.item[0] else {
        panic!("request")
    };
    assert!(matches!(plain.request.url, Url::Raw(_)));
    let Item::Request(scripted) = &reqs.item[3] else {
        panic!("request")
    };
    assert_eq!(scripted.event[0].listen, "prerequest");
    assert_eq!(scripted.response.len(), 1);
    let Item::Request(item) = &reqs.item[1] else {
        panic!("request")
    };
    let Url::Object { variable, .. } = &item.request.url else {
        panic!("object url")
    };
    assert_eq!(variable[0].value.as_deref(), Some("42"));
}
