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
    /// A `$ref` that points nowhere (or outside the document) in `count`
    /// requests. `examples` holds up to 3 request paths.
    UnresolvedRef {
        target: String,
        external: bool,
        count: usize,
        examples: Vec<String>,
    },
    SkippedBody {
        media: String,
        count: usize,
        examples: Vec<String>,
    },
    /// `count` requests used the `used` security requirement and also
    /// accept `others`.
    AuthChoice {
        used: String,
        others: String,
        count: usize,
    },
    /// `kind` is `OAuth 2` or `OpenID Connect`.
    AccessToken {
        var: String,
        kind: String,
    },
    UnsupportedScheme {
        scheme: String,
        kind: String,
        count: usize,
    },
    OwnServer {
        url: String,
        count: usize,
    },
    ServerVariable {
        name: String,
    },
    /// `count` counts parameters, `examples` their requests.
    ParamStyle {
        count: usize,
        examples: Vec<String>,
    },
    CookieParams {
        count: usize,
        examples: Vec<String>,
    },
    BodyShapes {
        count: usize,
        examples: Vec<String>,
    },
    /// `what` is `webhook`, `callback` or `link`.
    NotConverted {
        what: String,
        count: usize,
    },
    BadOperation {
        at: String,
        reason: String,
    },
    /// Parameters with no name or an unknown `in`; `count` counts
    /// parameters, `examples` their requests.
    BadParams {
        count: usize,
        examples: Vec<String>,
    },
}

/// `1 request`, `2 requests`.
struct Count<'a>(usize, &'a str);

impl fmt::Display for Count<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = if self.0 == 1 { "" } else { "s" };
        write!(f, "{} {}{s}", self.0, self.1)
    }
}

/// `a`, `a and b`, `a, b and c`.
struct Examples<'a>(&'a [String]);

impl fmt::Display for Examples<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            [] => Ok(()),
            [one] => f.write_str(one),
            [init @ .., last] => write!(f, "{} and {last}", init.join(", ")),
        }
    }
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
            Attention::UnresolvedRef {
                target,
                external,
                count,
                examples,
            } => {
                let reqs = Count(*count, "request");
                let like = Examples(examples);
                if *external {
                    write!(
                        f,
                        "Can't follow the external reference {target} in {reqs}, like {like}. Used an empty value."
                    )
                } else {
                    write!(
                        f,
                        "Couldn't resolve {target} in {reqs}, like {like}. Used an empty value."
                    )
                }
            }
            Attention::SkippedBody {
                media,
                count,
                examples,
            } => write!(
                f,
                "Skipped {media} bodies in {}, like {}. Add them by hand.",
                Count(*count, "request"),
                Examples(examples)
            ),
            Attention::AuthChoice {
                used,
                others,
                count,
            } => write!(
                f,
                "Used {used} in {} that also accept{} {others}",
                Count(*count, "request"),
                if *count == 1 { "s" } else { "" }
            ),
            Attention::AccessToken { var, kind } => {
                write!(f, "Put an {kind} access token in {var}")
            }
            Attention::UnsupportedScheme {
                scheme,
                kind,
                count,
            } => write!(
                f,
                "Skipped {kind} auth ({scheme}) in {}. Add it by hand.",
                Count(*count, "request")
            ),
            Attention::OwnServer { url, count } => {
                let verb = if *count == 1 { "uses its" } else { "use their" };
                write!(f, "{} {verb} own server {url}", Count(*count, "request"))
            }
            Attention::ServerVariable { name } => write!(
                f,
                "The server variable {name} has no default. Edit the URLs that use it."
            ),
            Attention::ParamStyle { count, examples } => {
                let (verb, check) = if *count == 1 {
                    ("uses", "its value")
                } else {
                    ("use", "their values")
                };
                write!(
                    f,
                    "{} {verb} a style curl-runner doesn't model, like in {}. Check {check}.",
                    Count(*count, "parameter"),
                    Examples(examples)
                )
            }
            Attention::CookieParams { count, examples } => write!(
                f,
                "Left out optional cookies in {}, like {}",
                Count(*count, "request"),
                Examples(examples)
            ),
            Attention::BodyShapes { count, examples } => write!(
                f,
                "Used the first of several body shapes in {}, like {}",
                Count(*count, "request"),
                Examples(examples)
            ),
            Attention::NotConverted { what, count } => {
                write!(f, "Skipped {}", Count(*count, what))
            }
            Attention::BadOperation { at, reason } => write!(f, "Skipped {at}: {reason}"),
            Attention::BadParams { count, examples } => write!(
                f,
                "Skipped {} with no name or an unknown location, like in {}",
                Count(*count, "parameter"),
                Examples(examples)
            ),
        }
    }
}
