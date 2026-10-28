//! The thin interactive layer. All decisions live in `prompts.rs`, `mod.rs`
//! and the pure builder; this only asks and shows.
//!
//! * [`CliclackUi`]: `cliclack` prompts (the `@clack/prompts` look), used on
//!   a terminal.
//! * [`LineUi`]: when stdin or stderr is not a terminal. One line of stdin
//!   answers one prompt; an empty line takes the default; end of input
//!   cancels (so a closed or empty stdin ends promptly instead of hanging).
//! * `ScriptedUi` (tests): canned answers.

use std::io::{self, BufRead, Write};
use std::rc::Rc;

/// Why a prompt did not produce an answer.
#[derive(Debug)]
pub enum Stop {
    /// Ctrl-C, Esc, or end of input (clack's `isCancel`).
    Cancelled,
    Io(io::Error),
}

impl From<io::Error> for Stop {
    fn from(e: io::Error) -> Self {
        if e.kind() == io::ErrorKind::Interrupted {
            Stop::Cancelled
        } else {
            Stop::Io(e)
        }
    }
}

pub type Res<T> = Result<T, Stop>;

/// A check on typed text: the error message, or `None` when it is fine.
pub type Validator = Rc<dyn Fn(&str) -> Option<String>>;

#[derive(Debug, Clone, PartialEq)]
pub struct Choice {
    pub value: String,
    pub label: String,
    pub hint: String,
}

impl Choice {
    pub fn new(value: &str, label: &str, hint: &str) -> Self {
        Choice {
            value: value.into(),
            label: label.into(),
            hint: hint.into(),
        }
    }
}

pub trait Ui {
    fn intro(&mut self, title: &str);
    fn outro(&mut self, message: &str);
    /// `p.cancel`: the closing line of an aborted wizard.
    fn cancel(&mut self, message: &str);
    fn info(&mut self, message: &str);
    fn step(&mut self, message: &str);
    fn success(&mut self, message: &str);
    fn warn(&mut self, message: &str);
    /// A line on stdout (`console.log`).
    fn print(&mut self, line: &str);

    /// Free text; `initial` is the value taken on an empty answer.
    fn text(
        &mut self,
        message: &str,
        placeholder: &str,
        initial: &str,
        validate: Option<Validator>,
    ) -> Res<String>;
    fn password(&mut self, message: &str, validate: Option<Validator>) -> Res<String>;
    fn select(&mut self, message: &str, options: &[Choice], initial: Option<&str>) -> Res<String>;
    fn multiselect(&mut self, message: &str, options: &[Choice]) -> Res<Vec<String>>;
    fn confirm(&mut self, message: &str, initial: bool) -> Res<bool>;
}

// ---------------------------------------------------------------------------
// cliclack

pub struct CliclackUi;

fn hinted(c: &Choice) -> (String, String, String) {
    (c.value.clone(), c.label.clone(), c.hint.clone())
}

impl Ui for CliclackUi {
    fn intro(&mut self, title: &str) {
        let _ = cliclack::intro(title);
    }
    fn outro(&mut self, message: &str) {
        let _ = cliclack::outro(message);
    }
    fn cancel(&mut self, message: &str) {
        let _ = cliclack::outro_cancel(message);
    }
    fn info(&mut self, message: &str) {
        let _ = cliclack::log::info(message);
    }
    fn step(&mut self, message: &str) {
        let _ = cliclack::log::step(message);
    }
    fn success(&mut self, message: &str) {
        let _ = cliclack::log::success(message);
    }
    fn warn(&mut self, message: &str) {
        let _ = cliclack::log::warning(message);
    }
    fn print(&mut self, line: &str) {
        println!("{line}");
    }

    fn text(
        &mut self,
        message: &str,
        placeholder: &str,
        initial: &str,
        validate: Option<Validator>,
    ) -> Res<String> {
        let mut p = cliclack::input(message).required(false);
        if !placeholder.is_empty() {
            p = p.placeholder(placeholder);
        }
        if !initial.is_empty() {
            p = p.default_input(initial);
        }
        if let Some(v) = validate {
            p = p.validate(move |s: &String| v(s).map_or(Ok(()), Err));
        }
        Ok(p.interact::<String>()?)
    }

    fn password(&mut self, message: &str, validate: Option<Validator>) -> Res<String> {
        let mut p = cliclack::password(message).allow_empty();
        if let Some(v) = validate {
            p = p.validate(move |s: &String| v(s).map_or(Ok(()), Err));
        }
        Ok(p.interact()?)
    }

    fn select(&mut self, message: &str, options: &[Choice], initial: Option<&str>) -> Res<String> {
        let mut p = cliclack::select(message);
        for c in options {
            let (v, l, h) = hinted(c);
            p = p.item(v, l, h);
        }
        if let Some(i) = initial {
            p = p.initial_value(i.to_string());
        }
        Ok(p.interact()?)
    }

    fn multiselect(&mut self, message: &str, options: &[Choice]) -> Res<Vec<String>> {
        let mut p = cliclack::multiselect(message).required(false);
        for c in options {
            let (v, l, h) = hinted(c);
            p = p.item(v, l, h);
        }
        Ok(p.interact()?)
    }

    fn confirm(&mut self, message: &str, initial: bool) -> Res<bool> {
        Ok(cliclack::confirm(message)
            .initial_value(initial)
            .interact()?)
    }
}

// ---------------------------------------------------------------------------
// line mode

/// Prompts answered from lines of input (no terminal needed).
pub struct LineUi {
    input: Box<dyn BufRead>,
    err: Box<dyn Write>,
    out: Box<dyn Write>,
}

impl LineUi {
    pub fn new(input: Box<dyn BufRead>, err: Box<dyn Write>, out: Box<dyn Write>) -> Self {
        LineUi { input, err, out }
    }

    pub fn stdio() -> Self {
        LineUi::new(
            Box::new(io::BufReader::new(io::stdin())),
            Box::new(io::stderr()),
            Box::new(io::stdout()),
        )
    }

    fn say(&mut self, prefix: &str, message: &str) {
        let _ = writeln!(self.err, "{prefix}  {message}");
    }

    /// The next line, without its line break; end of input cancels.
    fn line(&mut self) -> Res<String> {
        let mut s = String::new();
        match self.input.read_line(&mut s) {
            Ok(0) => Err(Stop::Cancelled),
            Ok(_) => {
                while s.ends_with(['\n', '\r']) {
                    s.pop();
                }
                Ok(s)
            }
            Err(e) => Err(e.into()),
        }
    }

    fn ask(&mut self, message: &str, hint: &str) {
        let _ = if hint.is_empty() {
            write!(self.err, "{message}: ")
        } else {
            write!(self.err, "{message} [{hint}]: ")
        };
        let _ = self.err.flush();
    }
}

/// An option by value (any case) or 1-based number.
fn pick<'a>(options: &'a [Choice], answer: &str) -> Option<&'a Choice> {
    let a = answer.trim();
    if let Ok(n) = a.parse::<usize>()
        && n >= 1
        && n <= options.len()
    {
        return Some(&options[n - 1]);
    }
    options.iter().find(|c| c.value.eq_ignore_ascii_case(a))
}

impl Ui for LineUi {
    fn intro(&mut self, title: &str) {
        self.say("┌", title);
    }
    fn outro(&mut self, message: &str) {
        self.say("└", message);
    }
    fn cancel(&mut self, message: &str) {
        self.say("└", message);
    }
    fn info(&mut self, message: &str) {
        self.say("●", message);
    }
    fn step(&mut self, message: &str) {
        self.say("◇", message);
    }
    fn success(&mut self, message: &str) {
        self.say("◆", message);
    }
    fn warn(&mut self, message: &str) {
        self.say("▲", message);
    }
    fn print(&mut self, line: &str) {
        let _ = writeln!(self.out, "{line}");
        let _ = self.out.flush();
    }

    fn text(
        &mut self,
        message: &str,
        placeholder: &str,
        initial: &str,
        validate: Option<Validator>,
    ) -> Res<String> {
        loop {
            self.ask(
                message,
                if initial.is_empty() {
                    placeholder
                } else {
                    initial
                },
            );
            let mut v = self.line()?;
            if v.is_empty() {
                v = initial.to_string();
            }
            match validate.as_ref().and_then(|f| f(&v)) {
                Some(err) => self.say("▲", &err),
                None => return Ok(v),
            }
        }
    }

    fn password(&mut self, message: &str, validate: Option<Validator>) -> Res<String> {
        loop {
            self.ask(message, "");
            let v = self.line()?;
            match validate.as_ref().and_then(|f| f(&v)) {
                Some(err) => self.say("▲", &err),
                None => return Ok(v),
            }
        }
    }

    fn select(&mut self, message: &str, options: &[Choice], initial: Option<&str>) -> Res<String> {
        let names: Vec<&str> = options.iter().map(|c| c.value.as_str()).collect();
        let default = initial
            .filter(|i| names.contains(i))
            .unwrap_or_else(|| names.first().copied().unwrap_or(""))
            .to_string();
        loop {
            self.ask(&format!("{message} ({})", names.join("/")), &default);
            let v = self.line()?;
            if v.trim().is_empty() {
                return Ok(default);
            }
            match pick(options, &v) {
                Some(c) => return Ok(c.value.clone()),
                None => self.say("▲", "Choose one of the listed values"),
            }
        }
    }

    fn multiselect(&mut self, message: &str, options: &[Choice]) -> Res<Vec<String>> {
        let names: Vec<&str> = options.iter().map(|c| c.value.as_str()).collect();
        loop {
            self.ask(
                &format!("{message} ({}, comma separated)", names.join("/")),
                "none",
            );
            let v = self.line()?;
            let parts: Vec<&str> = v
                .split(',')
                .map(str::trim)
                .filter(|p| !p.is_empty())
                .collect();
            let picked: Vec<Option<&Choice>> = parts.iter().map(|p| pick(options, p)).collect();
            if picked.iter().all(Option::is_some) {
                return Ok(picked
                    .into_iter()
                    .flatten()
                    .map(|c| c.value.clone())
                    .collect());
            }
            self.say("▲", "Choose from the listed values");
        }
    }

    fn confirm(&mut self, message: &str, initial: bool) -> Res<bool> {
        loop {
            self.ask(message, if initial { "Y/n" } else { "y/N" });
            let v = self.line()?;
            match v.trim().to_lowercase().as_str() {
                "" => return Ok(initial),
                "y" | "yes" | "true" => return Ok(true),
                "n" | "no" | "false" => return Ok(false),
                _ => self.say("▲", "Answer y or n"),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// tests

#[cfg(test)]
pub use scripted::{Ans, ScriptedUi};

#[cfg(test)]
mod scripted {
    use super::*;
    use std::collections::VecDeque;

    /// A canned answer.
    #[derive(Debug, Clone)]
    pub enum Ans {
        /// Text; the empty string takes the prompt's default.
        T(&'static str),
        /// A select value.
        S(&'static str),
        Y,
        N,
        M(&'static [&'static str]),
        Cancel,
    }

    #[derive(Default)]
    pub struct ScriptedUi {
        answers: VecDeque<Ans>,
        /// Every prompt asked and message shown, in order.
        pub log: Vec<String>,
        /// Lines printed to stdout.
        pub printed: Vec<String>,
    }

    impl ScriptedUi {
        pub fn new(answers: Vec<Ans>) -> Self {
            ScriptedUi {
                answers: answers.into(),
                ..Default::default()
            }
        }

        pub fn leftover(&self) -> usize {
            self.answers.len()
        }

        fn next(&mut self, prompt: &str) -> Res<Ans> {
            self.log.push(format!("ask:{prompt}"));
            match self.answers.pop_front() {
                Some(Ans::Cancel) | None => Err(Stop::Cancelled),
                Some(a) => Ok(a),
            }
        }

        fn note(&mut self, kind: &str, m: &str) {
            self.log.push(format!("{kind}:{m}"));
        }
    }

    impl Ui for ScriptedUi {
        fn intro(&mut self, m: &str) {
            self.note("intro", m);
        }
        fn outro(&mut self, m: &str) {
            self.note("outro", m);
        }
        fn cancel(&mut self, m: &str) {
            self.note("cancel", m);
        }
        fn info(&mut self, m: &str) {
            self.note("info", m);
        }
        fn step(&mut self, m: &str) {
            self.note("step", m);
        }
        fn success(&mut self, m: &str) {
            self.note("success", m);
        }
        fn warn(&mut self, m: &str) {
            self.note("warn", m);
        }
        fn print(&mut self, l: &str) {
            self.printed.push(l.to_string());
        }

        fn text(
            &mut self,
            message: &str,
            _placeholder: &str,
            initial: &str,
            validate: Option<Validator>,
        ) -> Res<String> {
            loop {
                let Ans::T(v) = self.next(message)? else {
                    panic!("text prompt {message:?} got a non-text answer");
                };
                let v = if v.is_empty() { initial } else { v };
                match validate.as_ref().and_then(|f| f(v)) {
                    Some(err) => self.log.push(format!("invalid:{err}")),
                    None => return Ok(v.to_string()),
                }
            }
        }

        fn password(&mut self, message: &str, validate: Option<Validator>) -> Res<String> {
            self.text(message, "", "", validate)
        }

        fn select(
            &mut self,
            message: &str,
            options: &[Choice],
            initial: Option<&str>,
        ) -> Res<String> {
            let Ans::S(v) = self.next(message)? else {
                panic!("select prompt {message:?} got a non-select answer");
            };
            if v.is_empty() {
                return Ok(initial
                    .filter(|i| options.iter().any(|c| c.value == *i))
                    .unwrap_or(&options[0].value)
                    .to_string());
            }
            assert!(
                options.iter().any(|c| c.value == v),
                "{v:?} is not an option of {message:?}"
            );
            Ok(v.to_string())
        }

        fn multiselect(&mut self, message: &str, options: &[Choice]) -> Res<Vec<String>> {
            let Ans::M(vs) = self.next(message)? else {
                panic!("multiselect prompt {message:?} got a non-multi answer");
            };
            for v in vs {
                assert!(options.iter().any(|c| c.value == *v), "{v:?} not an option");
            }
            Ok(vs.iter().map(|v| v.to_string()).collect())
        }

        fn confirm(&mut self, message: &str, initial: bool) -> Res<bool> {
            self.log.push(format!("initial:{initial}"));
            match self.next(message)? {
                Ans::Y => Ok(true),
                Ans::N => Ok(false),
                Ans::T("") => Ok(initial),
                a => panic!("confirm prompt {message:?} got {a:?}"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line_ui(input: &str) -> (LineUi, Rc<std::cell::RefCell<Vec<u8>>>) {
        struct Sink(Rc<std::cell::RefCell<Vec<u8>>>);
        impl Write for Sink {
            fn write(&mut self, b: &[u8]) -> io::Result<usize> {
                self.0.borrow_mut().extend_from_slice(b);
                Ok(b.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let err = Rc::new(std::cell::RefCell::new(Vec::new()));
        let ui = LineUi::new(
            Box::new(io::Cursor::new(input.as_bytes().to_vec())),
            Box::new(Sink(err.clone())),
            Box::new(io::sink()),
        );
        (ui, err)
    }

    fn opts() -> Vec<Choice> {
        vec![
            Choice::new("GET", "GET", ""),
            Choice::new("POST", "POST", ""),
        ]
    }

    #[test]
    fn line_text_default_validation_and_eof() {
        let nonempty: Validator = Rc::new(|v| v.is_empty().then(|| "required".to_string()));
        // empty line, no default: the validator rejects it, the next line passes
        let (mut ui, err) = line_ui("\nvalue\n");
        assert_eq!(
            ui.text("Q", "", "", Some(nonempty.clone())).unwrap(),
            "value"
        );
        assert!(String::from_utf8_lossy(&err.borrow()).contains("required"));
        let (mut ui, _) = line_ui("\n");
        assert_eq!(ui.text("Q", "", "dflt", None).unwrap(), "dflt");
        let (mut ui, _) = line_ui("");
        assert!(matches!(
            ui.text("Q", "", "", Some(nonempty)),
            Err(Stop::Cancelled)
        ));
    }

    #[test]
    fn line_select_by_value_number_default() {
        let (mut ui, _) = line_ui("post\n2\n\nnope\nGET\n");
        assert_eq!(ui.select("M", &opts(), None).unwrap(), "POST");
        assert_eq!(ui.select("M", &opts(), None).unwrap(), "POST");
        assert_eq!(ui.select("M", &opts(), Some("POST")).unwrap(), "POST");
        assert_eq!(ui.select("M", &opts(), None).unwrap(), "GET");
    }

    #[test]
    fn line_confirm_and_multiselect() {
        let (mut ui, _) = line_ui("y\nNo\n\nmaybe\ny\n1,post\n\n");
        assert!(ui.confirm("C", false).unwrap());
        assert!(!ui.confirm("C", true).unwrap());
        assert!(ui.confirm("C", true).unwrap());
        assert!(ui.confirm("C", false).unwrap()); // "maybe" re-asked, then y
        assert_eq!(ui.multiselect("M", &opts()).unwrap(), ["GET", "POST"]);
        assert!(ui.multiselect("M", &opts()).unwrap().is_empty());
        assert!(matches!(ui.confirm("C", true), Err(Stop::Cancelled)));
    }
}
