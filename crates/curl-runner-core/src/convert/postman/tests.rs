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
            Attention::Script { at, .. }
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
    let scripts: Vec<_> = m
        .report
        .attention
        .iter()
        .filter(|a| matches!(a, Attention::Script { at, .. } if *at == scripted.path()))
        .collect();
    // The empty `test` event is not reported.
    assert_eq!(
        scripts,
        [&Attention::Script {
            at: "Requests/Scripted".into(),
            listen: "prerequest".into()
        }]
    );
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
        at(Attention::Script {
            at: String::new(),
            listen: "other".into()
        }),
        "Skipped a script in the collection"
    );
    assert_eq!(
        at(Attention::Script {
            at: "Documents/Upload".into(),
            listen: "prerequest".into()
        }),
        "Skipped a pre-request script in Documents/Upload"
    );
    assert_eq!(
        at(Attention::Script {
            at: "A".into(),
            listen: "test".into()
        }),
        "Skipped a test script in A"
    );
}

#[test]
fn imported_var_debug_hides_the_value() {
    let m = mapped("sample.postman_collection.json");
    assert!(!format!("{:?} {:?}", m.variables, m.report).contains("test-key-123"));
    let e = mapped("edge.postman_collection.json");
    assert!(!format!("{:?} {:?}", e.variables, e.report).contains("abc"));
}

#[test]
fn event_exec_is_a_string_or_lines() {
    let json = r#"{"info":{"name":"e","schema":"https://schema.getpostman.com/json/collection/v2.1.0/collection.json"},
      "event":[{"listen":"test","script":{"exec":"pm.test('ok');"}}],
      "item":[{"name":"F","event":[{"listen":"prerequest","script":{"exec":" "}},{"listen":"test"}],
        "item":[{"name":"R","request":"https://api.example.com/r"}]}]}"#;
    let c = parse_collection(json).unwrap();
    assert!(c.event[0].has_code());
    let m = map_collection(&c);
    assert_eq!(
        m.report.attention,
        [Attention::Script {
            at: String::new(),
            listen: "test".into()
        }]
    );
}

#[test]
fn secret_form_fields_are_scrubbed() {
    let m = mapped("sample.postman_collection.json");
    let r = req(&m, "Convert document");
    let MappedBody::Form(f) = &r.body else {
        panic!()
    };
    let pw = f.iter().find(|x| x.key == "password").unwrap();
    assert!(pw.disabled);
    assert_eq!(text(pw), "${PASSWORD}");
    let var = m.variables.iter().find(|v| v.name == "PASSWORD").unwrap();
    assert!(var.secret && var.value == "secret");

    let json = r#"{"info":{"name":"u","schema":"https://schema.getpostman.com/json/collection/v2.1.0/collection.json"},
      "item":[{"name":"T","request":{"method":"POST","url":"https://api.example.com/t",
        "body":{"mode":"urlencoded","urlencoded":[
          {"key":"client_secret","value":"cs1"},{"key":"Token","value":"t1"},
          {"key":"password","value":"p1","disabled":true}]}}}]}"#;
    let u = map_collection(&parse_collection(json).unwrap());
    let r = &u.requests[0];
    assert!(matches!(&r.body, MappedBody::Text(t) if t == "client_secret=cs1&Token=${TOKEN}"));
    assert!(
        r.comments
            .iter()
            .any(|c| c == "Disabled form field: password=${PASSWORD}")
    );
    assert!(u.variables.iter().any(|v| v.secret && v.value == "p1"));

    // No literal secret anywhere in either fixture's mapped requests.
    for (file, secrets) in [
        (
            "sample.postman_collection.json",
            &["test-key-123", "secret"][..],
        ),
        ("edge.postman_collection.json", &["abc"][..]),
    ] {
        for r in mapped(file).requests {
            let fields = r.headers.iter().chain(&r.params).chain(match &r.body {
                MappedBody::Form(f) => f.as_slice(),
                _ => &[],
            });
            for f in fields {
                for s in secrets {
                    assert!(
                        !matches!(&f.value, FieldValue::Text(v) if v == s || v.contains(&format!(" {s}"))),
                        "{}",
                        r.name
                    );
                    assert_ne!(text(f), *s);
                }
            }
            for s in secrets {
                assert!(!format!("{:?}", r.comments).contains(s));
            }
        }
    }
}

// ---- emit ----

/// Every request of both fixtures, emitted.
fn all_emitted() -> Vec<(MappedRequest, String)> {
    [
        "sample.postman_collection.json",
        "edge.postman_collection.json",
    ]
    .into_iter()
    .flat_map(|f| mapped(f).requests)
    .map(|r| {
        let y = emit_request(&r, "Sample");
        (r, y)
    })
    .collect()
}

/// Load emitted YAML the way a run does: file variables and `env`
/// interpolated, defaults merged, then typed.
fn load_with_env(yaml: &str, env: &dyn Fn(&str) -> Option<String>) -> crate::model::RequestConfig {
    let raw = crate::parser::parse_raw(yaml).unwrap();
    let file = crate::parser::prepare_raw(
        &raw,
        Path::new("emitted.yaml"),
        &crate::config::DefaultsLayers::default(),
        Some(env),
    )
    .unwrap();
    let ctx = crate::interpolate::InterpCtx::new(&file.variables, None, env);
    crate::interpolate::interpolate_request_json(&file.requests[0], &ctx).unwrap()
}

#[test]
fn every_emitted_file_reads_as_yaml() {
    for (r, y) in all_emitted() {
        crate::parser::parse_raw_stream(&y).unwrap_or_else(|e| panic!("{}: {e:?}\n{y}", r.name));
        // And types as a request.
        let raw = crate::parser::parse_raw(&y).unwrap();
        crate::parser::from_raw(raw, Path::new("emitted.yaml"))
            .unwrap_or_else(|e| panic!("{}: {e:?}\n{y}", r.name));
    }
}

/// The disabled fields of `r`, as (section, key) in emitted order.
fn disabled_fields(r: &MappedRequest) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    for (section, fields) in [("headers", &r.headers), ("params", &r.params)] {
        out.extend(
            fields
                .iter()
                .filter(|f| f.disabled)
                .map(|f| (section, f.key.clone())),
        );
    }
    if let MappedBody::Form(f) = &r.body {
        out.extend(
            f.iter()
                .filter(|f| f.disabled)
                .map(|f| ("formData", f.key.clone())),
        );
    }
    out
}

/// Lines that are a commented-out field: indented, `# ` (not `# #`), and
/// what follows reads as a one-entry map. Paired with the section (the
/// nearest `  key:` line above) and the field key.
fn commented_fields(y: &str) -> Vec<(usize, String, String)> {
    let mut section = String::new();
    let mut out = Vec::new();
    for (i, l) in y.lines().enumerate() {
        let t = l.trim_start();
        let indent = l.len() - t.len();
        if indent == 2 && !t.starts_with('#') {
            section = t.split(':').next().unwrap().to_string();
        }
        let Some(rest) = t.strip_prefix("# ") else {
            continue;
        };
        if indent == 0 || rest.starts_with('#') {
            continue;
        }
        if let Ok(serde_json::Value::Object(o)) = crate::parser::parse_raw(rest)
            && o.len() == 1
        {
            out.push((i, section.clone(), o.keys().next().unwrap().clone()));
        }
    }
    out
}

#[test]
fn uncommenting_any_disabled_line_still_parses_and_adds_it() {
    let mut seen = 0;
    for (r, y) in all_emitted() {
        let want = disabled_fields(&r);
        let found = commented_fields(&y);
        let got: Vec<_> = found
            .iter()
            .map(|(_, s, k)| (s.as_str(), k.clone()))
            .collect();
        assert_eq!(got, want, "{}:\n{y}", r.name);
        for (i, section, key) in found {
            let lines: Vec<String> = y
                .lines()
                .enumerate()
                .map(|(j, l)| {
                    if j == i {
                        l.replacen("# ", "", 1)
                    } else {
                        l.to_string()
                    }
                })
                .collect();
            let text = lines.join("\n");
            let v = crate::parser::parse_raw_stream(&text)
                .unwrap_or_else(|e| panic!("{}: {e:?}\n{text}", r.name));
            assert!(
                v["request"][&section].get(&key).is_some(),
                "{}: {section}.{key} missing\n{text}",
                r.name
            );
            let raw = crate::parser::parse_raw(&text).unwrap();
            crate::parser::from_raw(raw, Path::new("emitted.yaml")).unwrap();
            seen += 1;
        }
    }
    // Sample: starting_after, watermark, password; plus whatever edge has.
    assert!(seen >= 3, "{seen}");
}

#[test]
fn notes_sit_above_their_field() {
    let r = req(
        &mapped("sample.postman_collection.json"),
        "Convert document",
    );
    let y = emit_request(&r, "Sample");
    let MappedBody::Form(f) = &r.body else {
        panic!()
    };
    let lines: Vec<&str> = y.lines().collect();
    for field in f.iter().filter(|x| x.note.is_some()) {
        let note = field.note.as_deref().unwrap().lines().last().unwrap();
        let prefix = if field.disabled { "    # # " } else { "    # " };
        let at = lines
            .iter()
            .position(|l| *l == format!("{prefix}{note}"))
            .unwrap_or_else(|| panic!("note for {}\n{y}", field.key));
        let next = lines[at + 1];
        let field_line = if field.disabled { "    # " } else { "    " };
        assert!(
            next.starts_with(&format!("{field_line}{}:", field.key)),
            "{next}\n{y}"
        );
    }
}

#[test]
fn keeps_postman_order_and_descriptions() {
    let r = req(&mapped("sample.postman_collection.json"), "Create customer");
    let y = emit_request(&r, "Sample");
    assert!(
        y.starts_with("# Imported from Postman: Sample\n# Creates a customer.\n\n"),
        "{y}"
    );
    assert!(y.find("X-Api-Key").unwrap() < y.find("Content-Type").unwrap()); // fixture order
    let keys = ["name:", "method:", "url:", "headers:", "body:"];
    let at: Vec<_> = keys
        .iter()
        .map(|k| y.find(&format!("  {k}")).unwrap())
        .collect();
    assert!(at.windows(2).all(|w| w[0] < w[1]), "{y}");
}

#[test]
fn the_runner_reads_what_postman_would_send() {
    let m = mapped("sample.postman_collection.json");
    let r = req(&m, "Create customer");
    let y = emit_request(&r, "Sample");
    let env = |k: &str| match k {
        "baseUrl" => Some("https://api.example.com".into()),
        "apiKey" => Some("k".into()),
        _ => None,
    };
    let parsed = load_with_env(&y, &env);
    assert_eq!(parsed.url, "https://api.example.com/v1/customers");
    let headers = parsed.headers.unwrap();
    assert_eq!(headers.get("X-Api-Key").map(String::as_str), Some("k"));
    let MappedBody::Json(body) = &r.body else {
        panic!()
    };
    assert_eq!(parsed.body.as_ref(), Some(body));
}

#[test]
fn path_variables_go_under_global() {
    let r = req(&mapped("edge.postman_collection.json"), "Get item");
    let y = emit_request(&r, "Edge");
    assert!(y.contains("\nglobal:\n  variables:\n"), "{y}");
    let parsed = load_with_env(&y, &|k| {
        (k == "baseUrl").then(|| "https://api.example.com".into())
    });
    assert_eq!(parsed.url, "https://api.example.com/items/42");
}

#[test]
fn auth_and_form_files_use_runner_shapes() {
    for (r, y) in all_emitted() {
        let raw = crate::parser::parse_raw(&y).unwrap();
        let file = crate::parser::from_raw(raw, Path::new("emitted.yaml")).unwrap();
        let got = &file.requests[0];
        match &r.auth {
            Some(MappedAuth::Bearer(t)) => {
                let a = got.auth.as_ref().unwrap();
                assert_eq!(
                    (a.auth_type.as_str(), a.token.as_ref()),
                    ("bearer", Some(t))
                );
            }
            Some(MappedAuth::Basic { username, password }) => {
                let a = got.auth.as_ref().unwrap();
                assert_eq!(a.auth_type, "basic");
                assert_eq!(a.username.as_ref(), Some(username));
                assert_eq!(a.password.as_ref(), Some(password));
            }
            None => assert!(got.auth.is_none(), "{}", r.name),
        }
        if let MappedBody::Form(fields) = &r.body {
            let form = got.form_data.clone().unwrap_or_default();
            for f in fields.iter().filter(|f| !f.disabled) {
                let v = form.get(&f.key).unwrap_or_else(|| panic!("{}\n{y}", f.key));
                match (&f.value, v) {
                    (FieldValue::Text(t), crate::model::FormFieldValue::Text(g)) => {
                        assert_eq!(t, g)
                    }
                    (FieldValue::File { path, .. }, crate::model::FormFieldValue::File(g)) => {
                        assert_eq!(path, &g.file)
                    }
                    other => panic!("{}: {other:?}", f.key),
                }
            }
        }
    }
}

#[test]
fn text_bodies_and_odd_values_round_trip() {
    let base = req(&mapped("sample.postman_collection.json"), "Health");
    let odd = [
        "line one\nline two",
        "trailing newline\n",
        "two trailing\n\n",
        "  leading spaces\nnext",
        "\nstarts with a newline",
        "tab\tand \"quotes\" and \\ back",
        "- a list?",
        "# not a comment",
        "yes",
        "0123",
        "",
        "a: b",
        "${baseUrl}/x?y=${id}",
        "{\"json\": ${id}}",
        "windows\r\nline",
        "lone\rcr and\u{2028}sep",
    ];
    for s in odd {
        let mut r = base.clone();
        r.body = MappedBody::Text(s.into());
        r.comments = vec![s.into()];
        r.headers = vec![Field {
            key: "X-Odd".into(),
            value: FieldValue::Text(s.into()),
            disabled: false,
            note: Some(s.into()),
        }];
        let y = emit_request(&r, s);
        let v = crate::parser::parse_raw_stream(&y).unwrap_or_else(|e| panic!("{s:?}: {e:?}\n{y}"));
        assert_eq!(v["request"]["body"], serde_json::json!(s), "{s:?}\n{y}");
        assert_eq!(
            v["request"]["headers"]["X-Odd"],
            serde_json::json!(s),
            "{s:?}\n{y}"
        );
    }
}

#[test]
fn multi_line_text_is_a_block_scalar() {
    let mut r = req(&mapped("sample.postman_collection.json"), "Health");
    r.body = MappedBody::Text("<a>\n  <b/>\n</a>\n".into());
    let y = emit_request(&r, "Sample");
    assert!(
        y.contains("  body: |\n    <a>\n      <b/>\n    </a>\n"),
        "{y}"
    );
}

#[test]
fn references_stay_unquoted() {
    let r = req(&mapped("sample.postman_collection.json"), "Create customer");
    let y = emit_request(&r, "Sample");
    assert!(y.contains("  url: ${baseUrl}/v1/customers\n"), "{y}");
    assert!(y.contains("    X-Api-Key: ${apiKey}\n"), "{y}");
}
#[test]
fn a_map_of_only_disabled_entries_is_still_written() {
    let mut r = req(&mapped("sample.postman_collection.json"), "Health");
    let off = |k: &str| Field {
        key: k.into(),
        value: FieldValue::Text("v".into()),
        disabled: true,
        note: None,
    };
    r.headers = vec![off("X-One"), off("X-Two")];
    let y = emit_request(&r, "Sample");
    assert!(
        y.contains("  headers:\n    # X-One: v\n    # X-Two: v\n"),
        "{y}"
    );
    let raw = crate::parser::parse_raw(&y).unwrap();
    crate::parser::from_raw(raw, Path::new("emitted.yaml")).unwrap();
    let found = commented_fields(&y);
    assert_eq!(found.len(), 2);
    for (i, _, key) in found {
        let text: Vec<String> = y
            .lines()
            .enumerate()
            .map(|(j, l)| {
                if j == i {
                    l.replacen("# ", "", 1)
                } else {
                    l.to_string()
                }
            })
            .collect();
        let v = crate::parser::parse_raw_stream(&text.join("\n")).unwrap();
        assert_eq!(v["request"]["headers"][&key], "v");
    }
}

#[test]
fn writes_the_sample_tree_and_env() {
    let dir = tempfile::tempdir().unwrap();
    let imp = convert_postman(&fixture("sample.postman_collection.json")).unwrap();
    let out = write_postman(dir.path(), Path::new("api"), &imp, "set it in .env.local").unwrap();
    assert_eq!(out.written.len(), 5);
    let env = std::fs::read_to_string(dir.path().join(".env")).unwrap();
    assert!(env.contains("baseUrl="));
    assert!(env.contains("# apiKey="));
    assert!(!env.contains("test-key-123"));
    for f in &out.written {
        assert!(
            !std::fs::read_to_string(dir.path().join(f))
                .unwrap()
                .contains("test-key-123")
        );
    }
}

#[test]
fn same_names_and_empty_slugs_get_distinct_files() {
    let imp = convert_postman(&fixture("edge.postman_collection.json")).unwrap();
    let paths: std::collections::HashSet<_> =
        imp.files.iter().map(|f| f.rel_path.clone()).collect();
    assert_eq!(paths.len(), imp.files.len());
    assert!(
        imp.files
            .iter()
            .all(|f| f.rel_path.file_stem().is_some_and(|s| !s.is_empty()))
    );
}

#[test]
fn importing_twice_overwrites_nothing() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join(".env"),
        "baseUrl=https://mine.example.com\n",
    )
    .unwrap();
    let imp = convert_postman(&fixture("sample.postman_collection.json")).unwrap();
    let first = write_postman(dir.path(), Path::new(""), &imp, "x").unwrap();
    let again = write_postman(dir.path(), Path::new(""), &imp, "x").unwrap();
    assert!(again.written.iter().all(|p| !first.written.contains(p)));
    assert!(
        again
            .written
            .iter()
            .any(|p| p.to_string_lossy().ends_with("-2.yaml"))
    );
    assert!(
        std::fs::read_to_string(dir.path().join(".env"))
            .unwrap()
            .starts_with("baseUrl=https://mine.example.com\n")
    );
    assert_eq!(first.kept_env, vec!["baseUrl".to_string()]);
}

#[test]
fn names_never_escape_the_target_folder() {
    let dir = tempfile::tempdir().unwrap();
    let imp = convert_postman(&fixture("edge.postman_collection.json")).unwrap();
    let out = write_postman(dir.path(), Path::new("in"), &imp, "x").unwrap();
    for p in &out.written {
        assert!(
            dir.path()
                .join(p)
                .canonicalize()
                .unwrap()
                .starts_with(dir.path().join("in").canonicalize().unwrap())
        );
        assert!(
            p.components()
                .all(|c| matches!(c, std::path::Component::Normal(_)))
        );
    }
}

fn collection(items: &str) -> String {
    format!(
        r#"{{"info":{{"name":"T","schema":"https://schema.getpostman.com/json/collection/v2.1.0/collection.json"}},"item":{items}}}"#
    )
}

#[test]
fn hostile_names_become_plain_components() {
    let req = |n: &str| {
        format!(r#"{{"name":{n:?},"request":{{"method":"GET","url":"https://api.example.com"}}}}"#)
    };
    let names = [
        "..",
        ".",
        "",
        "/etc/passwd",
        "a\\..\\b",
        "C:\\x",
        "~",
        "Get User",
        "🚀",
    ];
    let items: Vec<String> = names
        .iter()
        .map(|n| format!(r#"{{"name":{n:?},"item":[{}]}}"#, req(n)))
        .collect();
    let imp = convert_postman(&collection(&format!("[{}]", items.join(",")))).unwrap();
    assert_eq!(imp.files.len(), names.len());
    for f in &imp.files {
        assert_eq!(f.rel_path.components().count(), 2, "{:?}", f.rel_path);
        for c in f.rel_path.components() {
            let std::path::Component::Normal(s) = c else {
                panic!("{:?}", f.rel_path)
            };
            let s = s.to_str().unwrap();
            assert!(!s.is_empty() && !s.starts_with('-') && !s.starts_with('.'));
            assert!(
                s.bytes().all(|b| b.is_ascii_lowercase()
                    || b.is_ascii_digit()
                    || b == b'-'
                    || b == b'.')
            );
        }
    }
    let paths: Vec<_> = imp.files.iter().map(|f| f.rel_path.clone()).collect();
    assert!(paths.contains(&std::path::PathBuf::from("get-user/get-user.yaml")));
    assert!(paths.contains(&std::path::PathBuf::from("etc-passwd/etc-passwd.yaml")));
    assert!(paths.contains(&std::path::PathBuf::from("folder/request.yaml")));
    assert!(paths.contains(&std::path::PathBuf::from("folder/request-2.yaml")));
}

#[test]
fn refuses_a_folder_outside_the_root() {
    let dir = tempfile::tempdir().unwrap();
    let imp = convert_postman(&fixture("sample.postman_collection.json")).unwrap();
    for bad in ["../out", "a/../../out", "/tmp/out"] {
        let err = write_postman(dir.path(), Path::new(bad), &imp, "x").unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput, "{bad}");
    }
    assert!(std::fs::read_dir(dir.path()).unwrap().next().is_none());
}

#[test]
fn env_values_round_trip_through_the_runner_loader() {
    let dir = tempfile::tempdir().unwrap();
    let values = [
        ("plain", "https://api.example.com"),
        ("spaced", "two words"),
        ("hash", "a#b"),
        ("dq", "say \"hi\""),
        ("sq", "it's"),
        ("lead", "  padded "),
        ("quoted", "\"x\""),
        ("slash", "C:\\new\\table"),
        ("spaced_slash", "C:\\new folder"),
        ("newline", "one\ntwo"),
        ("empty", ""),
    ];
    let vars: Vec<String> = values
        .iter()
        .map(|(k, v)| format!(r#"{{"key":{k:?},"value":{v:?}}}"#))
        .collect();
    let json = collection("[]").replacen(
        r#""item":[]"#,
        &format!(r#""item":[],"variable":[{}]"#, vars.join(",")),
        1,
    );
    std::fs::write(dir.path().join(".env"), "existing=1").unwrap();
    let imp = convert_postman(&json).unwrap();
    write_postman(dir.path(), Path::new(""), &imp, "x").unwrap();
    let loaded = crate::env::load_env_files(dir.path(), None);
    assert_eq!(loaded["existing"], "1");
    for (k, v) in values {
        assert_eq!(loaded[k], v, "{k}");
    }
    let env = std::fs::read_to_string(dir.path().join(".env")).unwrap();
    assert!(env.contains("\n# Imported from Postman: T\nplain=https://api.example.com\n"));
    assert!(env.contains("\nspaced=\"two words\"\n"));
    assert!(env.contains("\nspaced_slash='C:\\new folder'\n"));
}

#[test]
fn secrets_get_a_note_and_same_values_are_not_kept() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(".env"), "baseUrl=https://api.example.com\n").unwrap();
    let imp = convert_postman(&fixture("sample.postman_collection.json")).unwrap();
    let out = write_postman(dir.path(), Path::new(""), &imp, "set it in .env.local").unwrap();
    assert!(out.kept_env.is_empty());
    let env = std::fs::read_to_string(dir.path().join(".env")).unwrap();
    assert_eq!(
        env,
        "baseUrl=https://api.example.com\n\
         # Imported from Postman: Example API\n\
         # apiKey is a secret: set it in .env.local\n\
         # apiKey=\n\
         # PASSWORD is a secret: set it in .env.local\n\
         # PASSWORD=\n"
    );
    // Nothing left to add: the file stays as it is.
    write_postman(dir.path(), Path::new(""), &imp, "x").unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join(".env")).unwrap(),
        env
    );
}

#[test]
fn readme_is_written_once() {
    let dir = tempfile::tempdir().unwrap();
    let imp = convert_postman(&fixture("sample.postman_collection.json")).unwrap();
    assert_eq!(
        imp.readme.as_deref(),
        Some("# Example API\n\nA small example collection.\n")
    );
    assert!(
        write_postman(dir.path(), Path::new("api"), &imp, "x")
            .unwrap()
            .readme
    );
    std::fs::write(dir.path().join("api/README.md"), "mine").unwrap();
    assert!(
        !write_postman(dir.path(), Path::new("api"), &imp, "x")
            .unwrap()
            .readme
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("api/README.md")).unwrap(),
        "mine"
    );
}

#[test]
fn folders_with_empty_slugs_fall_back_to_folder() {
    let imp = convert_postman(&fixture("edge.postman_collection.json")).unwrap();
    let paths: Vec<_> = imp.files.iter().map(|f| f.rel_path.clone()).collect();
    assert!(paths.contains(&std::path::PathBuf::from("escape/inside.yaml")));
    let req = r#"{"name":"Ping","request":{"method":"GET","url":"https://api.example.com"}}"#;
    let items = format!(r#"[{{"name":"🚀","item":[{req}]}},{{"name":"✨","item":[{req}]}}]"#);
    let imp = convert_postman(&collection(&items)).unwrap();
    let paths: Vec<_> = imp.files.iter().map(|f| f.rel_path.clone()).collect();
    assert_eq!(
        paths,
        [
            std::path::PathBuf::from("folder/ping.yaml"),
            std::path::PathBuf::from("folder/ping-2.yaml")
        ]
    );
}
