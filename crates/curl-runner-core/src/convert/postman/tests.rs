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

fn mapped(name: &str) -> Mapped {
    map_collection(&parse_collection(&fixture(name)).unwrap())
}

fn req(m: &Mapped, name: &str) -> MappedRequest {
    m.requests.iter().find(|r| r.name == name).unwrap().clone()
}

/// The report has an `Attention` for this request that `f` accepts.
fn m_has(m: &Mapped, r: &MappedRequest, f: impl Fn(&Attention) -> bool) -> bool {
    let path = r.path();
    m.report.attention.iter().any(|a| {
        let at = match a {
            Attention::PickFile { request, .. } => request,
            Attention::Script { at }
            | Attention::UnsupportedAuth { at, .. }
            | Attention::UnsupportedBody { at, .. }
            | Attention::DynamicVar { at, .. }
            | Attention::Examples { at, .. } => at,
            _ => return false,
        };
        *at == path && f(a)
    })
}

fn text(f: &Field) -> &str {
    match &f.value {
        FieldValue::Text(v) => v,
        FieldValue::File { path, .. } => path,
    }
}

#[test]
fn sample_maps_like_the_collection() {
    let m = mapped("sample.postman_collection.json");
    assert_eq!((m.report.folders, m.report.requests), (2, 5));
    let api_key = m.variables.iter().find(|v| v.name == "apiKey").unwrap();
    assert!(api_key.secret);
    let r = req(&m, "Create customer");
    assert_eq!(r.url, "${baseUrl}/v1/customers");
    assert!(r.headers.iter().any(
        |h| h.key == "X-Api-Key" && matches!(&h.value, FieldValue::Text(v) if v == "${apiKey}")
    ));
    assert!(
        !req(&m, "Health")
            .headers
            .iter()
            .any(|h| h.key == "X-Api-Key")
    ); // noauth
}

#[test]
fn sample_keeps_order_comments_and_query() {
    let m = mapped("sample.postman_collection.json");
    assert_eq!(
        m.description.as_deref(),
        Some("A small example collection.")
    );
    let r = req(&m, "Create customer");
    assert_eq!(r.folders, vec!["Customers".to_string()]);
    assert_eq!(r.comments, vec!["Creates a customer.".to_string()]);
    assert_eq!(r.method, "POST");
    let keys: Vec<_> = r.headers.iter().map(|h| h.key.as_str()).collect();
    assert_eq!(keys, ["X-Api-Key", "Content-Type"]); // auth first, then the request's own
    assert!(matches!(&r.body, MappedBody::Json(v) if v["name"] == "Ada Lovelace"));

    let list = req(&m, "List customers");
    assert_eq!(list.url, "${baseUrl}/v1/customers");
    let params: Vec<_> = list
        .params
        .iter()
        .map(|p| (p.key.as_str(), text(p), p.disabled))
        .collect();
    assert_eq!(
        params,
        [("limit", "10", false), ("starting_after", "cus_1", true)]
    );
    assert_eq!(
        list.comments,
        vec!["Lists customers, newest first.".to_string()]
    );
}

#[test]
fn disabled_fields_keep_their_place_and_note() {
    let m = mapped("sample.postman_collection.json");
    let r = req(&m, "Convert document");
    let MappedBody::Form(f) = &r.body else {
        panic!()
    };
    let keys: Vec<_> = f.iter().map(|x| (x.key.as_str(), x.disabled)).collect();
    assert_eq!(
        keys,
        [
            ("document", false),
            ("attachment", false),
            ("format", false),
            ("watermark", true),
            ("password", true),
        ]
    );
    assert!(f.iter().any(|x| x.disabled && x.note.is_some()));
    assert!(matches!(
        &f[0].value,
        FieldValue::File {
            placeholder: true,
            ..
        }
    ));
    assert!(
        matches!(&f[1].value, FieldValue::File { path, placeholder: false } if path == "/tmp/example.pdf")
    );
    assert!(m_has(
        &m,
        &r,
        |a| matches!(a, Attention::PickFile { field, .. } if field == "document")
    ));
    assert!(!m_has(
        &m,
        &r,
        |a| matches!(a, Attention::PickFile { field, .. } if field == "attachment")
    ));
}

#[test]
fn unquoted_variable_in_json_becomes_text() {
    let r = req(&mapped("edge.postman_collection.json"), "Count");
    assert!(matches!(&r.body, MappedBody::Text(t) if t.contains("\"count\": ${n}")));
    assert!(matches!(&r.body, MappedBody::Text(t) if t.contains("\"${UUID}\"")));
    assert!(matches!(&r.body, MappedBody::Text(t) if t.contains("{{$randomColor}}")));
    assert!(
        r.headers
            .iter()
            .any(|h| h.key == "Content-Type" && text(h) == "application/json")
    );
}

#[test]
fn undefined_variables_are_reported_once() {
    let m = mapped("edge.postman_collection.json");
    let n = m
        .report
        .attention
        .iter()
        .filter(|a| matches!(a, Attention::SetVariable { var } if var == "n"))
        .count();
    assert_eq!(n, 1);
    // `id` is a path variable with its own value; `baseUrl` is defined.
    assert!(
        !m.report.attention.iter().any(
            |a| matches!(a, Attention::SetVariable { var } if var == "id" || var == "baseUrl")
        )
    );
}

#[test]
fn edge_report_lists_every_unsupported_part() {
    let a = mapped("edge.postman_collection.json").report.attention;
    for want in [
        "Script",
        "UnsupportedAuth",
        "UnsupportedBody",
        "DynamicVar",
        "Examples",
        "Renamed",
    ] {
        assert!(
            a.iter().any(|x| format!("{x:?}").starts_with(want)),
            "{want}"
        );
    }
}

#[test]
fn edge_report_points_at_the_request() {
    let m = mapped("edge.postman_collection.json");
    let scripted = req(&m, "Scripted");
    assert!(m_has(&m, &scripted, |a| matches!(
        a,
        Attention::Script { .. }
    )));
    assert!(m_has(&m, &scripted, |a| matches!(
        a,
        Attention::Examples { count: 1, .. }
    )));
    let oauth = req(&m, "OAuth2");
    assert!(oauth.auth.is_none());
    assert!(m_has(
        &m,
        &oauth,
        |a| matches!(a, Attention::UnsupportedAuth { kind, .. } if kind == "oauth2")
    ));
    let upload = req(&m, "Upload");
    assert!(matches!(upload.body, MappedBody::None));
    assert!(m_has(
        &m,
        &upload,
        |a| matches!(a, Attention::UnsupportedBody { mode, .. } if mode == "file")
    ));
    let count = req(&m, "Count");
    assert!(m_has(
        &m,
        &count,
        |a| matches!(a, Attention::DynamicVar { name, .. } if name == "$randomColor")
    ));
}

#[test]
fn literal_bearer_is_scrubbed_into_a_secret() {
    let m = mapped("edge.postman_collection.json");
    let r = req(&m, "Literal auth");
    assert!(
        r.headers
            .iter()
            .all(|h| !matches!(&h.value, FieldValue::Text(v) if v.contains("abc")))
    );
    assert!(m.variables.iter().any(|v| v.secret && v.value == "abc"));
    assert_eq!(text(&r.headers[0]), "Bearer ${API_TOKEN}");
}

#[test]
fn bearer_and_basic_become_auth_with_secret_variables() {
    let m = mapped("edge.postman_collection.json");
    assert!(matches!(&req(&m, "Bearer").auth, Some(MappedAuth::Bearer(t)) if t == "${token}"));
    assert!(matches!(
        &req(&m, "Basic").auth,
        Some(MappedAuth::Basic { username, password })
            if username == "${API_USER}" && password == "${password}"
    ));
    let var = |n: &str| m.variables.iter().find(|v| v.name == n).unwrap();
    assert!(var("API_USER").secret && var("API_USER").value == "user");
    // Referenced by a credential but not defined: a secret to set.
    assert!(var("token").secret && var("password").secret);
    for name in ["token", "password"] {
        assert!(
            m.report
                .attention
                .iter()
                .any(|a| matches!(a, Attention::SetSecret { var } if var == name))
        );
        assert!(
            !m.report
                .attention
                .iter()
                .any(|a| matches!(a, Attention::SetVariable { var } if var == name))
        );
    }
}

#[test]
fn renames_reserved_and_invalid_names() {
    let m = mapped("edge.postman_collection.json");
    assert!(m.report.attention.iter().any(
        |a| matches!(a, Attention::Renamed { from, to } if from == "UUID" && to == "UUID_VAR")
    ));
    assert!(m.variables.iter().any(|v| v.name == "UUID_VAR"));
    assert_eq!(map::var_name("my-var.x"), "my_var_x");
    assert_eq!(map::var_name("TIMESTAMP"), "TIMESTAMP_VAR");
    assert_eq!(map::var_name("store.token"), "store_token_VAR");
    assert_eq!(map::var_name("baseUrl"), "baseUrl");
}

#[test]
fn path_variables_become_file_variables() {
    let r = req(&mapped("edge.postman_collection.json"), "Get item");
    assert_eq!(r.url, "${baseUrl}/items/${id}");
    assert_eq!(r.file_vars, vec![("id".to_string(), "42".to_string())]);
}

#[test]
fn other_body_modes() {
    let m = mapped("edge.postman_collection.json");
    let form = req(&m, "Form");
    assert!(matches!(&form.body, MappedBody::Text(t) if t == "grant_type=client_credentials"));
    assert!(
        form.headers
            .iter()
            .any(|h| h.key == "Content-Type" && text(h) == "application/x-www-form-urlencoded")
    );
    assert!(form.comments.iter().any(|c| c.contains("scope=read")));
    let gql = req(&m, "Query");
    assert!(matches!(&gql.body, MappedBody::Json(v)
        if v["query"] == "query { viewer { id } }" && v["variables"] == serde_json::json!({})));
    assert_eq!(req(&m, "Plain string").url, "https://api.example.com/plain");
    assert_eq!(
        req(&m, "Deep").folders,
        ["Nested", "Level two", "Level three"]
    );
    assert_eq!(m.report.folders, 6);
}

#[test]
fn literal_secret_params_and_apikey_in_query() {
    let json = r#"{"info":{"name":"q","schema":"https://schema.getpostman.com/json/collection/v2.1.0/collection.json"},
      "variable":[{"key":"API_KEY","value":"other"}],
      "auth":{"type":"apikey","apikey":[{"key":"in","value":"query"},{"key":"key","value":"key"},{"key":"value","value":"k1"}]},
      "item":[{"name":"S","request":{"method":"GET","header":[{"key":"content-type","value":"text/plain"}],
        "url":"https://api.example.com/s?token=t1&page=2",
        "body":{"mode":"raw","raw":"hi","options":{"raw":{"language":"json"}}}}}]}"#;
    let m = map_collection(&parse_collection(json).unwrap());
    let r = &m.requests[0];
    let params: Vec<_> = r.params.iter().map(|p| (p.key.as_str(), text(p))).collect();
    assert_eq!(
        params,
        [
            ("key", "${API_KEY_2}"),
            ("token", "${TOKEN}"),
            ("page", "2")
        ]
    );
    assert_eq!(r.headers.len(), 1); // content-type already set, any case
    let var = |n: &str| m.variables.iter().find(|v| v.name == n).unwrap();
    assert!(!var("API_KEY").secret);
    assert!(var("API_KEY_2").secret && var("API_KEY_2").value == "k1");
    assert!(var("TOKEN").secret && var("TOKEN").value == "t1");
}

#[test]
fn attention_lines_read_plainly() {
    let at = |a: Attention| a.to_string();
    assert_eq!(
        at(Attention::PickFile {
            request: "Convert document".into(),
            field: "file".into()
        }),
        "Pick a file for \"file\" in Convert document"
    );
    assert_eq!(
        at(Attention::SetSecret {
            var: "API_KEY".into()
        }),
        "Set the secret API_KEY"
    );
    assert_eq!(
        at(Attention::Examples {
            at: "A/B".into(),
            count: 2
        }),
        "Skipped 2 saved examples in A/B"
    );
    assert_eq!(
        at(Attention::Script { at: String::new() }),
        "Skipped a script in the collection"
    );
}

#[test]
fn imported_var_debug_hides_the_value() {
    let m = mapped("sample.postman_collection.json");
    assert!(!format!("{:?} {:?}", m.variables, m.report).contains("test-key-123"));
    let e = mapped("edge.postman_collection.json");
    assert!(!format!("{:?} {:?}", e.variables, e.report).contains("abc"));
}
