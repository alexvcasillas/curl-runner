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
        assert!(start.elapsed() < std::time::Duration::from_secs(1));
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
        assert!(start.elapsed() < std::time::Duration::from_secs(1));
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
                examples: vec!["a/PUT /a".into(), "b/PUT /b".into()],
            }]
        );
        assert_eq!(
            lines(&m),
            [
                "Skipped application/octet-stream bodies in 2 requests, like a/PUT /a and b/PUT /b. Add them by hand."
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
            "Used k in 5 requests that also accept o"
        );
    }

    #[test]
    fn literal_credential_scrubbed() {
        let m = map(spec(json!({ "/me": { "get": {
            "parameters": [
                { "name": "X-Auth-Token", "in": "header", "required": true,
                  "schema": { "type": "string" }, "example": "Bearer sk_live_x" },
                { "name": "api_key", "in": "query", "required": true,
                  "schema": { "type": "string", "example": "qk_live_y" } }
            ],
            "responses": {}
        } } })));
        let r = &m.requests[0];
        assert_eq!(
            r.headers,
            [field("X-Auth-Token", "${PETS_AUTH_TOKEN}", false, None)]
        );
        assert_eq!(r.params, [field("api_key", "${PETS_API_KEY}", false, None)]);
        for name in ["PETS_AUTH_TOKEN", "PETS_API_KEY"] {
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
                    examples: vec!["badref/POST /badref".into()]
                }
            ]
        );
        assert_eq!(
            lines(&m)[1],
            "Couldn't resolve #/components/requestBodies/Nope in 1 request, like badref/POST /badref. Used an empty value."
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
                    examples: vec!["1/POST /1".into(), "2/POST /2".into(), "3/POST /3".into()]
                },
                Attention::UnresolvedRef {
                    target: "other.yaml#/P".into(),
                    external: true,
                    count: 1,
                    examples: vec!["x/GET /x".into()]
                }
            ]
        );
        assert_eq!(
            lines(&m)[0],
            "Couldn't resolve #/components/schemas/Gone in 4 requests, like 1/POST /1, 2/POST /2 and 3/POST /3. Used an empty value."
        );
        assert_eq!(
            lines(&m)[1],
            "Can't follow the external reference other.yaml#/P in 1 request, like x/GET /x. Used an empty value."
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
                "Left out optional cookies in 1 request, like cookies/GET /cookies",
                "2 parameters use a style curl-runner doesn't model, like in cookies/GET /cookies. Check their values.",
                "Skipped 1 callback",
                "Skipped 2 links",
                "Used the first of several body shapes in 1 request, like shapes/POST /shapes",
                "Skipped 2 webhooks",
                "Set the secret PETS_SESSION",
            ]
        );
    }

    #[test]
    fn tsv_collection_format_is_a_param_style() {
        let d = doc(json!({ "swagger": "2.0", "info": { "title": "T" },
            "paths": { "/x": { "get": { "parameters": [{ "in": "query", "name": "q",
                "type": "array", "items": { "type": "string" }, "collectionFormat": "tsv",
                "required": true }] } } } }));
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
    fn path_param_secrets_scrubbed() {
        // Only a literal the document gives, under an exact secret name.
        let m = map(spec(
            json!({ "/keys/{api_key}/{token}/{x-api-key}/{secret_name}": { "get": {
            "parameters": [
                { "name": "api_key", "in": "path", "required": true,
                  "schema": { "type": "string" }, "example": "pk_live_z" },
                { "name": "token", "in": "path", "required": true,
                  "schema": { "type": "string" } },
                { "name": "x-api-key", "in": "path", "required": true,
                  "schema": { "type": "string", "example": "hk_live_w" } },
                { "name": "secret_name", "in": "path", "required": true,
                  "schema": { "type": "string", "example": "deploy" } }
            ],
            "responses": {}
        } } }),
        ));
        assert_eq!(
            m.requests[0].file_vars,
            [
                ("api_key".to_string(), "${PETS_API_KEY}".to_string()),
                ("token".to_string(), "string".to_string()),
                ("x_api_key".to_string(), "hk_live_w".to_string()),
                ("secret_name".to_string(), "deploy".to_string())
            ]
        );
        assert!(var(&m, "PETS_API_KEY").secret);
        assert!(!m.variables.iter().any(|v| v.name == "PETS_TOKEN"));
        assert!(!everything(&m).contains("pk_live_z"));

        // A schema `default` or `enum` counts as a literal too.
        for schema in [
            json!({ "type": "string", "default": "d_live" }),
            json!({ "type": "string", "enum": ["e_live"] }),
        ] {
            let m = map(spec(json!({ "/t/{token}": { "get": {
                "parameters": [{ "name": "token", "in": "path", "required": true, "schema": schema }],
                "responses": {}
            } } })));
            assert_eq!(
                m.requests[0].file_vars,
                [("token".to_string(), "${PETS_TOKEN}".to_string())]
            );
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
                "Skipped multipart/form-data bodies in 1 request, like up/POST /up. Add them by hand.",
                "Skipped application/x-www-form-urlencoded bodies in 1 request, like form/POST /form. Add them by hand."
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
            [
                "Skipped 4 parameters with no name or an unknown location, like in a/GET /a and b/GET /b"
            ]
        );
    }

    #[test]
    fn untagged_operations_get_a_path_folder() {
        let m = map(spec(json!({
            "/v1/customers/{id}": { "get": { "responses": {} } },
            "/api/V2.1/Charges": { "get": { "responses": {} } },
            "/{tenant}/v3/apiary": { "get": { "responses": {} } },
            "/v1/{id}": { "get": { "responses": {} } },
            "/": { "get": { "responses": {} } },
            "/v1/customers": { "post": { "tags": ["billing"], "responses": {} } }
        })));
        let folders: Vec<_> = m.requests.iter().map(|r| r.folders.clone()).collect();
        assert_eq!(
            folders,
            [
                vec!["customers".to_string()],
                vec!["Charges".to_string()],
                vec!["apiary".to_string()],
                vec![],
                vec![],
                vec!["billing".to_string()]
            ]
        );
        assert_eq!(m.report.folders, 4);
    }

    #[test]
    fn empty_form_schema_means_no_body() {
        let empty = json!({ "type": "object", "properties": {}, "additionalProperties": false });
        let op = |media: &str, schema: Value| {
            json!({ "get": {
                "requestBody": { "content": { media: { "schema": schema } } },
                "responses": {}
            } })
        };
        let m = map(spec(json!({
            "/a": op("application/x-www-form-urlencoded", empty.clone()),
            "/b": op("multipart/form-data", json!({ "type": "object" })),
            "/c": op("application/x-www-form-urlencoded", json!({ "type": "object", "additionalProperties": true })),
            "/d": op("multipart/form-data", json!({ "type": "string" })),
            "/e": op("application/x-www-form-urlencoded", json!({ "oneOf": [empty.clone(), empty] }))
        })));
        assert!(m.requests.iter().all(|r| r.body == MappedBody::None));
        assert!(m.requests.iter().all(|r| r.headers.is_empty()));
        assert_eq!(
            lines(&m),
            [
                "Skipped application/x-www-form-urlencoded bodies in 2 requests, like c/GET /c and e/GET /e. Add them by hand.",
                "Skipped multipart/form-data bodies in 1 request, like d/GET /d. Add them by hand."
            ]
        );
    }

    #[test]
    fn param_style_counts_enabled_params_only() {
        let m = map(spec(json!({ "/a": { "get": {
            "parameters": [
                { "name": "expand", "in": "query", "style": "deepObject",
                  "schema": { "type": "object" } },
                { "name": "ids", "in": "query", "style": "pipeDelimited", "required": true,
                  "schema": { "type": "array", "items": { "type": "integer" } } }
            ],
            "responses": {}
        } } })));
        assert_eq!(
            m.report.attention,
            [Attention::ParamStyle {
                count: 1,
                examples: vec!["a/GET /a".into()]
            }]
        );
    }

    #[test]
    fn html_is_stripped_from_comments_and_readme() {
        let mut v = spec(json!({ "/a": { "get": {
            "summary": "<b>Kept</b> as is",
            "description": "<p>Lists <code>items</code> where a < b &amp; c &gt; d.</p>\n<p>See <a href=\"https://docs.example.com\">the docs</a>.<br/></p>\n<p></p>",
            "parameters": [{ "name": "q", "in": "query", "description": "<p>Free &quot;text&quot; &#39;search&#39;</p>",
                "schema": { "type": "string" } }],
            "responses": {}
        } } }));
        v["info"]["description"] =
            json!("<p></p>\n<h1>Inventory</h1>\n\n<p>Tracks <em>stock</em>.</p>\n<br/>");
        let m = map(v);
        let r = &m.requests[0];
        assert_eq!(r.name, "<b>Kept</b> as is");
        assert_eq!(
            r.comments,
            ["Lists items where a < b & c > d.", "See the docs."]
        );
        assert_eq!(r.params[0].note.as_deref(), Some("Free \"text\" 'search'"));
        assert_eq!(m.description.as_deref(), Some("Inventory\n\nTracks stock."));
    }

    #[test]
    fn examples_ending_in_a_period_end_the_sentence_once() {
        let a = Attention::SkippedBody {
            media: "application/octet-stream".into(),
            count: 1,
            examples: vec!["pet/Uploads an image.".into()],
        };
        assert_eq!(
            a.to_string(),
            "Skipped application/octet-stream bodies in 1 request, like pet/Uploads an image. Add them by hand."
        );
        let a = Attention::UnresolvedRef {
            target: "#/x".into(),
            external: false,
            count: 2,
            examples: vec!["a.".into(), "b".into()],
        };
        assert_eq!(
            a.to_string(),
            "Couldn't resolve #/x in 2 requests, like a. and b. Used an empty value."
        );
        let a = Attention::ParamStyle {
            count: 1,
            examples: vec!["Do it.".into()],
        };
        assert_eq!(
            a.to_string(),
            "1 parameter uses a style curl-runner doesn't model, like in Do it. Check its value."
        );
    }

    #[test]
    fn server_urls_lose_control_characters() {
        let mut v = spec(json!({}));
        v["servers"] = json!([
            { "url": "https://api.example.com\n/v1\u{7}" },
            { "url": "https://b.example.com\nOTHER_TOKEN=evil\r\nX=1\u{2028}Y=2\u{2029}Z=3" }
        ]);
        let m = map(v);
        assert_eq!(var(&m, "PETS_BASE_URL").value, "https://api.example.com/v1");
        assert_eq!(
            m.env_comments,
            [(
                "PETS_BASE_URL".to_string(),
                "Other server: https://b.example.comOTHER_TOKEN=evilX=1Y=2Z=3".to_string()
            )]
        );
    }

    #[test]
    fn path_param_literals_from_any_source_scrubbed() {
        let m = map(spec(json!({ "/k/{token}/{api_key}/{secret}": { "get": {
            "parameters": [
                { "name": "token", "in": "path", "required": true,
                  "content": { "text/plain": { "schema": { "type": "string" }, "example": "tk_live_a" } } },
                { "name": "api_key", "in": "path", "required": true,
                  "schema": { "allOf": [{ "type": "string", "example": "ak_live_b" }] } },
                { "name": "secret", "in": "path", "required": true,
                  "schema": { "oneOf": [{ "type": "string", "example": "sc_live_c" }, { "type": "integer" }] } }
            ],
            "responses": {}
        } } })));
        assert_eq!(
            m.requests[0].file_vars,
            [
                ("token".to_string(), "${PETS_TOKEN}".to_string()),
                ("api_key".to_string(), "${PETS_API_KEY}".to_string()),
                ("secret".to_string(), "${PETS_SECRET}".to_string())
            ]
        );
        let all = everything(&m);
        for leak in ["tk_live_a", "ak_live_b", "sc_live_c"] {
            assert!(!all.contains(leak), "{all}");
        }
    }

    #[test]
    fn swagger2_body_uses_the_preferred_consumes() {
        let d = doc(json!({ "swagger": "2.0", "info": { "title": "Pets" },
            "host": "api.example.com",
            "paths": {
                "/x": { "post": {
                    "consumes": ["application/xml", "application/json"],
                    "parameters": [{ "in": "body", "name": "b",
                        "schema": { "type": "object", "properties": { "a": { "type": "string" } } } }],
                    "responses": {}
                } },
                "/f": { "post": {
                    "consumes": ["application/x-www-form-urlencoded", "multipart/form-data"],
                    "parameters": [{ "in": "formData", "name": "n", "type": "string", "required": true }],
                    "responses": {}
                } },
                "/u": { "post": {
                    "consumes": ["application/x-www-form-urlencoded", "multipart/form-data"],
                    "parameters": [{ "in": "formData", "name": "f", "type": "file", "required": true }],
                    "responses": {}
                } }
            } }));
        let content = &d.root["paths"]["/x"]["post"]["requestBody"]["content"];
        assert!(content.get("application/xml").is_some());
        assert!(content.get("application/json").is_some());
        let form = &d.root["paths"]["/f"]["post"]["requestBody"]["content"];
        assert!(form.get("application/x-www-form-urlencoded").is_some());
        assert!(form.get("multipart/form-data").is_some());
        let upload = &d.root["paths"]["/u"]["post"]["requestBody"]["content"];
        assert!(upload.get("application/x-www-form-urlencoded").is_none());
        assert!(upload.get("multipart/form-data").is_some());

        let m = map_document(&d);
        let r = &m.requests[0];
        assert_eq!(r.body, MappedBody::Json(json!({ "a": "string" })));
        assert_eq!(
            r.headers,
            [field("Content-Type", "application/json", false, None)]
        );
        assert!(matches!(m.requests[1].body, MappedBody::Form(_)));
    }

    #[test]
    fn ignored_header_params_and_one_cookie() {
        let mut v = spec(json!({
            "/me": { "get": {
                "parameters": [
                    { "name": "Authorization", "in": "header", "required": true,
                      "schema": { "type": "string" }, "example": "Bearer az_live_x" },
                    { "name": "Accept", "in": "header", "required": true,
                      "schema": { "type": "string" }, "example": "text/csv" },
                    { "name": "Content-Type", "in": "header", "required": true,
                      "schema": { "type": "string" } },
                    { "name": "Cookie", "in": "header", "required": true,
                      "schema": { "type": "string" }, "example": "ck_live_y=1" },
                    { "name": "session", "in": "cookie", "required": true,
                      "schema": { "type": "string" } }
                ],
                "responses": {}
            } },
            "/other": { "get": {
                "security": [{ "bearer": [] }],
                "parameters": [{ "name": "authorization", "in": "header", "required": true,
                    "schema": { "type": "string" } }],
                "responses": {}
            } }
        }));
        v["security"] = json!([{ "ck": [] }]);
        v["components"] = json!({ "securitySchemes": {
            "ck": { "type": "apiKey", "in": "cookie", "name": "sid" },
            "bearer": { "type": "http", "scheme": "bearer" }
        } });
        let m = map(v);
        assert_eq!(
            m.requests[0].headers,
            [field(
                "Cookie",
                "sid=${PETS_CK}; session=${PETS_SESSION}; ${PETS_COOKIE}",
                false,
                None
            )]
        );
        assert!(m.requests[1].headers.is_empty());
        let all = everything(&m);
        for leak in ["az_live_x", "ck_live_y", "text/csv"] {
            assert!(!all.contains(leak), "{all}");
        }
        assert!(!m.variables.iter().any(|v| v.name == "PETS_AUTHORIZATION"));
        assert_eq!(
            lines(&m)[0],
            "Left out the Authorization header parameter in 1 request, like me/GET /me. \
             OpenAPI sends auth through security schemes. Add it by hand if the API needs it."
        );
    }

    #[test]
    fn document_dollar_braces_reported() {
        let mut v = spec(json!({
            "/q": { "get": {
                "parameters": [{ "name": "q", "in": "query", "required": true,
                    "schema": { "type": "string" }, "example": "${HOME}" }],
                "responses": {}
            } },
            "/b": { "post": {
                "requestBody": { "content": { "application/json": {
                    "example": { "a": "${AWS_SECRET}" } } } },
                "responses": {}
            } },
            "/p/{id}": { "get": {
                "parameters": [{ "name": "id", "in": "path", "required": true,
                    "schema": { "type": "string", "default": "${USER}" } }],
                "responses": {}
            } },
            "/s": {
                "servers": [{ "url": "https://${HOST}.example.com" }],
                "get": { "responses": {} }
            },
            "/f": { "post": {
                "requestBody": { "content": { "application/x-www-form-urlencoded": {
                    "schema": { "type": "object", "required": ["a"],
                        "properties": { "a": { "type": "string", "enum": ["${PATH}"] } } } } } },
                "responses": {}
            } },
            "/plain": { "get": { "responses": {} } }
        }));
        v["servers"] = json!([{ "url": "https://api.example.com" }]);
        let m = map(v);
        let hits: Vec<_> = lines(&m)
            .into_iter()
            .filter(|l| l.starts_with("Kept ${"))
            .collect();
        assert_eq!(
            hits,
            [
                "Kept ${…} from the API description in 5 requests, like q/GET /q, b/POST /b and p/GET /p/{id}. \
              curl-runner fills them from your environment when it runs. Check them."
            ]
        );
        let one = Attention::DocumentVars {
            count: 1,
            examples: vec!["a".into()],
        };
        assert_eq!(
            one.to_string(),
            "Kept ${…} from the API description in 1 request, like a. \
             curl-runner fills it from your environment when it runs. Check it."
        );
    }

    #[test]
    fn auth_choice_with_no_auth() {
        let mut v = spec(json!({
            "/a": { "get": { "security": [{}, { "k": [] }], "responses": {} } },
            "/b": { "get": { "security": [{ "k": [] }, {}], "responses": {} } }
        }));
        v["components"] = json!({ "securitySchemes": {
            "k": { "type": "apiKey", "in": "header", "name": "X-Key" }
        } });
        let m = map(v);
        let l = lines(&m);
        assert_eq!(l[0], "Sent no auth in 1 request that also accepts k");
        assert_eq!(l[1], "Used k in 1 request that also accepts no auth");
    }

    #[test]
    fn undeclared_path_vars_get_a_sample() {
        let m = map(spec(json!({
            "/a/{id}": { "get": { "responses": {} } },
            "/b/{x}/{y}": { "get": {
                "parameters": [{ "name": "x", "in": "path", "required": true,
                    "schema": { "type": "integer" } }],
                "responses": {}
            } }
        })));
        assert_eq!(
            m.requests[0].file_vars,
            [("id".to_string(), "string".to_string())]
        );
        assert_eq!(
            m.requests[1].file_vars,
            [
                ("x".to_string(), "1".to_string()),
                ("y".to_string(), "string".to_string())
            ]
        );
        assert_eq!(
            lines(&m),
            [
                "Gave undeclared path variables a sample value in 2 requests, like a/GET /a/{id} and b/GET /b/{x}/{y}. Set the right values."
            ]
        );
    }

    #[test]
    fn server_credentials_removed() {
        let mut v = spec(json!({
            "/a": {
                "servers": [{ "url": "https://own:hunter3@files.example.com" }],
                "get": { "responses": {} }
            }
        }));
        v["servers"] = json!([
            { "url": "https://alice:hunter2@api.example.com/v1" },
            { "url": "http://bob@b.example.com/x@y" }
        ]);
        let m = map(v);
        assert_eq!(var(&m, "PETS_BASE_URL").value, "https://api.example.com/v1");
        assert_eq!(
            m.env_comments[0].1,
            "Other server: http://b.example.com/x@y"
        );
        assert_eq!(m.requests[0].url, "https://files.example.com/a");
        let all = everything(&m);
        for leak in ["alice", "hunter2", "bob", "own:", "hunter3"] {
            assert!(!all.contains(leak), "{all}");
        }
        let l = lines(&m);
        assert_eq!(
            l.iter()
                .filter(|l| l.starts_with("Removed a user name"))
                .count(),
            1,
            "{l:?}"
        );
        assert!(l.contains(
            &"Removed a user name or password from a server URL. Add credentials as auth if the API needs them."
                .to_string()
        ));
    }

    #[test]
    fn tags_differing_in_case_are_one_folder() {
        let m = map(spec(json!({
            "/a": { "get": { "tags": ["Pets"], "responses": {} } },
            "/b": { "get": { "tags": ["pets"], "responses": {} } },
            "/c": { "get": { "tags": ["Pet Store"], "responses": {} } },
            "/d": { "get": { "tags": ["pet-store"], "responses": {} } }
        })));
        assert_eq!(m.report.folders, 2);
    }
}

mod convert {
    use std::path::Path;

    use super::*;
    use crate::convert::import::{Import, write_import};

    const SMALL: &str = include_str!("../../../tests/fixtures/openapi/small_v30.yaml");
    const LARGE: &str = include_str!("../../../tests/fixtures/openapi/large_v31.json");
    const SMALL_EMITTED: &str =
        include_str!("../../../tests/fixtures/openapi/small_v30.emitted.txt");
    const LARGE_EMITTED: &str =
        include_str!("../../../tests/fixtures/openapi/large_v31.emitted.txt");

    /// Every file as `=== path`, then its YAML, in plan order.
    fn dump(imp: &Import) -> String {
        let mut out = String::new();
        for f in &imp.files {
            let rel = f
                .rel_path
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            out.push_str(&format!("=== {rel}\n{}\n", f.yaml));
        }
        out
    }

    fn lines(imp: &Import) -> Vec<String> {
        imp.report
            .attention
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn emits_small_spec() {
        let imp = convert_openapi(SMALL).unwrap();
        assert_eq!(dump(&imp), SMALL_EMITTED);
        assert_eq!(imp.name, "Pets");
        assert_eq!(imp.source, "OpenAPI");
        assert_eq!(
            imp.readme.as_deref(),
            Some("# Pets\n\nA small pet store API.\n")
        );
        assert_eq!(
            imp.env_comments,
            [(
                "PETS_BASE_URL".to_string(),
                "Other server: https://staging.example.com/v1".to_string()
            )]
        );
        assert_eq!((imp.report.folders, imp.report.requests), (3, 6));
        assert_eq!(
            lines(&imp),
            [
                "Pick a file for \"file\" in pets/Upload photo",
                "Set the secret PETS_API_KEY"
            ]
        );
        for f in &imp.files {
            assert!(f.yaml.contains("  expect:\n    status: "), "{}", f.request);
        }
    }

    #[test]
    fn disabled_limit_line_uncomments_into_a_request() {
        let imp = convert_openapi(SMALL).unwrap();
        let y = &imp.files[0].yaml;
        assert!(y.contains("    # limit: \"1\"\n"));
        let on = y.replacen("# limit", "limit", 1);
        let v = crate::parser::parse_raw_stream(&on).unwrap();
        assert_eq!(v["request"]["params"]["limit"], "1");
        let raw = crate::parser::parse_raw(&on).unwrap();
        let cfg = crate::parser::from_raw(raw, Path::new("emitted.yaml")).unwrap();
        assert_eq!(cfg.requests.len(), 1);
        // The doc comment line above it stays a comment.
        assert!(on.contains("# # How many pets to return."));
    }

    #[test]
    fn emitted_files_parse_as_requests() {
        for src in [SMALL, LARGE] {
            for f in &convert_openapi(src).unwrap().files {
                let raw = crate::parser::parse_raw(&f.yaml)
                    .unwrap_or_else(|e| panic!("{}: {e:?}\n{}", f.request, f.yaml));
                crate::parser::from_raw(raw, Path::new("emitted.yaml"))
                    .unwrap_or_else(|e| panic!("{}: {e:?}\n{}", f.request, f.yaml));
            }
        }
    }

    #[test]
    fn emits_large_spec() {
        let imp = convert_openapi(LARGE).unwrap();
        assert_eq!(dump(&imp), LARGE_EMITTED);
        assert_eq!(imp.name, "Inventory Service");
        assert_eq!((imp.report.folders, imp.report.requests), (4, 14));
        assert_eq!(
            lines(&imp),
            [
                "Used the first of several body shapes in 1 request, like orders/Place order",
                "Used bearer_auth in 1 request that also accepts key_auth",
                "Skipped 1 webhook",
                "Set the secret INVENTORY_SERVICE_BEARER_AUTH_TOKEN",
                "Set the secret INVENTORY_SERVICE_KEY_AUTH"
            ]
        );
    }

    #[test]
    fn missing_title_uses_a_default_name() {
        let imp = convert_openapi(r#"{"openapi":"3.0.0","paths":{}}"#).unwrap();
        assert_eq!(imp.name, "OpenAPI import");
        assert!(imp.readme.is_none());
    }

    #[test]
    fn two_apis_one_root() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let small = convert_openapi(SMALL).unwrap();
        let large = convert_openapi(LARGE).unwrap();
        let a = write_import(root, Path::new("pets"), &small, "note").unwrap();
        let b = write_import(root, Path::new("inventory"), &large, "note").unwrap();
        assert!(a.kept_env.is_empty() && b.kept_env.is_empty());
        let again = write_import(root, Path::new("pets"), &small, "note").unwrap();
        assert!(again.kept_env.is_empty());
        assert_eq!(again.written.len(), 6);
        assert!(
            again
                .written
                .iter()
                .all(|p| p.to_string_lossy().ends_with("-2.yaml"))
        );
        let env = std::fs::read_to_string(root.join(".env")).unwrap();
        for key in [
            "PETS_BASE_URL=https://api.example.com/v1",
            "INVENTORY_SERVICE_BASE_URL=https://api.example.com/inventory/v2",
            "# PETS_API_KEY=",
            "# INVENTORY_SERVICE_BEARER_AUTH_TOKEN=",
            "# INVENTORY_SERVICE_KEY_AUTH=",
        ] {
            assert_eq!(env.matches(key).count(), 1, "{key}\n{env}");
        }
        assert!(env.contains("# Imported from OpenAPI: Pets\n"));
        assert!(env.contains("# Imported from OpenAPI: Inventory Service\n"));
    }

    #[test]
    fn a_changed_base_url_is_reported_as_kept() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".env"),
            "PETS_BASE_URL=https://mine.example.com\n",
        )
        .unwrap();
        let imp = convert_openapi(SMALL).unwrap();
        let out = write_import(dir.path(), Path::new("pets"), &imp, "note").unwrap();
        assert_eq!(out.kept_env, ["PETS_BASE_URL"]);
    }

    #[test]
    fn secrets_never_written() {
        let spec = r#"{"openapi":"3.0.0","info":{"title":"Pets","version":"1"},
            "servers":[{"url":"https://api.example.com"}],
            "components":{"securitySchemes":{"api_key":{"type":"apiKey","in":"header","name":"X-API-Key","x-example":"sk-live-123"}}},
            "security":[{"api_key":[]}],
            "paths":{"/a":{"get":{"parameters":[
                {"name":"X-API-Key","in":"header","example":"sk-live-123","schema":{"type":"string"}}],
                "responses":{"200":{"description":"ok"}}}}}}"#;
        let dir = tempfile::tempdir().unwrap();
        for src in [SMALL, spec] {
            let imp = convert_openapi(src).unwrap();
            write_import(dir.path(), Path::new("api"), &imp, "note").unwrap();
        }
        let env = std::fs::read_to_string(dir.path().join(".env")).unwrap();
        assert!(env.contains("# PETS_API_KEY=\n"));
        assert!(!env.lines().any(|l| l.starts_with("PETS_API_KEY=")));
        assert!(!env.contains("sk-live-123"));
        for f in &convert_openapi(spec).unwrap().files {
            assert!(!f.yaml.contains("sk-live-123"), "{}", f.yaml);
        }
    }

    /// About 1.2 MB at 3000 operations: a scaling check, not the spec's 10 MB figure.
    ///
    /// `n` operations over `schemas` schemas, each schema reaching the next
    /// through a 5-deep `$ref` chain.
    fn synthetic(n: usize, schemas: usize) -> String {
        let mut schema_map = serde_json::Map::new();
        for i in 0..schemas {
            let mut props = serde_json::Map::new();
            props.insert("id".into(), json!({ "type": "integer" }));
            props.insert(
                "next".into(),
                json!({ "$ref": format!("#/components/schemas/S{}", (i + 1) % schemas) }),
            );
            props.insert(
                "alias".into(),
                json!({ "$ref": format!("#/components/schemas/A{i}_0") }),
            );
            schema_map.insert(
                format!("S{i}"),
                json!({ "type": "object", "properties": props }),
            );
            for d in 0..4 {
                schema_map.insert(
                    format!("A{i}_{d}"),
                    json!({ "$ref": format!("#/components/schemas/A{i}_{}", d + 1) }),
                );
            }
            schema_map.insert(format!("A{i}_4"), json!({ "type": "string" }));
        }
        let mut paths = serde_json::Map::new();
        for i in 0..n {
            let s = i % schemas;
            paths.insert(
                format!("/r{i}/{{id}}"),
                json!({ "post": {
                    "tags": [format!("t{}", i % 20)],
                    "summary": format!("Op {i}"),
                    "parameters": [{ "name": "id", "in": "path", "required": true, "schema": { "type": "integer" } }],
                    "requestBody": { "content": { "application/json": {
                        "schema": { "$ref": format!("#/components/schemas/S{s}") } } } },
                    "responses": { "200": { "description": "ok" } }
                } }),
            );
        }
        json!({
            "openapi": "3.0.0",
            "info": { "title": "Big", "version": "1" },
            "servers": [{ "url": "https://api.example.com" }],
            "paths": paths,
            "components": { "schemas": schema_map }
        })
        .to_string()
    }

    #[test]
    #[cfg_attr(debug_assertions, ignore = "run with --release")]
    fn synthetic_speed() {
        let text = synthetic(3000, 300);
        let start = std::time::Instant::now();
        let imp = convert_openapi(&text).unwrap();
        let took = start.elapsed();
        assert_eq!(imp.files.len(), 3000);
        assert!(took < std::time::Duration::from_secs(2), "{took:?}");
    }

    /// Catches a quadratic slip in a debug build, where the release-only
    /// test above doesn't run.
    #[test]
    fn synthetic_speed_debug() {
        let text = synthetic(500, 50);
        let start = std::time::Instant::now();
        let imp = convert_openapi(&text).unwrap();
        let took = start.elapsed();
        assert_eq!(imp.files.len(), 500);
        assert!(took < std::time::Duration::from_secs(5), "{took:?}");
    }

    #[test]
    fn other_servers_cannot_inject_env_lines() {
        let spec = json!({
            "openapi": "3.0.0",
            "info": { "title": "Pets", "version": "1" },
            "servers": [
                { "url": "https://api.example.com" },
                { "url": "https://b.example.com\nOTHER_TOKEN=evil\nOTHER_BASE_URL=https://evil.example.com" },
                { "url": "https://c.example.com\r\nOTHER_TOKEN=evil2\u{2028}OTHER_TOKEN=evil3" }
            ],
            "paths": {}
        });
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".env"),
            "OTHER_TOKEN=realsecret\nOTHER_BASE_URL=https://other.example.com\n",
        )
        .unwrap();
        let imp = convert_openapi(&spec.to_string()).unwrap();
        write_import(dir.path(), Path::new("pets"), &imp, "note").unwrap();
        let env = std::fs::read_to_string(dir.path().join(".env")).unwrap();
        let keys: Vec<&str> = env
            .split(['\n', '\r', '\u{2028}', '\u{2029}'])
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .collect();
        assert_eq!(
            keys,
            [
                "OTHER_TOKEN=realsecret",
                "OTHER_BASE_URL=https://other.example.com",
                "PETS_BASE_URL=https://api.example.com"
            ],
            "{env}"
        );
        let loaded = crate::env::parse_env_content(&env);
        assert_eq!(loaded["OTHER_TOKEN"], "realsecret");
        assert_eq!(loaded["OTHER_BASE_URL"], "https://other.example.com");
    }

    #[test]
    fn dry_run_sends_one_authorization_and_one_cookie() {
        let spec = json!({
            "openapi": "3.0.0",
            "info": { "title": "Pets", "version": "1" },
            "servers": [{ "url": "https://api.example.com" }],
            "security": [{ "bearer": [], "ck": [] }],
            "components": { "securitySchemes": {
                "bearer": { "type": "http", "scheme": "bearer" },
                "ck": { "type": "apiKey", "in": "cookie", "name": "sid" }
            } },
            "paths": { "/me": { "get": {
                "parameters": [
                    { "name": "Authorization", "in": "header", "required": true,
                      "schema": { "type": "string" } },
                    { "name": "Cookie", "in": "header", "required": true,
                      "schema": { "type": "string" } },
                    { "name": "session", "in": "cookie", "required": true,
                      "schema": { "type": "string" } }
                ],
                "responses": {}
            } } }
        });
        let imp = convert_openapi(&spec.to_string()).unwrap();
        let yaml = &imp.files[0].yaml;
        let raw = crate::parser::parse_raw(yaml).unwrap_or_else(|e| panic!("{e:?}\n{yaml}"));
        let parsed = crate::parser::from_raw(raw, Path::new("me.yaml")).unwrap();
        let cmd = crate::transport::to_curl_command(&parsed.requests[0], &parsed.global);
        let lower = cmd.to_ascii_lowercase();
        assert_eq!(lower.matches("authorization:").count(), 1, "{cmd}\n{yaml}");
        assert_eq!(lower.matches("cookie:").count(), 1, "{cmd}\n{yaml}");
    }

    #[test]
    fn synthetic_small_shape_converts() {
        let imp = convert_openapi(&synthetic(30, 10)).unwrap();
        assert_eq!(imp.files.len(), 30);
    }
}
