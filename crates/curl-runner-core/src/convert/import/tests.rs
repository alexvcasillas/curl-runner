use super::*;

fn request() -> MappedRequest {
    MappedRequest {
        folders: Vec::new(),
        name: "List pets".into(),
        comments: Vec::new(),
        method: "GET".into(),
        url: "${API_BASE_URL}/pets".into(),
        headers: Vec::new(),
        params: Vec::new(),
        body: MappedBody::None,
        auth: None,
        file_vars: Vec::new(),
        expect_status: None,
    }
}

#[test]
fn emit_writes_expect_status() {
    let r = MappedRequest {
        expect_status: Some(200),
        ..request()
    };
    let y = emit_request(&r, "OpenAPI", "Pets");
    assert!(y.starts_with("# Imported from OpenAPI: Pets\n"), "{y}");
    assert!(y.ends_with("  expect:\n    status: 200\n"), "{y}");
}

#[test]
fn emit_without_expect_status_has_no_expect() {
    let y = emit_request(&request(), "Postman", "Pets");
    assert!(!y.contains("expect:"), "{y}");
}

#[test]
fn expect_status_follows_the_body() {
    let r = MappedRequest {
        body: MappedBody::Text("hi".into()),
        expect_status: Some(201),
        ..request()
    };
    let y = emit_request(&r, "OpenAPI", "Pets");
    assert!(
        y.ends_with("  body: hi\n  expect:\n    status: 201\n"),
        "{y}"
    );
}

#[test]
fn plan_import_names_files_and_source() {
    let mapped = Mapped {
        requests: vec![MappedRequest {
            folders: vec!["Pet Store".into()],
            ..request()
        }],
        variables: Vec::new(),
        description: Some("About".into()),
        env_comments: Vec::new(),
        report: ImportReport::default(),
    };
    let imp = plan_import("My  API".into(), mapped, "OpenAPI");
    assert_eq!(imp.files.len(), 1);
    assert_eq!(
        imp.files[0].rel_path,
        std::path::Path::new("pet-store/list-pets.yaml")
    );
    assert!(
        imp.files[0]
            .yaml
            .starts_with("# Imported from OpenAPI: My  API\n")
    );
    assert_eq!(imp.readme.as_deref(), Some("# My API\n\nAbout\n"));
}

#[test]
fn env_comments_go_above_new_keys_only() {
    let mapped = Mapped {
        requests: Vec::new(),
        variables: vec![
            ImportedVar {
                name: "PETS_BASE_URL".into(),
                value: "https://api.example.com".into(),
                secret: false,
            },
            ImportedVar {
                name: "PETS_API_KEY".into(),
                value: String::new(),
                secret: true,
            },
            ImportedVar {
                name: "OLD".into(),
                value: "1".into(),
                secret: false,
            },
        ],
        description: None,
        env_comments: vec![
            (
                "PETS_BASE_URL".into(),
                "Other server: https://b.example.com".into(),
            ),
            ("PETS_API_KEY".into(), "Sent as X-API-Key".into()),
            ("OLD".into(), "never written".into()),
        ],
        report: ImportReport::default(),
    };
    let imp = plan_import("Pets".into(), mapped, "OpenAPI");
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(".env"), "OLD=1\n").unwrap();
    write_import(dir.path(), std::path::Path::new("pets"), &imp, "note").unwrap();
    let env = std::fs::read_to_string(dir.path().join(".env")).unwrap();
    assert_eq!(
        env,
        "OLD=1\n# Imported from OpenAPI: Pets\n\
         # Other server: https://b.example.com\nPETS_BASE_URL=https://api.example.com\n\
         # Sent as X-API-Key\n# PETS_API_KEY is a secret: note\n# PETS_API_KEY=\n"
    );
}

#[test]
fn env_header_names_the_source_format() {
    let mapped = Mapped {
        requests: Vec::new(),
        variables: vec![ImportedVar {
            name: "A".into(),
            value: "1".into(),
            secret: false,
        }],
        description: None,
        env_comments: Vec::new(),
        report: ImportReport::default(),
    };
    let imp = plan_import("Pets".into(), mapped, "OpenAPI");
    let dir = tempfile::tempdir().unwrap();
    write_import(dir.path(), std::path::Path::new("pets"), &imp, "note").unwrap();
    let env = std::fs::read_to_string(dir.path().join(".env")).unwrap();
    assert_eq!(env, "# Imported from OpenAPI: Pets\nA=1\n");
}
