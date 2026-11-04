use serde_json::json;

use super::*;

const YAML: &str = "openapi: 3.0.0\ninfo:\n  title: T\n  version: '1'\npaths:\n  /a:\n    get:\n      responses:\n        200:\n          description: ok\n";
const JSON: &str = r#"{"openapi":"3.0.0","info":{"title":"T","version":"1"},"paths":{"/a":{"get":{"responses":{"200":{"description":"ok"}}}}}}"#;

fn doc(v: serde_json::Value) -> Doc {
    parse_document(&v.to_string()).unwrap()
}

#[test]
fn parses_yaml_and_json() {
    let y = parse_document(YAML).unwrap();
    let j = parse_document(JSON).unwrap();
    assert_eq!(y.root, j.root);
    assert_eq!(y.version, SpecVersion::OpenApi30);
}

#[test]
fn yaml_numeric_keys_become_strings() {
    let y = parse_document(YAML).unwrap();
    assert!(y.root["paths"]["/a"]["get"]["responses"]["200"].is_object());
}

#[test]
fn rejects_non_openapi() {
    let e = parse_document(r#"{"info":{},"item":[]}"#).unwrap_err();
    assert_eq!(
        e.0,
        "Not an OpenAPI document: no \"openapi\" or \"swagger\" field"
    );
}

#[test]
fn rejects_openapi_2_5() {
    let e = parse_document(r#"{"openapi":"2.5"}"#).unwrap_err();
    assert_eq!(
        e.0,
        "Unsupported OpenAPI version: 2.5. Use OpenAPI 3.0, 3.1 or Swagger 2.0."
    );
}

#[test]
fn rejects_garbage() {
    let e = parse_document("{not json").unwrap_err();
    assert!(e.0.starts_with("Not an OpenAPI document: "));
}

#[test]
fn accepts_31_and_swagger_number() {
    assert_eq!(
        parse_document(r#"{"openapi":"3.1.0"}"#).unwrap().version,
        SpecVersion::OpenApi31
    );
    assert_eq!(
        parse_document("swagger: 2.0\n").unwrap().version,
        SpecVersion::Swagger2
    );
}

#[test]
fn resolves_escaped_pointer() {
    let d = doc(json!({
        "openapi": "3.0.0",
        "components": { "schemas": { "a/b": { "type": "string" }, "c~d": { "type": "integer" } } }
    }));
    let r = json!({ "$ref": "#/components/schemas/a~1b" });
    assert_eq!(d.resolve(&r), Resolved::Value(&json!({ "type": "string" })));
    let r = json!({ "$ref": "#/components/schemas/c~0d" });
    assert_eq!(
        d.resolve(&r),
        Resolved::Value(&json!({ "type": "integer" }))
    );
    let r = json!({ "$ref": "#/components/schemas/a%2Fb" });
    assert_eq!(d.resolve(&r), Resolved::Value(&json!({ "type": "string" })));
}

#[test]
fn ref_cycle_is_missing_not_hang() {
    let d = doc(json!({
        "openapi": "3.0.0",
        "components": { "schemas": {
            "A": { "$ref": "#/components/schemas/B" },
            "B": { "$ref": "#/components/schemas/A" }
        } }
    }));
    let r = json!({ "$ref": "#/components/schemas/A" });
    assert!(matches!(d.resolve(&r), Resolved::Missing(_)));
}

#[test]
fn missing_and_plain_values() {
    let d = doc(json!({ "openapi": "3.0.0" }));
    let r = json!({ "$ref": "#/nope" });
    assert_eq!(d.resolve(&r), Resolved::Missing("#/nope".into()));
    let plain = json!({ "type": "string" });
    assert_eq!(d.resolve(&plain), Resolved::Value(&plain));
}

#[test]
fn external_ref() {
    let d = doc(json!({ "openapi": "3.0.0" }));
    let r = json!({ "$ref": "other.yaml#/X" });
    assert_eq!(d.resolve(&r), Resolved::External("other.yaml#/X".into()));
}

#[test]
fn swagger2_normalizes() {
    let d = doc(json!({
        "swagger": "2.0",
        "info": { "title": "T", "version": "1" },
        "host": "api.example.com",
        "basePath": "/v1",
        "schemes": ["http"],
        "paths": {
            "/pets": {
                "post": {
                    "parameters": [
                        { "in": "body", "name": "b", "required": true,
                          "schema": { "$ref": "#/definitions/Pet" } },
                        { "in": "query", "name": "limit", "type": "integer" }
                    ],
                    "responses": { "200": { "description": "ok", "schema": { "$ref": "#/definitions/Pet" } } }
                }
            },
            "/upload": {
                "post": {
                    "parameters": [
                        { "in": "formData", "name": "file", "type": "file", "required": true },
                        { "in": "formData", "name": "note", "type": "string" }
                    ],
                    "responses": {}
                }
            },
            "/form": {
                "post": {
                    "parameters": [ { "in": "formData", "name": "a", "type": "string" } ],
                    "responses": {}
                }
            }
        },
        "definitions": { "Pet": { "type": "object" } },
        "securityDefinitions": { "b": { "type": "basic" } }
    }));
    assert_eq!(d.version, SpecVersion::Swagger2);
    let r = &d.root;
    assert_eq!(r["servers"][0]["url"], "http://api.example.com/v1");
    let post = &r["paths"]["/pets"]["post"];
    assert_eq!(
        post["requestBody"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/Pet"
    );
    assert_eq!(post["parameters"].as_array().unwrap().len(), 1);
    assert_eq!(post["parameters"][0]["schema"]["type"], "integer");
    assert_eq!(
        post["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/Pet"
    );
    let up =
        &r["paths"]["/upload"]["post"]["requestBody"]["content"]["multipart/form-data"]["schema"];
    assert_eq!(
        up["properties"]["file"],
        json!({ "type": "string", "format": "binary" })
    );
    assert_eq!(up["required"], json!(["file"]));
    assert!(
        r["paths"]["/form"]["post"]["requestBody"]["content"]
            .get("application/x-www-form-urlencoded")
            .is_some()
    );
    assert_eq!(
        r["components"]["securitySchemes"]["b"],
        json!({ "type": "http", "scheme": "basic" })
    );
    assert!(r["components"]["schemas"]["Pet"].is_object());
}

#[test]
fn swagger2_consumes_and_no_host() {
    let d = doc(json!({
        "swagger": "2.0",
        "basePath": "/v1",
        "consumes": ["application/xml"],
        "paths": { "/x": { "post": { "parameters": [
            { "in": "body", "name": "b", "schema": { "type": "string" } }
        ] } } }
    }));
    assert_eq!(d.root["servers"][0]["url"], "/v1");
    assert!(
        d.root["paths"]["/x"]["post"]["requestBody"]["content"]
            .get("application/xml")
            .is_some()
    );
}

#[test]
fn swagger2_ref_parameters() {
    let d = doc(json!({
        "swagger": "2.0",
        "paths": { "/x": { "post": { "parameters": [
            { "$ref": "#/parameters/B" }, { "$ref": "#/parameters/Q" }
        ] } } },
        "parameters": {
            "B": { "in": "body", "name": "b", "schema": { "type": "string" } },
            "Q": { "in": "query", "name": "q", "type": "string" }
        }
    }));
    let op = &d.root["paths"]["/x"]["post"];
    assert!(op["requestBody"]["content"]["application/json"].is_object());
    assert_eq!(op["parameters"][0]["$ref"], "#/components/parameters/Q");
    assert_eq!(
        d.root["components"]["parameters"]["Q"]["schema"]["type"],
        "string"
    );
}

#[test]
fn swagger2_oauth2_becomes_flows() {
    let d = doc(json!({
        "swagger": "2.0",
        "securityDefinitions": { "o": {
            "type": "oauth2", "flow": "accessCode",
            "authorizationUrl": "https://auth.example.com/a",
            "tokenUrl": "https://auth.example.com/t",
            "scopes": { "read": "Read" }
        } }
    }));
    let o = &d.root["components"]["securitySchemes"]["o"];
    assert_eq!(o["type"], "oauth2");
    let f = &o["flows"]["authorizationCode"];
    assert_eq!(f["tokenUrl"], "https://auth.example.com/t");
    assert_eq!(f["scopes"]["read"], "Read");
}

#[test]
fn swagger2_response_uses_produces() {
    let d = doc(json!({
        "swagger": "2.0",
        "produces": ["application/xml"],
        "paths": { "/x": {
            "get": { "responses": { "200": { "description": "ok", "schema": { "type": "string" } } } },
            "post": { "produces": ["text/csv"],
                "responses": { "200": { "description": "ok", "schema": { "type": "string" } } } }
        } }
    }));
    let p = &d.root["paths"]["/x"];
    assert!(p["get"]["responses"]["200"]["content"]["application/xml"].is_object());
    assert!(p["post"]["responses"]["200"]["content"]["text/csv"].is_object());
}

#[test]
fn swagger2_path_level_params_merge_into_operations() {
    let d = doc(json!({
        "swagger": "2.0",
        "paths": { "/x/{id}": {
            "parameters": [
                { "in": "body", "name": "b", "schema": { "type": "string" } },
                { "in": "path", "name": "id", "required": true, "type": "string" },
                { "in": "query", "name": "q", "type": "string", "description": "path" }
            ],
            "get": { "parameters": [
                { "in": "query", "name": "q", "type": "integer", "description": "op" }
            ] },
            "put": {}
        } }
    }));
    let item = &d.root["paths"]["/x/{id}"];
    assert!(item.get("parameters").is_none());
    for m in ["get", "put"] {
        assert!(
            item[m]["requestBody"]["content"]["application/json"]["schema"].is_object(),
            "{m}"
        );
    }
    let get = item["get"]["parameters"].as_array().unwrap();
    assert_eq!(get.len(), 2);
    let q = get.iter().find(|p| p["name"] == "q").unwrap();
    assert_eq!(q["description"], "op");
    assert_eq!(item["put"]["parameters"].as_array().unwrap().len(), 2);
}

#[test]
fn swagger2_collection_formats() {
    let param = |cf: Option<&str>| {
        let mut p =
            json!({ "in": "query", "name": "q", "type": "array", "items": { "type": "string" } });
        if let Some(c) = cf {
            p["collectionFormat"] = json!(c);
        }
        let d = doc(json!({ "swagger": "2.0",
            "paths": { "/x": { "get": { "parameters": [p] } } } }));
        d.root["paths"]["/x"]["get"]["parameters"][0].clone()
    };
    let csv = param(Some("csv"));
    assert_eq!(
        (csv["style"].clone(), csv["explode"].clone()),
        (json!("form"), json!(false))
    );
    let multi = param(Some("multi"));
    assert_eq!(
        (multi["style"].clone(), multi["explode"].clone()),
        (json!("form"), json!(true))
    );
    assert_eq!(param(Some("ssv"))["style"], "spaceDelimited");
    assert_eq!(param(Some("pipes"))["style"], "pipeDelimited");
    let tsv = param(Some("tsv"));
    assert_eq!(tsv["x-collectionFormat"], "tsv");
    assert!(tsv.get("style").is_none());
    assert!(param(None).get("style").is_none());
    assert!(csv.get("collectionFormat").is_none());
}

#[test]
fn yaml_merge_keys_apply() {
    let y = "openapi: 3.0.0\nx-base: &base\n  description: shared\n  type: string\ncomponents:\n  schemas:\n    A:\n      <<: *base\n      type: integer\n";
    let d = parse_document(y).unwrap();
    let a = &d.root["components"]["schemas"]["A"];
    assert_eq!(a["description"], "shared");
    assert_eq!(a["type"], "integer");
    assert!(a.get("<<").is_none());
}

mod sample {
    use serde_json::{Value, json};

    use super::doc;
    use crate::convert::openapi::sample::{Sampler, UnresolvedRef};

    fn empty_doc() -> crate::convert::openapi::Doc {
        doc(json!({"openapi": "3.0.0", "info": {"title": "T", "version": "1"}, "paths": {}}))
    }

    fn sample(schema: Value) -> Value {
        let d = empty_doc();
        Sampler::new(&d).value(&schema)
    }

    fn leaves(v: &Value) -> usize {
        match v {
            Value::Object(m) if !m.is_empty() => m.values().map(leaves).sum(),
            Value::Array(a) if !a.is_empty() => a.iter().map(leaves).sum(),
            _ => 1,
        }
    }

    #[test]
    fn uses_example_before_schema() {
        let d = empty_doc();
        let mut s = Sampler::new(&d);
        let media = json!({"example": {"a": 1}, "schema": {"type": "string", "example": "x"}});
        assert_eq!(s.example(&media), Some(json!({"a": 1})));
        let media = json!({"examples": {"one": {"value": 7}}, "schema": {"type": "string"}});
        assert_eq!(s.example(&media), Some(json!(7)));
        assert_eq!(s.example(&json!({"schema": {}})), None);
        assert_eq!(
            s.value(&json!({"type": "string", "example": "x"})),
            json!("x")
        );
    }

    #[test]
    fn literal_order() {
        assert_eq!(
            sample(json!({"type": "string", "default": "d", "enum": ["e"]})),
            json!("d")
        );
        assert_eq!(
            sample(json!({"type": "string", "enum": ["e", "f"]})),
            json!("e")
        );
        assert_eq!(sample(json!({"const": 3})), json!(3));
        assert_eq!(
            sample(json!({"type": "string", "examples": ["a", "b"]})),
            json!("a")
        );
    }

    #[test]
    fn formats_are_fixed() {
        let v = sample(json!({"type": "object", "properties": {
            "id": {"type": "integer"},
            "at": {"type": "string", "format": "date-time"},
            "mail": {"type": "string", "format": "email"}
        }}));
        assert_eq!(
            v,
            json!({"id": 1, "at": "2024-01-01T00:00:00Z", "mail": "user@example.com"})
        );
        for (f, want) in [
            ("date", "2024-01-01"),
            ("uuid", "00000000-0000-4000-8000-000000000000"),
            ("uri", "https://example.com"),
            ("binary", "./pick-a-file"),
            ("other", "string"),
        ] {
            assert_eq!(sample(json!({"type": "string", "format": f})), json!(want));
        }
        assert_eq!(sample(json!({"type": "boolean"})), json!(true));
    }

    #[test]
    fn minimum_wins() {
        assert_eq!(sample(json!({"type": "integer", "minimum": 5})), json!(5));
    }

    #[test]
    fn nullable_31() {
        assert_eq!(sample(json!({"type": ["string", "null"]})), json!("string"));
    }

    #[test]
    fn all_of_merges() {
        let v = sample(json!({"allOf": [
            {"type": "object", "properties": {"a": {"type": "integer"}}},
            {"type": "object", "properties": {"b": {"type": "boolean"}}}
        ]}));
        assert_eq!(v, json!({"a": 1, "b": true}));
        assert_eq!(serde_json::to_string(&v).unwrap(), r#"{"a":1,"b":true}"#);
    }

    #[test]
    fn one_of_first() {
        let schema = json!({"oneOf": [{"type": "integer"}, {"type": "string"}]});
        let d = empty_doc();
        let mut s = Sampler::new(&d);
        assert_eq!(s.value(&schema), json!(1));
        assert_eq!(s.alternatives(&schema), 2);
        assert_eq!(s.alternatives(&json!({"type": "string"})), 0);
    }

    fn with_schemas(schemas: Value) -> crate::convert::openapi::Doc {
        doc(
            json!({"openapi": "3.0.0", "info": {"title": "T", "version": "1"},
            "paths": {}, "components": {"schemas": schemas}}),
        )
    }

    #[test]
    fn recursive_ref_stops() {
        let d = with_schemas(json!({"Node": {"type": "object", "properties": {
            "name": {"type": "string"},
            "children": {"type": "array", "items": {"$ref": "#/components/schemas/Node"}}
        }}}));
        let v = Sampler::new(&d).value(&json!({"$ref": "#/components/schemas/Node"}));
        assert_eq!(v, json!({"name": "string", "children": [{}]}));
    }

    #[test]
    fn wide_graph_is_budgeted() {
        let mut schemas = serde_json::Map::new();
        for i in 0..40 {
            let props: serde_json::Map<String, Value> = (0..10)
                .map(|j| {
                    (
                        format!("p{j}"),
                        json!({"$ref": format!("#/components/schemas/S{}", (i + j + 1) % 40)}),
                    )
                })
                .collect();
            schemas.insert(
                format!("S{i}"),
                json!({"type": "object", "properties": props}),
            );
        }
        let d = with_schemas(Value::Object(schemas));
        let start = std::time::Instant::now();
        let v = Sampler::new(&d).value(&json!({"$ref": "#/components/schemas/S0"}));
        assert!(start.elapsed() < std::time::Duration::from_millis(50));
        assert!(leaves(&v) <= 400, "{} leaves", leaves(&v));
    }

    #[test]
    fn wide_required_graph_is_capped() {
        // Past the cap, open objects still give their remaining required
        // siblings an empty value, so the bound is cap + fan-out * depth.
        let mut schemas = serde_json::Map::new();
        for i in 0..40 {
            let props: serde_json::Map<String, Value> = (0..10)
                .map(|j| {
                    (
                        format!("p{j}"),
                        json!({"$ref": format!("#/components/schemas/S{}", (i + j + 1) % 40)}),
                    )
                })
                .collect();
            let required: Vec<String> = props.keys().cloned().collect();
            schemas.insert(
                format!("S{i}"),
                json!({"type": "object", "properties": props, "required": required}),
            );
        }
        let d = with_schemas(Value::Object(schemas));
        let start = std::time::Instant::now();
        let v = Sampler::new(&d).value(&json!({"$ref": "#/components/schemas/S0"}));
        assert!(start.elapsed() < std::time::Duration::from_millis(50));
        assert!(leaves(&v) <= 400 + 10 * 11, "{} leaves", leaves(&v));
    }

    #[test]
    fn missing_ref_is_empty() {
        let d = empty_doc();
        let mut s = Sampler::new(&d);
        assert_eq!(
            s.value(&json!({"$ref": "#/components/schemas/Nope"})),
            json!({})
        );
        assert_eq!(s.value(&json!({"$ref": "other.yaml#/X"})), json!({}));
        s.value(&json!({"$ref": "#/components/schemas/Nope"}));
        assert_eq!(
            s.unresolved_refs(),
            [
                UnresolvedRef {
                    target: "#/components/schemas/Nope".into(),
                    external: false
                },
                UnresolvedRef {
                    target: "other.yaml#/X".into(),
                    external: true
                },
            ]
        );
    }
}
