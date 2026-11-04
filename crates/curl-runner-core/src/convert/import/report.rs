//! What an import converted, and what it left for the user to do.
//!
//! Attention entries hold names and paths only, never a variable's value.

use std::fmt;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ImportReport {
    pub folders: usize,
    pub requests: usize,
    pub attention: Vec<Attention>,
}

/// One thing the import couldn't finish. `request` and `at` are a request's
/// or folder's path joined with `/`; an empty `at` is the collection itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Attention {
    PickFile {
        request: String,
        field: String,
    },
    /// A file field that held `count` files; only the first was kept.
    ExtraFiles {
        request: String,
        field: String,
        count: usize,
    },
    SetSecret {
        var: String,
    },
    SetVariable {
        var: String,
    },
    Renamed {
        from: String,
        to: String,
    },
    /// A collection variable `from` whose name `to` an earlier one took.
    SkippedVariable {
        from: String,
        to: String,
    },
    /// `listen` is Postman's event kind: `prerequest`, `test`, ….
    Script {
        at: String,
        listen: String,
    },
    UnsupportedAuth {
        at: String,
        kind: String,
    },
    UnsupportedBody {
        at: String,
        mode: String,
    },
    DynamicVar {
        at: String,
        name: String,
    },
    Examples {
        at: String,
        count: usize,
    },
    KeptEnv {
        var: String,
    },
}

struct Place<'a>(&'a str);

impl fmt::Display for Place<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            f.write_str("the collection")
        } else {
            f.write_str(self.0)
        }
    }
}

impl fmt::Display for Attention {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Attention::PickFile { request, field } => {
                write!(f, "Pick a file for \"{field}\" in {}", Place(request))
            }
            Attention::ExtraFiles {
                request,
                field,
                count,
            } => write!(
                f,
                "Kept the first of {count} files for \"{field}\" in {}. Add the others by hand.",
                Place(request)
            ),
            Attention::SetSecret { var } => write!(f, "Set the secret {var}"),
            Attention::SetVariable { var } => write!(f, "Set the variable {var}"),
            Attention::Renamed { from, to } => write!(f, "Renamed the variable {from} to {to}"),
            Attention::SkippedVariable { from, to } => write!(
                f,
                "Skipped the variable {from}: another variable is already named {to}"
            ),
            Attention::Script { at, listen } => {
                let kind = match listen.as_str() {
                    "prerequest" => "pre-request ",
                    "test" => "test ",
                    _ => "",
                };
                write!(f, "Skipped a {kind}script in {}", Place(at))
            }
            Attention::UnsupportedAuth { at, kind } => {
                write!(f, "Skipped {kind} auth in {}. Add it by hand.", Place(at))
            }
            Attention::UnsupportedBody { at, mode } => {
                write!(f, "Skipped a {mode} body in {}", Place(at))
            }
            Attention::DynamicVar { at, name } => {
                write!(
                    f,
                    "Kept {{{{{name}}}}} in {}. It has no curl-runner match.",
                    Place(at)
                )
            }
            Attention::Examples { at, count } => {
                let s = if *count == 1 { "" } else { "s" };
                write!(f, "Skipped {count} saved example{s} in {}", Place(at))
            }
            Attention::KeptEnv { var } => {
                write!(f, "Kept the value of {var} already in .env")
            }
        }
    }
}
