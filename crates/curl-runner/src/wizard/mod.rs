//! Interactive wizard (`curl-runner init|edit`, `wizard/index.ts` and the
//! `init`/`edit` handlers of `cli.ts`).
//!
//! The flow is plain code over [`ui::Ui`]; answers become a config value
//! (`yaml_builder`), then text, which is checked to read back as that config
//! before it is written atomically. See `crates/CHANGES-FROM-TS.md`.

pub mod edit;
pub mod parser;
pub mod prompts;
pub mod templates;
pub mod types;
pub mod ui;
pub mod yaml_builder;

use crate::args::{parse_edit_args, parse_init_args};
use crate::colors::Color;
use crate::output::{Console, Logger, OutCfg};
use curl_runner_core::VERSION;
use curl_runner_core::model::Json;
use std::io::IsTerminal;
use std::path::Path;
use templates::get_template;
use types::{WizardAnswers, WizardOptions};
use ui::{Choice, Stop, Ui};

/// How a wizard ends early.
#[derive(Debug)]
pub enum Exit {
    /// `p.cancel(msg)` then exit 0 (also Ctrl-C and end of input).
    Cancelled(&'static str),
    /// `p.cancel(msg)` then exit 1 (TS: file not found).
    CancelledFailure(String),
    /// An error for the top-level handler: `\u{2717} msg`, exit 1.
    Fail(String),
}

type Flow<T> = Result<T, Exit>;

/// Run the finished file (`bun run <cli> <file>` in TS): this executable.
pub trait Runner {
    fn run_file(&mut self, file: &str);
}

struct SelfRunner;

impl Runner for SelfRunner {
    fn run_file(&mut self, file: &str) {
        if let Ok(exe) = std::env::current_exe() {
            let _ = std::process::Command::new(exe).arg(file).status();
        }
    }
}

/// Map a prompt failure: a cancel ends with `message`, an I/O error fails.
fn ask<T>(r: ui::Res<T>, message: &'static str) -> Flow<T> {
    r.map_err(|e| match e {
        Stop::Cancelled => Exit::Cancelled(message),
        Stop::Io(e) => Exit::Fail(format!("Cannot read the answer: {e}")),
    })
}

const CANCELLED: &str = "Wizard cancelled";

/// `runPromptFlow`.
fn prompt_flow(ui: &mut dyn Ui, defaults: &WizardAnswers) -> Flow<WizardAnswers> {
    let mut answers = defaults.clone();
    ask(
        prompts::prompt_basics(ui, defaults, &mut answers),
        CANCELLED,
    )?;

    let headers = ask(
        prompts::prompt_headers(ui, defaults.headers.as_ref()),
        CANCELLED,
    )?;
    if headers.is_some() {
        answers.headers = headers;
    }

    if !["GET", "HEAD", "OPTIONS"].contains(&answers.method.as_str()) {
        ask(prompts::prompt_body(ui, defaults, &mut answers), CANCELLED)?;
    }
    ask(prompts::prompt_auth(ui, defaults, &mut answers), CANCELLED)?;
    ask(
        prompts::prompt_advanced(ui, defaults, &mut answers),
        CANCELLED,
    )?;
    ask(prompts::prompt_retry(ui, defaults, &mut answers), CANCELLED)?;
    ask(
        prompts::prompt_validation(ui, defaults, &mut answers),
        CANCELLED,
    )?;
    Ok(answers)
}

/// `runWizard`.
pub fn run_wizard(
    ui: &mut dyn Ui,
    mut options: WizardOptions,
    runner: &mut dyn Runner,
) -> Flow<()> {
    ui.intro(&format!("curl-runner wizard v{VERSION}"));

    let answers;
    let mut loaded = None;
    if let Some(file) = options.edit_file.clone() {
        let path = Path::new(&file);
        if !parser::file_exists(path) {
            return Err(Exit::CancelledFailure(format!("File not found: {file}")));
        }
        ui.info(&format!("Editing: {file}"));
        let l = parser::load_file(path).map_err(Exit::Fail)?;
        answers = prompt_flow(ui, &l.answers)?;
        loaded = Some(l);
        options.output_path = options.output_path.or(Some(file));
    } else {
        let template = ask(prompts::prompt_template(ui), CANCELLED)?;
        let defaults = template
            .map(|t| get_template(t).defaults)
            .unwrap_or_default();
        answers = prompt_flow(ui, &defaults)?;
    }

    let (output_path, run_after) = ask(
        prompts::prompt_output(ui, options.output_path.as_deref()),
        CANCELLED,
    )?;

    // What gets written: the whole document with the edited request updated
    // (edit), or a new `request:` document.
    let data: Json = match &loaded {
        Some(l) => l.apply(&answers),
        None => yaml_builder::build_yaml_config(&answers),
    };
    let yaml = edit::render_checked(&data)
        .map_err(|e| Exit::Fail(format!("Refusing to save {output_path}: {e}")))?;

    ui.step("Preview:");
    ui.print("");
    ui.print(&yaml);
    ui.print("");

    let save = ask(
        ui.confirm(&format!("Save to {output_path}?"), true),
        CANCELLED,
    )?;
    if !save {
        return Err(Exit::Cancelled(CANCELLED));
    }

    edit::write_atomic(Path::new(&output_path), &yaml)
        .map_err(|e| Exit::Fail(format!("Cannot write {output_path}: {e}")))?;
    ui.success(&format!("Saved to {output_path}"));

    if run_after {
        ui.step("Running request...");
        ui.print("");
        runner.run_file(&output_path);
    }
    ui.outro("Done!");
    Ok(())
}

/// `quickInit`.
pub fn quick_init(
    ui: &mut dyn Ui,
    url: Option<&str>,
    output: Option<&str>,
    runner: &mut dyn Runner,
) -> Flow<()> {
    const CANCEL: &str = "Cancelled";
    ui.intro("curl-runner quick init");

    let url = match url {
        Some(u) if !u.is_empty() => u.to_string(),
        _ => ask(
            ui.text(
                "Request URL",
                "https://api.example.com/endpoint",
                "",
                Some(std::rc::Rc::new(prompts::validate_url)),
            ),
            CANCEL,
        )?,
    };
    let methods: Vec<Choice> = ["GET", "POST", "PUT", "DELETE"]
        .iter()
        .map(|m| Choice::new(m, m, ""))
        .collect();
    let method = ask(ui.select("HTTP method", &methods, Some("GET")), CANCEL)?;
    let filename = ask(
        ui.text(
            "Save to",
            "request.yaml",
            output.filter(|o| !o.is_empty()).unwrap_or("request.yaml"),
            Some(std::rc::Rc::new(prompts::validate_filename)),
        ),
        CANCEL,
    )?;

    let answers = WizardAnswers {
        url,
        method,
        ..Default::default()
    };
    let yaml = edit::render_checked(&yaml_builder::build_yaml_config(&answers))
        .map_err(|e| Exit::Fail(format!("Refusing to save {filename}: {e}")))?;
    edit::write_atomic(Path::new(&filename), &yaml)
        .map_err(|e| Exit::Fail(format!("Cannot write {filename}: {e}")))?;
    ui.success(&format!("Created {filename}"));

    // A cancelled "Run now?" still ends with `Done!` (TS).
    match ui.confirm("Run now?", true) {
        Err(_) => {}
        Ok(true) => {
            ui.print("");
            runner.run_file(&filename);
        }
        Ok(false) => {}
    }
    ui.outro("Done!");
    Ok(())
}

/// Report how a flow ended and give the exit code.
fn finish(r: Flow<()>, ui: &mut dyn Ui, log: &Logger, console: &mut Console) -> i32 {
    match r {
        Ok(()) => 0,
        Err(Exit::Cancelled(msg)) => {
            ui.cancel(msg);
            0
        }
        Err(Exit::CancelledFailure(msg)) => {
            ui.cancel(&msg);
            1
        }
        Err(Exit::Fail(msg)) => {
            log.log_error(console, &msg);
            1
        }
    }
}

fn help(log: &Logger, edit: bool) -> String {
    let c = |t: &str, col| log.color(t, col);
    if edit {
        format!(
            "\n{}\n\n{}\n  curl-runner edit <file.yaml> [options]\n\n{}\n  -o, --output <file>   Save to different file (default: overwrite original)\n  -h, --help            Show this help\n\n{}\n  # Edit existing file\n  curl-runner edit api-test.yaml\n\n  # Edit and save to new file\n  curl-runner edit api-test.yaml -o api-test-v2.yaml\n",
            c("\u{1F680} CURL RUNNER - EDIT", Color::Bright),
            c("USAGE:", Color::Yellow),
            c("OPTIONS:", Color::Yellow),
            c("EXAMPLES:", Color::Yellow),
        )
    } else {
        format!(
            "\n{}\n\n{}\n  curl-runner init [url] [options]\n\n{}\n  -w, --wizard          Launch full interactive wizard\n  -o, --output <file>   Output file path (default: request.yaml)\n  -h, --help            Show this help\n\n{}\n  # Quick init with prompts\n  curl-runner init\n\n  # Quick init with URL\n  curl-runner init https://api.example.com/users\n\n  # Full wizard mode\n  curl-runner init --wizard\n\n  # Wizard with custom output\n  curl-runner init -w -o api-test.yaml\n",
            c("\u{1F680} CURL RUNNER - INIT", Color::Bright),
            c("USAGE:", Color::Yellow),
            c("OPTIONS:", Color::Yellow),
            c("EXAMPLES:", Color::Yellow),
        )
    }
}

/// `curl-runner init|edit ...`; `argv[0]` is the subcommand. Returns the exit
/// code. With a terminal the prompts are `cliclack`; without one (piped or
/// closed stdin) they read lines from stdin and end the wizard at end of
/// input instead of waiting.
pub fn run(argv: &[String], color: bool, console: &mut Console) -> i32 {
    let log = Logger::new(OutCfg::from_config(None), color, None);
    let is_edit = argv.first().is_some_and(|a| a == "edit");

    if argv.iter().any(|a| a == "--help" || a == "-h") {
        console.out(&help(&log, is_edit));
        return 0;
    }

    let interactive = std::io::stdin().is_terminal() && std::io::stderr().is_terminal();
    let mut ui: Box<dyn Ui> = if interactive {
        Box::new(ui::CliclackUi)
    } else {
        Box::new(ui::LineUi::stdio())
    };
    let mut runner = SelfRunner;
    dispatch(argv, is_edit, ui.as_mut(), &mut runner, &log, console)
}

fn dispatch(
    argv: &[String],
    is_edit: bool,
    ui: &mut dyn Ui,
    runner: &mut dyn Runner,
    log: &Logger,
    console: &mut Console,
) -> i32 {
    let flow = if is_edit {
        let o = parse_edit_args(argv);
        if o.file.is_empty() {
            log.log_error(console, "Usage: curl-runner edit <file.yaml>");
            return 1;
        }
        run_wizard(
            ui,
            WizardOptions {
                edit_file: Some(o.file),
                output_path: o.output_path,
            },
            runner,
        )
    } else {
        let o = parse_init_args(argv);
        if o.wizard {
            run_wizard(
                ui,
                WizardOptions {
                    edit_file: None,
                    output_path: o.output_path,
                },
                runner,
            )
        } else {
            quick_init(ui, o.url.as_deref(), o.output_path.as_deref(), runner)
        }
    };
    finish(flow, ui, log, console)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui::{Ans, ScriptedUi};

    #[derive(Default)]
    struct Rec(Vec<String>);
    impl Runner for Rec {
        fn run_file(&mut self, file: &str) {
            self.0.push(file.to_string());
        }
    }

    fn opts(path: &Path) -> WizardOptions {
        WizardOptions {
            edit_file: None,
            output_path: Some(path.to_string_lossy().into_owned()),
        }
    }

    fn run_init(answers: Vec<Ans>, path: &Path) -> (Flow<()>, ScriptedUi, Rec) {
        let mut ui = ScriptedUi::new(answers);
        let mut rec = Rec::default();
        let r = run_wizard(&mut ui, opts(path), &mut rec);
        (r, ui, rec)
    }

    /// blank template, GET `url`, no headers/auth/advanced/retry/validation.
    fn basics(url: &'static str) -> Vec<Ans> {
        vec![
            Ans::S("blank"),
            Ans::T(url),
            Ans::S("GET"),
            Ans::T(""),     // template, basics
            Ans::N,         // headers
            Ans::S("none"), // auth (GET has no body prompt)
            Ans::N,
            Ans::N,
            Ans::N, // advanced, retry, validation
        ]
    }

    #[test]
    fn init_flow_writes_the_ts_text() {
        let d = tempfile::tempdir().unwrap();
        let out = d.path().join("r.yaml");
        let mut a = basics("https://api.example.com/users");
        a.extend([Ans::T(""), Ans::N, Ans::Y]); // save to (default), run after, confirm
        let (r, ui, rec) = run_init(a, &out);
        r.unwrap();
        assert_eq!(
            std::fs::read_to_string(&out).unwrap(),
            "request:\n  url: \"https://api.example.com/users\""
        );
        assert!(rec.0.is_empty());
        assert_eq!(ui.leftover(), 0);
        assert_eq!(
            ui.printed,
            ["", "request:\n  url: \"https://api.example.com/users\"", ""]
        );
        let asked: Vec<&String> = ui
            .log
            .iter()
            .filter(|l| !l.starts_with("initial:"))
            .collect();
        let log = asked
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("|");
        assert!(log.starts_with(&format!("intro:curl-runner wizard v{VERSION}|ask:Start from a template?|ask:Request URL|ask:HTTP method|ask:Request name (optional)|ask:Add headers?|ask:Authentication|ask:Configure advanced options?|ask:Configure retry strategy?|ask:Add response validation?|ask:Save to file|ask:Run request after creating?|step:Preview:|ask:Save to ")), "{log}");
        assert!(
            log.contains("|success:Saved to ") && log.ends_with("outro:Done!"),
            "{log}"
        );
    }

    #[test]
    fn template_defaults_flow_through_the_prompts() {
        let d = tempfile::tempdir().unwrap();
        let out = d.path().join("r.yaml");
        let (r, ui, _) = run_init(
            vec![
                Ans::S("basic-post"),
                Ans::T("https://x.test/a"),
                Ans::S(""),
                Ans::T("Create"),
                Ans::N, // headers: initial true (template), say no
                Ans::S(""),
                Ans::T("{\"a\": 1}"), // body: json is the default type
                Ans::S("none"),
                Ans::N,
                Ans::N,
                Ans::N,
                Ans::T(""),
                Ans::N,
                Ans::Y,
            ],
            &out,
        );
        r.unwrap();
        assert!(ui.log.contains(&"initial:true".to_string())); // "Add headers?" follows the template
        assert_eq!(
            std::fs::read_to_string(&out).unwrap(),
            "request:\n  url: \"https://x.test/a\"\n  name: Create\n  method: POST\n  headers:\n    Content-Type: application/json\n  body:\n    a: 1"
        );
    }

    #[test]
    fn full_answers_match_the_builder_golden() {
        let d = tempfile::tempdir().unwrap();
        let out = d.path().join("r.yaml");
        let (r, _, rec) = run_init(
            vec![
                Ans::S("blank"),
                Ans::T("https://api.example.com/users"),
                Ans::S("POST"),
                Ans::T("Full"),
                Ans::Y,
                Ans::M(&["accept-json"]),
                Ans::Y,
                Ans::T("X-A"),
                Ans::T("b"),
                Ans::N,
                Ans::S("json"),
                Ans::T("{\"x\": [1, 2]}"),
                Ans::S("bearer"),
                Ans::T("tok"),
                Ans::Y,
                Ans::T("5000"),
                Ans::N,
                Ans::Y,
                Ans::Y,
                Ans::Y,
                Ans::T("3"),
                Ans::T("1000"),
                Ans::T("2"),
                Ans::Y,
                Ans::T("200,201"),
                Ans::T("< 2000"),
                Ans::T(""),
                Ans::Y,
                Ans::Y,
            ],
            &out,
        );
        r.unwrap();
        assert_eq!(rec.0, [out.to_string_lossy().into_owned()]);
        assert_eq!(
            std::fs::read_to_string(&out).unwrap(),
            "request:\n  url: \"https://api.example.com/users\"\n  name: Full\n  method: POST\n  headers:\n    Accept: application/json\n    X-A: b\n  body:\n    x: [1, 2]\n  auth:\n    type: bearer\n    token: tok\n  timeout: 5000\n  followRedirects: false\n  insecure: true\n  http2: true\n  retry:\n    count: 3\n    delay: 1000\n    backoff: 2\n  expect:\n    status: [200, 201]\n    responseTime: < 2000"
        );
    }

    #[test]
    fn cancel_anywhere_exits_cancelled_and_writes_nothing() {
        let d = tempfile::tempdir().unwrap();
        let out = d.path().join("r.yaml");
        let full = {
            let mut a = basics("https://x.io");
            a.extend([Ans::T(""), Ans::N, Ans::Y]);
            a
        };
        for cut in 0..full.len() {
            let mut a = full[..cut].to_vec();
            a.push(Ans::Cancel);
            let (r, ui, _) = run_init(a, &out);
            assert!(
                matches!(r, Err(Exit::Cancelled("Wizard cancelled"))),
                "cut {cut}: {r:?}"
            );
            let _ = ui;
            assert!(!out.exists(), "cut {cut}");
        }
    }

    #[test]
    fn declining_the_save_cancels() {
        let d = tempfile::tempdir().unwrap();
        let out = d.path().join("r.yaml");
        let mut a = basics("https://x.io");
        a.extend([Ans::T(""), Ans::N, Ans::N]);
        let (r, _, _) = run_init(a, &out);
        assert!(matches!(r, Err(Exit::Cancelled("Wizard cancelled"))));
        assert!(!out.exists());
    }

    #[test]
    fn body_prompt_is_skipped_for_get_head_options_only() {
        for (method, asks_body) in [
            ("GET", false),
            ("HEAD", false),
            ("OPTIONS", false),
            ("DELETE", true),
            ("PATCH", true),
        ] {
            let mut answers = vec![Ans::T("https://x.io"), Ans::S(method), Ans::T(""), Ans::N];
            if asks_body {
                answers.push(Ans::S("none"));
            }
            answers.extend([Ans::S("none"), Ans::N, Ans::N, Ans::N]);
            let mut ui = ScriptedUi::new(answers);
            let r = prompt_flow(&mut ui, &WizardAnswers::default());
            let asked = ui.log.iter().any(|l| l == "ask:Request body type");
            assert_eq!(asked, asks_body, "{method}: {r:?}");
        }
    }

    const MULTI: &str = "global:\n  timeout: 9\nrequests:\n  - url: https://a.test\n    method: GET\n    store:\n      id: body.id\n  - url: https://b.test\n    method: POST\n";

    /// all the prompts of an edit with nothing changed except the method.
    fn edit_answers(method: &'static str) -> Vec<Ans> {
        vec![
            Ans::T(""),
            Ans::S(method),
            Ans::T(""), // url, method, name
            Ans::N,     // headers
            Ans::S(""), // auth (initial none)
            Ans::N,
            Ans::N,
            Ans::N,
            Ans::T(""),
            Ans::N,
            Ans::Y, // save to, run after, confirm
        ]
    }

    #[test]
    fn edit_updates_only_the_request_and_keeps_the_rest_of_the_file() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("t.yaml");
        std::fs::write(&f, MULTI).unwrap();
        let mut ui = ScriptedUi::new(edit_answers("HEAD"));
        let mut rec = Rec::default();
        run_wizard(
            &mut ui,
            WizardOptions {
                edit_file: Some(f.to_string_lossy().into_owned()),
                output_path: None,
            },
            &mut rec,
        )
        .unwrap();
        let back = curl_runner_core::parser::load_raw(&f).unwrap();
        assert_eq!(back["global"], serde_json::json!({"timeout": 9}));
        assert_eq!(back["requests"][0]["method"], "HEAD");
        assert_eq!(
            back["requests"][0]["store"],
            serde_json::json!({"id": "body.id"})
        );
        assert_eq!(
            back["requests"][1],
            serde_json::json!({"url": "https://b.test", "method": "POST"})
        );
        assert!(ui.log.iter().any(|l| l.starts_with("info:Editing: ")));
        // no temp files left behind
        assert_eq!(std::fs::read_dir(d.path()).unwrap().count(), 1);
    }

    #[test]
    fn edit_to_another_file_leaves_the_original_alone() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("t.yaml");
        let g = d.path().join("copy.yaml");
        std::fs::write(&f, MULTI).unwrap();
        let mut ui = ScriptedUi::new(edit_answers("OPTIONS"));
        run_wizard(
            &mut ui,
            WizardOptions {
                edit_file: Some(f.to_string_lossy().into_owned()),
                output_path: Some(g.to_string_lossy().into_owned()),
            },
            &mut Rec::default(),
        )
        .unwrap();
        assert_eq!(std::fs::read_to_string(&f).unwrap(), MULTI);
        assert_eq!(
            curl_runner_core::parser::load_raw(&g).unwrap()["requests"][0]["method"],
            "OPTIONS"
        );
    }

    #[test]
    fn edit_never_touches_a_file_it_cannot_edit() {
        let d = tempfile::tempdir().unwrap();
        for (name, src) in [
            ("list.yaml", "- url: a\n- url: b\n"),
            (
                "multidoc.yaml",
                "request:\n  url: a\n---\nrequest:\n  url: b\n",
            ),
            ("norequest.yaml", "global:\n  timeout: 1\n"),
            ("broken.yaml", "request: [\n"),
            ("scalar.yaml", "hello\n"),
        ] {
            let f = d.path().join(name);
            std::fs::write(&f, src).unwrap();
            let mut ui = ScriptedUi::new(edit_answers("GET"));
            let r = run_wizard(
                &mut ui,
                WizardOptions {
                    edit_file: Some(f.to_string_lossy().into_owned()),
                    output_path: None,
                },
                &mut Rec::default(),
            );
            assert!(matches!(r, Err(Exit::Fail(_))), "{name}: {r:?}");
            assert_eq!(std::fs::read_to_string(&f).unwrap(), src, "{name}");
            assert_eq!(
                ui.leftover(),
                edit_answers("GET").len(),
                "{name}: no prompt was shown"
            );
        }
    }

    #[test]
    fn edit_missing_file_is_a_failed_cancel() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("nope.yaml");
        let mut ui = ScriptedUi::new(vec![]);
        let r = run_wizard(
            &mut ui,
            WizardOptions {
                edit_file: Some(f.to_string_lossy().into_owned()),
                output_path: None,
            },
            &mut Rec::default(),
        );
        match r {
            Err(Exit::CancelledFailure(m)) => assert!(m.starts_with("File not found: ")),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn quick_init_writes_and_can_run() {
        let d = tempfile::tempdir().unwrap();
        let out = d.path().join("q.yaml");
        let mut ui = ScriptedUi::new(vec![Ans::S("POST"), Ans::T(""), Ans::Y]);
        let mut rec = Rec::default();
        quick_init(
            &mut ui,
            Some("https://api.example.com/users"),
            Some(&out.to_string_lossy()),
            &mut rec,
        )
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(&out).unwrap(),
            "request:\n  url: \"https://api.example.com/users\"\n  method: POST"
        );
        assert_eq!(rec.0.len(), 1);
        assert!(
            ui.log
                .contains(&format!("success:Created {}", out.display()))
        );
        assert!(ui.log.last().unwrap() == "outro:Done!");
    }

    #[test]
    fn quick_init_asks_for_the_url_and_a_cancelled_run_prompt_still_says_done() {
        let d = tempfile::tempdir().unwrap();
        let out = d.path().join("q.yaml");
        let mut ui = ScriptedUi::new(vec![
            Ans::T("bad"),
            Ans::T("https://x.io"),
            Ans::S(""),
            Ans::T(""),
            Ans::Cancel,
        ]);
        quick_init(
            &mut ui,
            None,
            Some(&out.to_string_lossy()),
            &mut Rec::default(),
        )
        .unwrap();
        assert!(ui.log.contains(&"invalid:Invalid URL format".to_string()));
        assert!(out.exists());
        assert_eq!(ui.log.last().unwrap(), "outro:Done!");
    }

    #[test]
    fn quick_init_cancel_says_cancelled() {
        let mut ui = ScriptedUi::new(vec![Ans::Cancel]);
        let r = quick_init(&mut ui, None, None, &mut Rec::default());
        assert!(matches!(r, Err(Exit::Cancelled("Cancelled"))));
    }

    #[test]
    fn interrupted_io_error_is_a_cancel() {
        let s: Stop = std::io::Error::from(std::io::ErrorKind::Interrupted).into();
        assert!(matches!(s, Stop::Cancelled));
        let s: Stop = std::io::Error::from(std::io::ErrorKind::NotConnected).into();
        assert!(matches!(s, Stop::Io(_)));
    }
}
