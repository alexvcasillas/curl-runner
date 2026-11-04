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

    #[test]
    fn take_hits_reports_each_call_again() {
        let d = empty_doc();
        let mut s = Sampler::new(&d);
        let nope = json!({"$ref": "#/components/schemas/Nope"});
        s.value(&json!({"type": "array", "items": nope}));
        s.value(&nope);
        assert_eq!(s.take_hits().len(), 1);
        assert!(s.take_hits().is_empty());
        s.value(&nope);
        assert_eq!(s.take_hits()[0].target, "#/components/schemas/Nope");
        assert_eq!(s.unresolved_refs().len(), 1);
    }
}

mod map {
    use serde_json::{Value, json};

    use super::doc;
    use crate::convert::import::{Attention, Field, FieldValue, Mapped, MappedAuth, MappedBody};
    use crate::convert::openapi::{api_prefix, map_document, parse_document};

    const SMALL: &str = include_str!("../../../tests/fixtures/openapi/small_v30.yaml");
    const V2: &str = include_str!("../../../tests/fixtures/openapi/petstore_v2.json");

    fn small() -> Mapped {
        map_document(&parse_document(SMALL).unwrap())
    }

    /// A 3.0 document titled `Pets` on `https://api.example.com`.
    fn spec(paths: Value) -> Value {
        json!({
            "openapi": "3.0.3",
            "info": { "title": "Pets", "version": "1" },
            "servers": [{ "url": "https://api.example.com" }],
            "paths": paths
        })
    }

    fn map(v: Value) -> Mapped {
        map_document(&doc(v))
    }

    fn lines(m: &Mapped) -> Vec<String> {
        m.report.attention.iter().map(ToString::to_string).collect()
    }

    fn field(key: &str, value: &str, disabled: bool, note: Option<&str>) -> Field {
        Field {
            key: key.into(),
            value: FieldValue::Text(value.into()),
            disabled,
            note: note.map(Into::into),
        }
    }

    fn var<'m>(m: &'m Mapped, name: &str) -> &'m crate::convert::import::ImportedVar {
        m.variables
            .iter()
            .find(|v| v.name == name)
            .unwrap_or_else(|| panic!("no {name} in {:?}", m.variables))
    }

    #[test]
    fn maps_small_spec() {
        let m = small();
        let folders: Vec<Vec<String>> = m.requests.iter().map(|r| r.folders.clone()).collect();
        assert_eq!(
            folders,
            [["pets"], ["pets"], ["store"], ["pets"], ["pets"], ["users"]]
                .map(|f| vec![f[0].to_string()])
        );
        assert_eq!((m.report.requests, m.report.folders), (6, 3));
        let r = &m.requests[0];
        assert_eq!(r.name, "List pets");
        assert_eq!(r.method, "GET");
        assert_eq!(r.url, "${PETS_BASE_URL}/pets");
        assert_eq!(r.expect_status, Some(200));
        assert_eq!(
            r.params,
            [field("limit", "1", true, Some("How many pets to return."))]
        );
        assert_eq!(
            r.headers,
            [field("X-API-Key", "${PETS_API_KEY}", false, None)]
        );
        assert_eq!(
            r.comments,
            ["operationId: listPets", "Everything about pets."]
        );
        assert_eq!(m.requests[1].expect_status, Some(201));
        assert_eq!(m.requests[3].expect_status, Some(200));
        // `2XX`
        assert_eq!(m.requests[5].expect_status, Some(200));
        assert_eq!(
            m.requests[5].file_vars,
            [("username".to_string(), "alice".to_string())]
        );

        let base = var(&m, "PETS_BASE_URL");
        assert_eq!(
            (base.value.as_str(), base.secret),
            ("https://api.example.com/v1", false)
        );
        let key = var(&m, "PETS_API_KEY");
        assert_eq!((key.value.as_str(), key.secret), ("", true));
        assert_eq!(
            m.env_comments,
            [(
                "PETS_BASE_URL".to_string(),
                "Other server: https://staging.example.com/v1".to_string()
            )]
        );
        assert_eq!(m.description.as_deref(), Some("A small pet store API."));
        assert_eq!(
            lines(&m),
            [
                "Pick a file for \"file\" in pets/Upload photo",
                "Set the secret PETS_API_KEY"
            ]
        );
    }

    #[test]
    fn path_params_in_file_vars() {
        let m = small();
        let r = &m.requests[3];
        assert_eq!(r.name, "Get pet");
        assert_eq!(r.url, "${PETS_BASE_URL}/pets/${petId}");
        assert_eq!(r.file_vars, [("petId".to_string(), "1".to_string())]);
    }

    #[test]
    fn json_body_and_content_type() {
        let m = small();
        let r = &m.requests[1];
        assert_eq!(r.method, "POST");
        assert_eq!(
            r.body,
            MappedBody::Json(json!({ "id": 1, "name": "Rex", "tag": "string" }))
        );
        assert!(
            r.headers
                .contains(&field("Content-Type", "application/json", false, None)),
            "{:?}",
            r.headers
        );
    }

    #[test]
    fn multipart_file_field() {
        let m = small();
        let r = &m.requests[4];
        assert_eq!(
            r.body,
            MappedBody::Form(vec![
                Field {
                    key: "file".into(),
                    value: FieldValue::File {
                        path: "./pick-a-file".into(),
                        placeholder: true
                    },
                    disabled: false,
                    note: None,
                },
                field("caption", "string", true, Some("Shown under the photo.")),
            ])
        );
        assert!(
            !r.headers
                .iter()
                .any(|h| h.key.eq_ignore_ascii_case("content-type"))
        );
        assert!(m.report.attention.contains(&Attention::PickFile {
            request: "pets/Upload photo".into(),
            field: "file".into()
        }));
    }

    #[test]
    fn content_media_type_is_a_file_field() {
        let mut v = spec(json!({ "/up": { "post": {
            "requestBody": { "content": { "multipart/form-data": { "schema": {
                "type": "object",
                "properties": { "doc": { "type": "string", "contentMediaType": "application/pdf" } }
            } } } },
            "responses": {}
        } } }));
        v["openapi"] = json!("3.1.0");
        let m = map(v);
        let MappedBody::Form(f) = &m.requests[0].body else {
            panic!("{:?}", m.requests[0].body)
        };
        assert!(matches!(
            f[0].value,
            FieldValue::File {
                placeholder: true,
                ..
            }
        ));
        // Optional, so there's nothing to pick yet.
        assert!(f[0].disabled);
    }

    #[test]
    fn urlencoded_required_only() {
        let m = map(spec(json!({ "/login": { "post": {
            "requestBody": { "content": { "application/x-www-form-urlencoded": { "schema": {
                "type": "object",
                "required": ["user name"],
                "properties": {
                    "user name": { "type": "string", "example": "a&b" },
                    "remember": { "type": "boolean" },
                    "next": { "type": "string" }
                }
            } } } },
            "responses": { "204": { "description": "ok" } }
        } } })));
        let r = &m.requests[0];
        assert_eq!(r.body, MappedBody::Text("user+name=a%26b".into()));
        assert!(
            r.comments
                .contains(&"Optional form fields: remember, next".to_string())
        );
        assert!(r.headers.contains(&field(
            "Content-Type",
            "application/x-www-form-urlencoded",
            false,
            None
        )));
        assert_eq!(r.expect_status, Some(204));
    }

    #[test]
    fn text_body_uses_example() {
        let m = map(spec(json!({ "/x": { "put": {
            "requestBody": { "content": {
                "application/octet-stream": {},
                "application/xml": { "example": "<a/>" }
            } },
            "responses": {}
        } } })));
        let r = &m.requests[0];
        assert_eq!(r.body, MappedBody::Text("<a/>".into()));
        assert!(
            r.headers
                .contains(&field("Content-Type", "application/xml", false, None))
        );
        assert!(m.report.attention.is_empty(), "{:?}", lines(&m));
    }

    #[test]
    fn octet_stream_skipped() {
        let op = json!({ "put": {
            "requestBody": { "content": { "application/octet-stream": {} } },
            "responses": {}
        } });
        let m = map(spec(json!({ "/a": op, "/b": op })));
        assert_eq!(m.requests.len(), 2);
        assert_eq!(m.requests[0].body, MappedBody::None);
        assert_eq!(
            m.report.attention,
            [Attention::SkippedBody {
                media: "application/octet-stream".into(),
                count: 2,
                examples: vec!["PUT /a".into(), "PUT /b".into()],
            }]
        );
        assert_eq!(
            lines(&m),
            [
                "Skipped application/octet-stream bodies in 2 requests, like PUT /a and PUT /b. Add them by hand."
            ]
        );
    }

    #[test]
    fn auth_rows() {
        let op = |scheme: &str| json!({ "get": { "security": [{ scheme: [] }], "responses": {} } });
        let mut v = spec(json!({
            "/h": op("hk"),
            "/q": op("qk"),
            "/c": op("ck"),
            "/b": op("bearerAuth"),
            "/basic": op("basicAuth"),
            "/o": op("oauth"),
            "/oidc": op("oidc"),
            "/d": op("dig"),
            "/none": { "get": { "security": [], "responses": {} } },
            "/default": { "get": { "responses": {} } }
        }));
        v["security"] = json!([{ "hk": [] }]);
        v["components"] = json!({ "securitySchemes": {
            "hk": { "type": "apiKey", "in": "header", "name": "X-Key" },
            "qk": { "type": "apiKey", "in": "query", "name": "key" },
            "ck": { "type": "apiKey", "in": "cookie", "name": "sid" },
            "bearerAuth": { "type": "http", "scheme": "bearer" },
            "basicAuth": { "type": "http", "scheme": "Basic" },
            "oauth": { "type": "oauth2", "flows": {} },
            "oidc": { "type": "openIdConnect", "openIdConnectUrl": "https://api.example.com/.well-known" },
            "dig": { "type": "http", "scheme": "digest" }
        } });
        let m = map(v);
        let r = |i: usize| &m.requests[i];
        assert_eq!(r(0).headers, [field("X-Key", "${PETS_HK}", false, None)]);
        assert_eq!(r(1).params, [field("key", "${PETS_QK}", false, None)]);
        assert_eq!(
            r(2).headers,
            [field("Cookie", "sid=${PETS_CK}", false, None)]
        );
        assert_eq!(
            r(3).auth,
            Some(MappedAuth::Bearer("${PETS_BEARERAUTH_TOKEN}".into()))
        );
        assert_eq!(
            r(4).auth,
            Some(MappedAuth::Basic {
                username: "${PETS_BASICAUTH_USERNAME}".into(),
                password: "${PETS_BASICAUTH_PASSWORD}".into()
            })
        );
        assert_eq!(r(5).auth, Some(MappedAuth::Bearer("${PETS_OAUTH}".into())));
        assert_eq!(r(6).auth, Some(MappedAuth::Bearer("${PETS_OIDC}".into())));
        assert_eq!(r(7).auth, None);
        assert!(r(7).headers.is_empty());
        assert_eq!((r(8).auth.clone(), r(8).headers.len()), (None, 0));
        assert_eq!(r(9).headers, [field("X-Key", "${PETS_HK}", false, None)]);

        for (name, secret) in [
            ("PETS_HK", true),
            ("PETS_QK", true),
            ("PETS_CK", true),
            ("PETS_BEARERAUTH_TOKEN", true),
            ("PETS_BASICAUTH_USERNAME", false),
            ("PETS_BASICAUTH_PASSWORD", true),
            ("PETS_OAUTH", true),
            ("PETS_OIDC", true),
        ] {
            let v = var(&m, name);
            assert_eq!((v.secret, v.value.as_str()), (secret, ""), "{name}");
        }
        let a = &m.report.attention;
        assert!(a.contains(&Attention::AccessToken {
            var: "PETS_OAUTH".into(),
            kind: "OAuth 2".into()
        }));
        assert!(a.contains(&Attention::AccessToken {
            var: "PETS_OIDC".into(),
            kind: "OpenID Connect".into()
        }));
        assert!(a.contains(&Attention::UnsupportedScheme {
            scheme: "dig".into(),
            kind: "digest".into(),
            count: 1
        }));
        assert!(a.contains(&Attention::SetVariable {
            var: "PETS_BASICAUTH_USERNAME".into()
        }));
        assert!(a.contains(&Attention::SetSecret {
            var: "PETS_BASICAUTH_PASSWORD".into()
        }));
        // The access token line already says to set it.
        assert!(!a.contains(&Attention::SetSecret {
            var: "PETS_OAUTH".into()
        }));
        let l = lines(&m);
        assert!(
            l.contains(&"Put an OAuth 2 access token in PETS_OAUTH".to_string()),
            "{l:?}"
        );
        assert!(
            l.contains(&"Skipped digest auth (dig) in 1 request. Add it by hand.".to_string()),
            "{l:?}"
        );
    }

    #[test]
    fn auth_choice_aggregated() {
        let op = json!({ "get": { "security": [{ "k": [] }, { "o": [] }], "responses": {} } });
        let mut v = spec(json!({ "/1": op, "/2": op, "/3": op, "/4": op, "/5": op }));
        v["components"] = json!({ "securitySchemes": {
            "k": { "type": "apiKey", "in": "header", "name": "X-Key" },
            "o": { "type": "oauth2" }
        } });
        let m = map(v);
        let choices: Vec<_> = m
            .report
            .attention
            .iter()
            .filter(|a| matches!(a, Attention::AuthChoice { .. }))
            .collect();
        assert_eq!(
            choices,
            [&Attention::AuthChoice {
                used: "k".into(),
                others: "o".into(),
                count: 5
            }]
        );
        assert_eq!(
            choices[0].to_string(),
            "Used k auth in 5 requests that also accept o"
        );
    }

    #[test]
    fn literal_credential_scrubbed() {
        let m = map(spec(json!({ "/me": { "get": {
            "parameters": [
                { "name": "Authorization", "in": "header", "required": true,
                  "schema": { "type": "string" }, "example": "Bearer sk_live_x" },
                { "name": "api_key", "in": "query", "required": true,
                  "schema": { "type": "string", "example": "qk_live_y" } }
            ],
            "responses": {}
        } } })));
        let r = &m.requests[0];
        assert_eq!(
            r.headers,
            [field(
                "Authorization",
                "Bearer ${PETS_AUTHORIZATION}",
                false,
                None
            )]
        );
        assert_eq!(r.params, [field("api_key", "${PETS_API_KEY}", false, None)]);
        for name in ["PETS_AUTHORIZATION", "PETS_API_KEY"] {
            let v = var(&m, name);
            assert!(v.secret);
            assert_eq!(v.value, "");
        }
        let everything = format!(
            "{:?}{:?}{:?}{}",
            m.requests,
            m.variables.iter().map(|v| &v.value).collect::<Vec<_>>(),
            m.env_comments,
            lines(&m).join("\n")
        );
        for secret in ["sk_live_x", "qk_live_y"] {
            assert!(!everything.contains(secret), "{everything}");
        }
    }

    #[test]
    fn relative_server_needs_setting() {
        let mut v = spec(json!({ "/a": { "get": { "responses": {} } } }));
        v["servers"] = json!([{ "url": "/v1" }]);
        let m = map(v);
        assert_eq!(var(&m, "PETS_BASE_URL").value, "/v1");
        assert!(m.report.attention.contains(&Attention::SetVariable {
            var: "PETS_BASE_URL".into()
        }));

        let mut v = spec(json!({}));
        v.as_object_mut().unwrap().remove("servers");
        let m = map(v);
        assert_eq!(var(&m, "PETS_BASE_URL").value, "");
        assert_eq!(
            m.report.attention,
            [Attention::SetVariable {
                var: "PETS_BASE_URL".into()
            }]
        );
    }

    #[test]
    fn server_variables_default() {
        let mut v = spec(json!({}));
        v["servers"] = json!([{
            "url": "https://{region}.example.com/{version}",
            "variables": { "region": { "default": "eu" }, "version": { "enum": ["v1"] } }
        }]);
        let m = map(v);
        assert_eq!(
            var(&m, "PETS_BASE_URL").value,
            "https://eu.example.com/{version}"
        );
        assert_eq!(
            lines(&m),
            ["The server variable version has no default. Edit the URLs that use it."]
        );
    }

    #[test]
    fn bad_parts_dont_stop_others() {
        let m = map(spec(json!({
            "/broken": "oops",
            "/nores": { "get": { "summary": "No responses" } },
            "/badref": { "post": {
                "requestBody": { "$ref": "#/components/requestBodies/Nope" },
                "responses": { "200": { "description": "ok" } }
            } },
            "/ok": { "get": { "responses": { "200": { "description": "ok" } } } }
        })));
        let names: Vec<_> = m.requests.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["No responses", "POST /badref", "GET /ok"]);
        assert_eq!(m.requests[0].expect_status, None);
        assert_eq!(m.requests[1].body, MappedBody::None);
        assert_eq!(
            m.report.attention,
            [
                Attention::BadOperation {
                    at: "/broken".into(),
                    reason: "it isn't a path item object".into()
                },
                Attention::UnresolvedRef {
                    target: "#/components/requestBodies/Nope".into(),
                    external: false,
                    count: 1,
                    examples: vec!["POST /badref".into()]
                }
            ]
        );
        assert_eq!(
            lines(&m)[1],
            "Couldn't resolve #/components/requestBodies/Nope in 1 request, like POST /badref. Used an empty value."
        );
    }

    #[test]
    fn unresolved_refs_count_requests_and_keep_three_examples() {
        let op = json!({ "post": {
            "requestBody": { "content": { "application/json": { "schema": {
                "type": "object",
                "properties": {
                    "a": { "$ref": "#/components/schemas/Gone" },
                    "b": { "$ref": "#/components/schemas/Gone" }
                }
            } } } },
            "responses": {}
        } });
        let ext =
            json!({ "get": { "parameters": [{ "$ref": "other.yaml#/P" }], "responses": {} } });
        let m = map(spec(
            json!({ "/1": op, "/2": op, "/3": op, "/4": op, "/x": ext }),
        ));
        assert_eq!(
            m.report.attention,
            [
                Attention::UnresolvedRef {
                    target: "#/components/schemas/Gone".into(),
                    external: false,
                    count: 4,
                    examples: vec!["POST /1".into(), "POST /2".into(), "POST /3".into()]
                },
                Attention::UnresolvedRef {
                    target: "other.yaml#/P".into(),
                    external: true,
                    count: 1,
                    examples: vec!["GET /x".into()]
                }
            ]
        );
        assert_eq!(
            lines(&m)[0],
            "Couldn't resolve #/components/schemas/Gone in 4 requests, like POST /1, POST /2 and POST /3. Used an empty value."
        );
        assert_eq!(
            lines(&m)[1],
            "Can't follow the external reference other.yaml#/P in 1 request, like GET /x. Used an empty value."
        );
    }

    #[test]
    fn other_issues_aggregate() {
        let mut v = spec(json!({
            "/own": {
                "servers": [{ "url": "https://files.example.com" }],
                "get": { "responses": {} },
                "put": { "responses": {} }
            },
            "/same": {
                "servers": [{ "url": "https://api.example.com/" }],
                "get": { "responses": {} }
            },
            "/cookies": { "get": {
                "parameters": [
                    { "name": "session", "in": "cookie", "required": true, "schema": { "type": "string" } },
                    { "name": "theme", "in": "cookie", "schema": { "type": "string" } },
                    { "name": "ids", "in": "query", "required": true, "style": "pipeDelimited",
                      "schema": { "type": "array", "items": { "type": "integer" } } },
                    { "name": "tags", "in": "query", "required": true, "explode": false,
                      "schema": { "type": "array", "items": { "type": "string" } }, "example": ["a", "b"] },
                    { "name": "many", "in": "query", "required": true,
                      "schema": { "type": "array", "items": { "type": "string" } }, "example": ["a", "b"] }
                ],
                "callbacks": { "done": { "{$request.body#/url}": {} } },
                "responses": { "200": { "description": "ok", "links": { "self": {}, "next": {} } } }
            } },
            "/shapes": { "post": {
                "requestBody": { "content": { "application/json": { "schema": {
                    "oneOf": [{ "type": "object" }, { "type": "string" }]
                } } } },
                "responses": {}
            } }
        }));
        v["openapi"] = json!("3.1.0");
        v["webhooks"] = json!({ "newPet": {}, "oldPet": {} });
        let m = map(v);
        let urls: Vec<_> = m.requests.iter().map(|r| r.url.as_str()).collect();
        assert_eq!(
            urls,
            [
                "https://files.example.com/own",
                "https://files.example.com/own",
                "${PETS_BASE_URL}/same",
                "${PETS_BASE_URL}/cookies",
                "${PETS_BASE_URL}/shapes"
            ]
        );
        let c = &m.requests[3];
        assert_eq!(
            c.headers,
            [field("Cookie", "session=${PETS_SESSION}", false, None)]
        );
        assert_eq!(
            c.params,
            [
                field("ids", "1", false, None),
                field("tags", "a,b", false, None),
                field("many", "a,b", false, None)
            ]
        );
        assert_eq!(
            lines(&m),
            [
                "2 requests use their own server https://files.example.com",
                "Left out optional cookies in 1 request, like GET /cookies",
                "2 parameters use a style curl-runner doesn't model, like in GET /cookies. Check their values.",
                "Skipped 1 callback",
                "Skipped 2 links",
                "Used the first of several body shapes in 1 request, like POST /shapes",
                "Skipped 2 webhooks",
                "Set the secret PETS_SESSION",
            ]
        );
    }

    #[test]
    fn tsv_collection_format_is_a_param_style() {
        let d = doc(json!({ "swagger": "2.0", "info": { "title": "T" },
            "paths": { "/x": { "get": { "parameters": [{ "in": "query", "name": "q",
                "type": "array", "items": { "type": "string" }, "collectionFormat": "tsv" }] } } } }));
        let m = map_document(&d);
        assert!(
            matches!(
                m.report.attention.as_slice(),
                [
                    Attention::SetVariable { .. },
                    Attention::ParamStyle { count: 1, .. }
                ]
            ),
            "{:?}",
            m.report.attention
        );
    }

    #[test]
    fn names_and_comments() {
        let mut v = spec(json!({ "/pets/{id}": { "delete": {
            "tags": ["pets", "admin"],
            "operationId": "deletePet",
            "description": "Removes a pet.\n\nForever.",
            "deprecated": true,
            "externalDocs": { "url": "https://docs.example.com/delete" },
            "parameters": [{ "name": "id", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }],
            "responses": { "204": { "description": "gone" } }
        } } }));
        v["tags"] = json!([{ "name": "pets", "description": "Pet things." }]);
        let m = map(v);
        let r = &m.requests[0];
        assert_eq!(r.name, "deletePet");
        assert_eq!(r.method, "DELETE");
        assert_eq!(r.folders, ["pets"]);
        assert_eq!(
            r.comments,
            [
                "Removes a pet.",
                "Forever.",
                "operationId: deletePet",
                "Deprecated",
                "https://docs.example.com/delete",
                "Pet things."
            ]
        );
        assert_eq!(
            r.file_vars,
            [(
                "id".to_string(),
                "00000000-0000-4000-8000-000000000000".to_string()
            )]
        );
    }

    #[test]
    fn methods_in_order_and_renamed_path_vars() {
        let m = map(spec(json!({ "/a/{pet-id}": {
            "post": { "responses": {} },
            "get": { "responses": {} },
            "parameters": [{ "name": "pet-id", "in": "path", "required": true, "schema": { "type": "integer" } }]
        } })));
        let names: Vec<_> = m.requests.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["GET /a/{pet-id}", "POST /a/{pet-id}"]);
        assert_eq!(m.requests[0].url, "${PETS_BASE_URL}/a/${pet_id}");
        assert_eq!(
            m.requests[0].file_vars,
            [("pet_id".to_string(), "1".to_string())]
        );
        assert_eq!(
            m.report.attention,
            [Attention::Renamed {
                from: "pet-id".into(),
                to: "pet_id".into()
            }]
        );
    }

    #[test]
    fn api_prefix_cases() {
        assert_eq!(
            api_prefix("Swagger Petstore - OpenAPI 3.0"),
            "SWAGGER_PETSTORE"
        );
        assert_eq!(api_prefix("Pets"), "PETS");
        assert_eq!(api_prefix(""), "API");
        assert_eq!(api_prefix(" -- "), "API");
        let long = "Very Long Inventory Management Service Api";
        assert_eq!(long.len(), 42);
        assert_eq!(api_prefix(long), "VERY_LONG_INVENTORY");
        assert_eq!(
            api_prefix("Supercalifragilisticexpialidocious"),
            "SUPERCALIFRAGILISTICEXPI"
        );
        assert_eq!(api_prefix("my_api v2"), "MY_API_V2");
    }

    #[test]
    fn swagger2_end_to_end() {
        let m = map_document(&parse_document(V2).unwrap());
        assert_eq!((m.report.requests, m.report.folders), (2, 1));
        assert_eq!(
            var(&m, "PETSTORE_V2_BASE_URL").value,
            "https://api.example.com/v2"
        );
        let r = &m.requests[0];
        assert_eq!(r.url, "${PETSTORE_V2_BASE_URL}/pets/${petId}");
        assert_eq!(
            r.auth,
            Some(MappedAuth::Basic {
                username: "${PETSTORE_V2_BASIC_AUTH_USERNAME}".into(),
                password: "${PETSTORE_V2_BASIC_AUTH_PASSWORD}".into()
            })
        );
        let MappedBody::Form(f) = &m.requests[1].body else {
            panic!("{:?}", m.requests[1].body)
        };
        assert_eq!(f[0].key, "file");
        assert!(matches!(
            f[0].value,
            FieldValue::File {
                placeholder: true,
                ..
            }
        ));
        assert!(!f[0].disabled);
        assert_eq!(f[1], field("note", "string", true, None));
        assert!(m.report.attention.contains(&Attention::PickFile {
            request: "pets/Upload image".into(),
            field: "file".into()
        }));
    }

    /// Everything a mapping could write or show, values included.
    fn everything(m: &Mapped) -> String {
        format!(
            "{:?}{:?}{:?}{}",
            m.requests,
            m.variables.iter().map(|v| &v.value).collect::<Vec<_>>(),
            m.env_comments,
            lines(m).join("\n")
        )
    }

    #[test]
    fn cookie_values_scrubbed() {
        let mut v = spec(json!({ "/me": { "get": {
            "parameters": [
                { "name": "session", "in": "cookie", "required": true,
                  "schema": { "type": "string" }, "example": "s3cr3t_cookie" },
                { "name": "sid", "in": "cookie", "required": true,
                  "schema": { "type": "string" }, "example": "leak_sid" }
            ],
            "responses": {}
        } } }));
        v["security"] = json!([{ "ck": [] }]);
        v["components"] = json!({ "securitySchemes": {
            "ck": { "type": "apiKey", "in": "cookie", "name": "sid" }
        } });
        let m = map(v);
        assert_eq!(
            m.requests[0].headers,
            [field(
                "Cookie",
                "sid=${PETS_CK}; session=${PETS_SESSION}",
                false,
                None
            )]
        );
        let s = var(&m, "PETS_SESSION");
        assert_eq!((s.secret, s.value.as_str()), (true, ""));
        let all = everything(&m);
        for secret in ["s3cr3t_cookie", "leak_sid"] {
            assert!(!all.contains(secret), "{all}");
        }
    }

    #[test]
    fn only_known_schemes_stay_in_front_of_a_credential() {
        let m = map(spec(json!({ "/me": { "get": {
            "parameters": [{ "name": "Authorization", "in": "header", "required": true,
                "schema": { "type": "string" }, "example": "sk_live_x extra" }],
            "responses": {}
        } } })));
        assert_eq!(
            m.requests[0].headers,
            [field("Authorization", "${PETS_AUTHORIZATION}", false, None)]
        );
        assert!(!everything(&m).contains("sk_live_x"));

        let m = map(spec(json!({ "/me": { "get": {
            "parameters": [{ "name": "Authorization", "in": "header", "required": true,
                "schema": { "type": "string" }, "example": "token abc123" }],
            "responses": {}
        } } })));
        assert_eq!(
            m.requests[0].headers,
            [field(
                "Authorization",
                "token ${PETS_AUTHORIZATION}",
                false,
                None
            )]
        );
        assert!(!everything(&m).contains("abc123"));
    }

    #[test]
    fn path_param_secrets_scrubbed() {
        let m = map(spec(json!({ "/keys/{api_key}/{x-api-key}": { "get": {
            "parameters": [
                { "name": "api_key", "in": "path", "required": true,
                  "schema": { "type": "string" }, "example": "pk_live_z" },
                { "name": "x-api-key", "in": "path", "required": true,
                  "schema": { "type": "string", "example": "hk_live_w" } }
            ],
            "responses": {}
        } } })));
        assert_eq!(
            m.requests[0].file_vars,
            [
                ("api_key".to_string(), "${PETS_API_KEY}".to_string()),
                ("x_api_key".to_string(), "${PETS_X_API_KEY}".to_string())
            ]
        );
        assert!(var(&m, "PETS_API_KEY").secret);
        let all = everything(&m);
        for secret in ["pk_live_z", "hk_live_w"] {
            assert!(!all.contains(secret), "{all}");
        }
    }

    #[test]
    fn second_auth_scheme_registers_nothing() {
        let mut v = spec(json!({ "/a": { "get": {
            "security": [{ "first": [], "oauth": [], "basicAuth": [] }],
            "responses": {}
        } } }));
        v["components"] = json!({ "securitySchemes": {
            "first": { "type": "http", "scheme": "bearer" },
            "oauth": { "type": "oauth2" },
            "basicAuth": { "type": "http", "scheme": "basic" }
        } });
        let m = map(v);
        assert_eq!(
            m.requests[0].auth,
            Some(MappedAuth::Bearer("${PETS_FIRST_TOKEN}".into()))
        );
        let names: Vec<_> = m.variables.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(names, ["PETS_BASE_URL", "PETS_FIRST_TOKEN"]);
        assert_eq!(
            lines(&m),
            [
                "Skipped oauth2 auth (oauth) in 1 request. Add it by hand.",
                "Skipped basic auth (basicAuth) in 1 request. Add it by hand.",
                "Set the secret PETS_FIRST_TOKEN"
            ]
        );
    }

    #[test]
    fn form_schemas_merge_all_of() {
        let mut v = spec(json!({
            "/up": { "post": {
                "requestBody": { "content": { "multipart/form-data": { "schema": {
                    "allOf": [
                        { "$ref": "#/components/schemas/Base" },
                        { "properties": { "note": { "type": "string" } } }
                    ]
                } } } },
                "responses": {}
            } },
            "/form": { "post": {
                "requestBody": { "content": { "application/x-www-form-urlencoded": { "schema": {
                    "allOf": [{ "$ref": "#/components/schemas/Base" }],
                    "properties": { "extra": { "type": "integer" } }
                } } } },
                "responses": {}
            } }
        }));
        v["components"] = json!({ "schemas": { "Base": {
            "type": "object",
            "required": ["file", "name"],
            "properties": {
                "file": { "type": "string", "format": "binary" },
                "name": { "type": "string", "example": "Rex" }
            }
        } } });
        let m = map(v);
        let MappedBody::Form(f) = &m.requests[0].body else {
            panic!("{:?}", m.requests[0].body)
        };
        let keys: Vec<_> = f.iter().map(|f| (f.key.as_str(), f.disabled)).collect();
        assert_eq!(keys, [("file", false), ("name", false), ("note", true)]);
        // `file` has no text form, so it isn't in the urlencoded body.
        assert_eq!(
            m.requests[1].body,
            MappedBody::Text("file=.%2Fpick-a-file&name=Rex".into())
        );
        assert!(
            m.requests[1]
                .comments
                .contains(&"Optional form fields: extra".to_string())
        );
    }

    #[test]
    fn empty_form_bodies_reported() {
        let m = map(spec(json!({
            "/up": { "post": {
                "requestBody": { "content": { "multipart/form-data": { "schema": {
                    "type": "object", "additionalProperties": { "type": "string" }
                } } } },
                "responses": {}
            } },
            "/form": { "post": {
                "requestBody": { "content": { "application/x-www-form-urlencoded": {} } },
                "responses": {}
            } }
        })));
        assert!(m.requests.iter().all(|r| r.body == MappedBody::None));
        assert!(m.requests.iter().all(|r| r.headers.is_empty()));
        assert_eq!(
            lines(&m),
            [
                "Skipped multipart/form-data bodies in 1 request, like POST /up. Add them by hand.",
                "Skipped application/x-www-form-urlencoded bodies in 1 request, like POST /form. Add them by hand."
            ]
        );
    }

    #[test]
    fn bad_params_aggregate() {
        let op = json!({ "get": {
            "parameters": [
                { "in": "query", "schema": { "type": "string" } },
                { "name": "x", "in": "body" },
                { "name": "ok", "in": "query", "required": true, "schema": { "type": "string" } }
            ],
            "responses": {}
        } });
        let m = map(spec(json!({ "/a": op, "/b": op })));
        assert_eq!(m.requests[0].params, [field("ok", "string", false, None)]);
        assert_eq!(
            lines(&m),
            ["Skipped 4 parameters with no name or an unknown location, like in GET /a and GET /b"]
        );
    }
}
