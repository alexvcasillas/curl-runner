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
