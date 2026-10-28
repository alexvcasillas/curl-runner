//! `.env` file loading (`env-file-loader.ts`) and `CURL_RUNNER_*` environment
//! overrides (`env-loader.ts`).
//!
//! Numeric env values follow JS `Number.parseInt`/`parseFloat` (leading
//! numeric prefix). Where TS would store `NaN` the Rust port leaves the field
//! unset, and negative values for unsigned fields are ignored.

use crate::model::{
    CiExitConfig, ConnectionPoolConfig, GlobalConfig, GlobalDiffConfig, GlobalSnapshotConfig,
    OutputConfig, ProfileConfig, RequestConfig, RetryConfig, SecurityConfig, WatchConfig,
};
use indexmap::IndexMap;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// .env files
// ---------------------------------------------------------------------------

/// Parse `.env` content: comments, blank lines and lines without `=` are
/// skipped; matching surrounding quotes are stripped; double-quoted values get
/// `\n \r \t \\` unescaped.
pub fn parse_env_content(content: &str) -> IndexMap<String, String> {
    let mut result = IndexMap::new();
    // JS `trim()` treats U+FEFF as whitespace, so a leading BOM is harmless in TS.
    let content = content.strip_prefix('\u{feff}').unwrap_or(content);
    for line in content.split('\n') {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some(eq) = trimmed.find('=') else {
            continue;
        };
        let key = trimmed[..eq].trim();
        let raw_value = trimmed[eq + 1..].trim();
        let mut value = raw_value.to_string();
        let quoted = (raw_value.starts_with('"') && raw_value.ends_with('"'))
            || (raw_value.starts_with('\'') && raw_value.ends_with('\''));
        if quoted {
            // JS `slice(1, -1)`: a lone quote character becomes empty.
            value = if raw_value.len() >= 2 {
                raw_value[1..raw_value.len() - 1].to_string()
            } else {
                String::new()
            };
        }
        if raw_value.starts_with('"') {
            value = value
                .replace("\\n", "\n")
                .replace("\\r", "\r")
                .replace("\\t", "\t")
                .replace("\\\\", "\\");
        }
        if !key.is_empty() {
            result.insert(key.to_string(), value);
        }
    }
    result
}

fn env_file_candidates(cwd: &Path, environment: Option<&str>) -> Vec<PathBuf> {
    let mut paths = vec![cwd.join(".env"), cwd.join(".env.local")];
    if let Some(e) = environment.filter(|e| !e.is_empty()) {
        paths.push(cwd.join(format!(".env.{e}")));
        paths.push(cwd.join(format!(".env.{e}.local")));
    }
    paths
}

/// Load `.env`, `.env.local`, `.env.{environment}`, `.env.{environment}.local`
/// (later files win). Missing or unreadable files are ignored.
pub fn load_env_files(cwd: &Path, environment: Option<&str>) -> IndexMap<String, String> {
    let mut vars = IndexMap::new();
    for path in env_file_candidates(cwd, environment) {
        if let Ok(content) = std::fs::read_to_string(&path) {
            vars.extend(parse_env_content(&content));
        }
    }
    vars
}

/// The env files from [`load_env_files`] that exist.
pub fn env_file_paths(cwd: &Path, environment: Option<&str>) -> Vec<PathBuf> {
    env_file_candidates(cwd, environment)
        .into_iter()
        .filter(|p| p.is_file())
        .collect()
}

/// Keys to redact in output (`SECRET_` prefix).
pub fn secret_keys(vars: &IndexMap<String, String>) -> Vec<String> {
    vars.keys()
        .filter(|k| k.starts_with("SECRET_"))
        .cloned()
        .collect()
}

// ---------------------------------------------------------------------------
// CURL_RUNNER_* overrides
// ---------------------------------------------------------------------------

/// JS `Number.parseInt(s, 10)`; `None` for NaN.
fn parse_int(s: &str) -> Option<i64> {
    let s = s.trim_start();
    let (neg, rest) = match s.as_bytes().first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    };
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    let n: i64 = digits.parse().ok()?;
    Some(if neg { -n } else { n })
}

/// JS `Number.parseFloat`; `None` for NaN.
fn parse_float(s: &str) -> Option<f64> {
    let s = s.trim_start();
    let b = s.as_bytes();
    let mut i = 0;
    if matches!(b.first(), Some(b'+' | b'-')) {
        i += 1;
    }
    let int_start = i;
    while b.get(i).is_some_and(u8::is_ascii_digit) {
        i += 1;
    }
    let mut digits = i - int_start;
    if b.get(i) == Some(&b'.') {
        let frac_start = i + 1;
        let mut j = frac_start;
        while b.get(j).is_some_and(u8::is_ascii_digit) {
            j += 1;
        }
        if digits > 0 || j > frac_start {
            digits += j - frac_start;
            i = j;
        }
    }
    if digits == 0 {
        return None;
    }
    if matches!(b.get(i), Some(b'e' | b'E')) {
        let mut j = i + 1;
        if matches!(b.get(j), Some(b'+' | b'-')) {
            j += 1;
        }
        let exp_start = j;
        while b.get(j).is_some_and(u8::is_ascii_digit) {
            j += 1;
        }
        if j > exp_start {
            i = j;
        }
    }
    s[..i].parse().ok()
}

fn unsigned(s: &str) -> Option<u64> {
    parse_int(s).and_then(|n| u64::try_from(n).ok())
}

fn is_true(s: &str) -> bool {
    s.to_lowercase() == "true"
}

/// Build the config contributed by `CURL_RUNNER_*` variables (TS
/// `loadEnvironmentConfig`); empty values count as unset.
pub fn load_environment_config(env: &impl Fn(&str) -> Option<String>) -> GlobalConfig {
    let get = |k: &str| env(k).filter(|v| !v.is_empty());
    let mut c = GlobalConfig {
        execution: get("CURL_RUNNER_EXECUTION"),
        continue_on_error: get("CURL_RUNNER_CONTINUE_ON_ERROR").map(|v| is_true(&v)),
        dry_run: get("CURL_RUNNER_DRY_RUN").map(|v| is_true(&v)),
        http2: get("CURL_RUNNER_HTTP2").map(|v| is_true(&v)),
        ..Default::default()
    };

    if let Some(n) = get("CURL_RUNNER_MAX_CONCURRENCY")
        .as_deref()
        .and_then(unsigned)
        .filter(|n| *n > 0)
    {
        c.max_concurrency = Some(n);
    }

    // Connection pool
    let pool = ConnectionPoolConfig {
        enabled: get("CURL_RUNNER_CONNECTION_POOL").map(|v| is_true(&v)),
        max_streams_per_host: get("CURL_RUNNER_MAX_STREAMS_PER_HOST")
            .as_deref()
            .and_then(unsigned),
        keepalive_time: get("CURL_RUNNER_KEEPALIVE_TIME")
            .as_deref()
            .and_then(unsigned),
        connect_timeout: get("CURL_RUNNER_CONNECT_TIMEOUT")
            .as_deref()
            .and_then(unsigned),
    };
    if pool != ConnectionPoolConfig::default() {
        c.connection_pool = Some(pool);
    }

    // Output
    let output = OutputConfig {
        verbose: get("CURL_RUNNER_VERBOSE").map(|v| is_true(&v)),
        format: get("CURL_RUNNER_OUTPUT_FORMAT")
            .filter(|f| ["json", "pretty", "raw"].contains(&f.as_str())),
        pretty_level: get("CURL_RUNNER_PRETTY_LEVEL")
            .filter(|f| ["minimal", "standard", "detailed"].contains(&f.as_str())),
        save_to_file: get("CURL_RUNNER_OUTPUT_FILE"),
        ..Default::default()
    };
    if output != OutputConfig::default() {
        c.output = Some(output);
    }

    // CI
    let ci = CiExitConfig {
        strict_exit: get("CURL_RUNNER_STRICT_EXIT").map(|v| is_true(&v)),
        fail_on: get("CURL_RUNNER_FAIL_ON").as_deref().and_then(unsigned),
        fail_on_percentage: get("CURL_RUNNER_FAIL_ON_PERCENTAGE")
            .as_deref()
            .and_then(parse_float)
            .filter(|p| (0.0..=100.0).contains(p)),
    };
    if ci != CiExitConfig::default() {
        c.ci = Some(ci);
    }

    // Watch
    let watch = WatchConfig {
        enabled: get("CURL_RUNNER_WATCH").map(|v| is_true(&v)),
        debounce: get("CURL_RUNNER_WATCH_DEBOUNCE")
            .as_deref()
            .and_then(unsigned),
        clear: get("CURL_RUNNER_WATCH_CLEAR").map(|v| v.to_lowercase() != "false"),
    };
    if watch != WatchConfig::default() {
        c.watch = Some(watch);
    }

    // Profile: any profile var implies `iterations` (default 10).
    let iterations = get("CURL_RUNNER_PROFILE")
        .as_deref()
        .and_then(unsigned)
        .filter(|n| *n > 0);
    let warmup = get("CURL_RUNNER_PROFILE_WARMUP")
        .as_deref()
        .and_then(unsigned);
    let concurrency = get("CURL_RUNNER_PROFILE_CONCURRENCY")
        .as_deref()
        .and_then(unsigned);
    let histogram = get("CURL_RUNNER_PROFILE_HISTOGRAM").map(|v| is_true(&v));
    let export_file = get("CURL_RUNNER_PROFILE_EXPORT");
    if iterations.is_some()
        || warmup.is_some()
        || concurrency.is_some()
        || histogram.is_some()
        || export_file.is_some()
    {
        c.profile = Some(ProfileConfig {
            iterations: iterations.unwrap_or(10),
            warmup,
            concurrency,
            histogram,
            export_file,
        });
    }

    // Snapshot
    let snapshot = GlobalSnapshotConfig {
        enabled: get("CURL_RUNNER_SNAPSHOT").map(|v| is_true(&v)),
        update_mode: get("CURL_RUNNER_SNAPSHOT_UPDATE")
            .map(|v| v.to_lowercase())
            .filter(|m| ["none", "all", "failing"].contains(&m.as_str())),
        dir: get("CURL_RUNNER_SNAPSHOT_DIR"),
        ci: get("CURL_RUNNER_SNAPSHOT_CI").map(|v| is_true(&v)),
        ..Default::default()
    };
    if snapshot != GlobalSnapshotConfig::default() {
        c.snapshot = Some(snapshot);
    }

    // Diff
    let diff = GlobalDiffConfig {
        enabled: get("CURL_RUNNER_DIFF").map(|v| is_true(&v)),
        save: get("CURL_RUNNER_DIFF_SAVE").map(|v| is_true(&v)),
        label: get("CURL_RUNNER_DIFF_LABEL"),
        compare_with: get("CURL_RUNNER_DIFF_COMPARE"),
        dir: get("CURL_RUNNER_DIFF_DIR"),
        output_format: get("CURL_RUNNER_DIFF_OUTPUT")
            .map(|v| v.to_lowercase())
            .filter(|f| ["terminal", "json", "markdown"].contains(&f.as_str())),
        ..Default::default()
    };
    if diff != GlobalDiffConfig::default() {
        c.diff = Some(diff);
    }

    // Timeout and retries
    let timeout = get("CURL_RUNNER_TIMEOUT").as_deref().and_then(unsigned);
    let retry = RetryConfig {
        count: get("CURL_RUNNER_RETRIES").as_deref().and_then(unsigned),
        delay: get("CURL_RUNNER_RETRY_DELAY").as_deref().and_then(unsigned),
        ..Default::default()
    };
    let retry = (retry != RetryConfig::default()).then_some(retry);
    if timeout.is_some() || retry.is_some() {
        c.defaults = Some(RequestConfig {
            timeout,
            retry,
            ..Default::default()
        });
    }

    // Security
    let protocols: Vec<String> = get("CURL_RUNNER_ALLOWED_PROTOCOLS")
        .map(|v| {
            v.split(',')
                .map(|p| p.trim().to_lowercase())
                .filter(|p| !p.is_empty())
                .collect()
        })
        .unwrap_or_default();
    let security = SecurityConfig {
        allowed_protocols: (!protocols.is_empty()).then_some(protocols),
        allow_paths: get("CURL_RUNNER_ALLOW_PATHS").map(|v| is_true(&v)),
    };
    if security != SecurityConfig::default() {
        c.security = Some(security);
    }

    c
}

/// Apply `CURL_RUNNER_*` overrides on top of `cfg`, with the same semantics as
/// TS `mergeGlobalConfigs(cfg, loadEnvironmentConfig())`: nested groups merge
/// field by field, `profile` is replaced wholesale, scalars override.
pub fn apply_env_overrides(cfg: &mut GlobalConfig, env: &impl Fn(&str) -> Option<String>) {
    let e = load_environment_config(env);

    macro_rules! over {
        ($($f:ident),*) => { $( if e.$f.is_some() { cfg.$f = e.$f.clone(); } )* };
    }
    // Scalars and groups replaced as a whole.
    over!(
        execution,
        continue_on_error,
        dry_run,
        http2,
        max_concurrency,
        profile
    );

    macro_rules! merge_group {
        ($field:ident, $ty:ty, [$($f:ident),*]) => {
            if let Some(o) = &e.$field {
                let b = cfg.$field.get_or_insert_with(<$ty>::default);
                $( if o.$f.is_some() { b.$f = o.$f.clone(); } )*
            }
        };
    }
    merge_group!(
        connection_pool,
        ConnectionPoolConfig,
        [
            enabled,
            max_streams_per_host,
            keepalive_time,
            connect_timeout
        ]
    );
    merge_group!(
        output,
        OutputConfig,
        [verbose, format, pretty_level, save_to_file]
    );
    merge_group!(ci, CiExitConfig, [strict_exit, fail_on, fail_on_percentage]);
    merge_group!(watch, WatchConfig, [enabled, debounce, clear]);
    merge_group!(
        snapshot,
        GlobalSnapshotConfig,
        [enabled, update_mode, dir, ci]
    );
    merge_group!(
        diff,
        GlobalDiffConfig,
        [enabled, save, label, compare_with, dir, output_format]
    );
    merge_group!(security, SecurityConfig, [allowed_protocols, allow_paths]);

    if let Some(o) = &e.defaults {
        let d = cfg.defaults.get_or_insert_with(RequestConfig::default);
        if o.timeout.is_some() {
            d.timeout = o.timeout;
        }
        if let Some(or) = &o.retry {
            let r = d.retry.get_or_insert_with(RetryConfig::default);
            if or.count.is_some() {
                r.count = or.count;
            }
            if or.delay.is_some() {
                r.delay = or.delay;
            }
        }
    }
}

/// Env lookup backed by the process environment, for `apply_env_overrides`.
pub fn process_env(name: &str) -> Option<String> {
    crate::interpolate::process_env(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write(dir: &Path, name: &str, content: &str) {
        fs::write(dir.join(name), content).unwrap();
    }

    fn map(pairs: &[(&str, &str)]) -> IndexMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    // ---- env-file-loader.test.ts ----

    #[test]
    fn empty_when_no_files() {
        let d = tempfile::tempdir().unwrap();
        let v = load_env_files(d.path(), None);
        assert!(v.is_empty());
        assert!(secret_keys(&v).is_empty());
    }

    #[test]
    fn loads_env_file() {
        let d = tempfile::tempdir().unwrap();
        write(
            d.path(),
            ".env",
            "API_URL=https://api.example.com\nAPI_KEY=abc123",
        );
        assert_eq!(
            load_env_files(d.path(), None),
            map(&[
                ("API_URL", "https://api.example.com"),
                ("API_KEY", "abc123")
            ])
        );
    }

    #[test]
    fn local_overrides_env() {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), ".env", "API_URL=https://prod.api.com\nFOO=bar");
        write(d.path(), ".env.local", "API_URL=https://local.api.com");
        let v = load_env_files(d.path(), None);
        assert_eq!(v["API_URL"], "https://local.api.com");
        assert_eq!(v["FOO"], "bar");
    }

    #[test]
    fn environment_specific_files() {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), ".env", "API_URL=https://default.api.com");
        write(d.path(), ".env.production", "API_URL=https://prod.api.com");
        assert_eq!(
            load_env_files(d.path(), Some("production"))["API_URL"],
            "https://prod.api.com"
        );
        assert_eq!(
            load_env_files(d.path(), None)["API_URL"],
            "https://default.api.com"
        );
    }

    #[test]
    fn env_local_over_env_specific() {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), ".env.staging", "API_URL=https://staging.api.com");
        write(
            d.path(),
            ".env.staging.local",
            "API_URL=https://my-staging.api.com",
        );
        assert_eq!(
            load_env_files(d.path(), Some("staging"))["API_URL"],
            "https://my-staging.api.com"
        );
    }

    #[test]
    fn full_priority_chain() {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), ".env", "A=1\nB=1\nC=1\nD=1");
        write(d.path(), ".env.local", "B=2\nC=2\nD=2");
        write(d.path(), ".env.test", "C=3\nD=3");
        write(d.path(), ".env.test.local", "D=4");
        assert_eq!(
            load_env_files(d.path(), Some("test")),
            map(&[("A", "1"), ("B", "2"), ("C", "3"), ("D", "4")])
        );
    }

    #[test]
    fn extracts_secret_keys() {
        let d = tempfile::tempdir().unwrap();
        write(
            d.path(),
            ".env",
            "API_URL=https://api.com\nSECRET_API_KEY=supersecret\nSECRET_TOKEN=mytoken\nPUBLIC_KEY=pub123",
        );
        let keys = secret_keys(&load_env_files(d.path(), None));
        assert_eq!(keys, vec!["SECRET_API_KEY", "SECRET_TOKEN"]);
    }

    #[test]
    fn quoted_values() {
        assert_eq!(
            parse_env_content(r#"MESSAGE="Hello World""#)["MESSAGE"],
            "Hello World"
        );
        assert_eq!(
            parse_env_content("MESSAGE='Hello World'")["MESSAGE"],
            "Hello World"
        );
        assert_eq!(parse_env_content("M=\"")["M"], "");
    }

    #[test]
    fn escape_sequences_only_in_double_quotes() {
        assert_eq!(
            parse_env_content(r#"M="line1\nline2\tindented""#)["M"],
            "line1\nline2\tindented"
        );
        assert_eq!(parse_env_content(r"M='a\nb'")["M"], r"a\nb");
        assert_eq!(parse_env_content(r"M=a\nb")["M"], r"a\nb");
    }

    #[test]
    fn comments_blank_lines_and_invalid_lines() {
        assert_eq!(
            parse_env_content("# c\nAPI_URL=https://api.com\n# d"),
            map(&[("API_URL", "https://api.com")])
        );
        assert_eq!(
            parse_env_content("\n\nAPI_URL=u\n\n\nAPI_KEY=key\n"),
            map(&[("API_URL", "u"), ("API_KEY", "key")])
        );
        assert_eq!(
            parse_env_content("VALID=value\nINVALID_LINE\nANOTHER=value2"),
            map(&[("VALID", "value"), ("ANOTHER", "value2")])
        );
        assert_eq!(
            parse_env_content("=novalue\n"),
            IndexMap::<String, String>::new()
        );
    }

    #[test]
    fn equals_empty_and_whitespace() {
        assert_eq!(
            parse_env_content("URL=https://api.com?key=value&foo=bar")["URL"],
            "https://api.com?key=value&foo=bar"
        );
        let v = parse_env_content("EMPTY=\nANOTHER=value");
        assert_eq!((v["EMPTY"].as_str(), v["ANOTHER"].as_str()), ("", "value"));
        assert_eq!(parse_env_content("  KEY  =  value  ")["KEY"], "value");
        assert_eq!(
            parse_env_content("K=v\r\nL=w\r\n"),
            map(&[("K", "v"), ("L", "w")])
        );
    }

    #[test]
    fn leading_bom_is_ignored() {
        assert_eq!(
            parse_env_content("\u{feff}A=1\nB=2"),
            map(&[("A", "1"), ("B", "2")])
        );
        assert_eq!(parse_env_content("\u{feff}# c\nA=1"), map(&[("A", "1")]));
    }

    #[test]
    fn env_file_paths_lists_existing() {
        let d = tempfile::tempdir().unwrap();
        assert!(env_file_paths(d.path(), None).is_empty());
        write(d.path(), ".env", "K=v");
        write(d.path(), ".env.local", "K=l");
        write(d.path(), ".env.production", "K=p");
        assert_eq!(
            env_file_paths(d.path(), None),
            vec![d.path().join(".env"), d.path().join(".env.local")]
        );
        assert_eq!(
            env_file_paths(d.path(), Some("production")),
            vec![
                d.path().join(".env"),
                d.path().join(".env.local"),
                d.path().join(".env.production")
            ]
        );
    }

    // ---- env-loader.test.ts ----

    fn load(pairs: &[(&str, &str)]) -> GlobalConfig {
        let m = map(pairs);
        load_environment_config(&|k: &str| m.get(k).cloned())
    }

    #[test]
    fn no_env_vars_gives_empty_config() {
        assert_eq!(load(&[]), GlobalConfig::default());
    }

    #[test]
    fn execution_vars() {
        assert_eq!(
            load(&[("CURL_RUNNER_EXECUTION", "parallel")])
                .execution
                .as_deref(),
            Some("parallel")
        );
        assert_eq!(
            load(&[("CURL_RUNNER_CONTINUE_ON_ERROR", "true")]).continue_on_error,
            Some(true)
        );
        assert_eq!(
            load(&[("CURL_RUNNER_CONTINUE_ON_ERROR", "TRUE")]).continue_on_error,
            Some(true)
        );
        assert_eq!(
            load(&[("CURL_RUNNER_CONTINUE_ON_ERROR", "no")]).continue_on_error,
            Some(false)
        );
        assert_eq!(load(&[("CURL_RUNNER_DRY_RUN", "true")]).dry_run, Some(true));
        assert_eq!(load(&[("CURL_RUNNER_HTTP2", "true")]).http2, Some(true));
        assert_eq!(
            load(&[("CURL_RUNNER_MAX_CONCURRENCY", "10")]).max_concurrency,
            Some(10)
        );
        assert_eq!(
            load(&[("CURL_RUNNER_MAX_CONCURRENCY", "0")]).max_concurrency,
            None
        );
        assert_eq!(
            load(&[("CURL_RUNNER_MAX_CONCURRENCY", "abc")]).max_concurrency,
            None
        );
    }

    #[test]
    fn connection_pool_vars() {
        let p = |k, v| load(&[(k, v)]).connection_pool.unwrap();
        assert_eq!(p("CURL_RUNNER_CONNECTION_POOL", "true").enabled, Some(true));
        assert_eq!(
            p("CURL_RUNNER_MAX_STREAMS_PER_HOST", "20").max_streams_per_host,
            Some(20)
        );
        assert_eq!(
            p("CURL_RUNNER_KEEPALIVE_TIME", "120").keepalive_time,
            Some(120)
        );
        assert_eq!(
            p("CURL_RUNNER_CONNECT_TIMEOUT", "60").connect_timeout,
            Some(60)
        );
    }

    #[test]
    fn output_vars() {
        let o = |k, v| load(&[(k, v)]).output;
        assert_eq!(
            o("CURL_RUNNER_VERBOSE", "true").unwrap().verbose,
            Some(true)
        );
        assert_eq!(
            o("CURL_RUNNER_OUTPUT_FORMAT", "json")
                .unwrap()
                .format
                .as_deref(),
            Some("json")
        );
        assert_eq!(
            o("CURL_RUNNER_OUTPUT_FORMAT", "pretty")
                .unwrap()
                .format
                .as_deref(),
            Some("pretty")
        );
        assert_eq!(o("CURL_RUNNER_OUTPUT_FORMAT", "invalid"), None);
        assert_eq!(
            o("CURL_RUNNER_PRETTY_LEVEL", "minimal")
                .unwrap()
                .pretty_level
                .as_deref(),
            Some("minimal")
        );
        assert_eq!(
            o("CURL_RUNNER_PRETTY_LEVEL", "detailed")
                .unwrap()
                .pretty_level
                .as_deref(),
            Some("detailed")
        );
        assert_eq!(
            o("CURL_RUNNER_OUTPUT_FILE", "results.json")
                .unwrap()
                .save_to_file
                .as_deref(),
            Some("results.json")
        );
    }

    #[test]
    fn ci_vars() {
        let c = |k, v| load(&[(k, v)]).ci;
        assert_eq!(
            c("CURL_RUNNER_STRICT_EXIT", "true").unwrap().strict_exit,
            Some(true)
        );
        assert_eq!(c("CURL_RUNNER_FAIL_ON", "3").unwrap().fail_on, Some(3));
        assert_eq!(
            c("CURL_RUNNER_FAIL_ON_PERCENTAGE", "25.5")
                .unwrap()
                .fail_on_percentage,
            Some(25.5)
        );
        assert_eq!(c("CURL_RUNNER_FAIL_ON_PERCENTAGE", "150"), None);
        assert_eq!(c("CURL_RUNNER_FAIL_ON_PERCENTAGE", "-10"), None);
    }

    #[test]
    fn watch_vars() {
        let w = |k, v| load(&[(k, v)]).watch.unwrap();
        assert_eq!(w("CURL_RUNNER_WATCH", "true").enabled, Some(true));
        assert_eq!(w("CURL_RUNNER_WATCH_DEBOUNCE", "500").debounce, Some(500));
        assert_eq!(w("CURL_RUNNER_WATCH_CLEAR", "true").clear, Some(true));
        assert_eq!(w("CURL_RUNNER_WATCH_CLEAR", "false").clear, Some(false));
    }

    #[test]
    fn profile_vars() {
        let p = |k, v| load(&[(k, v)]).profile;
        assert_eq!(p("CURL_RUNNER_PROFILE", "100").unwrap().iterations, 100);
        assert_eq!(p("CURL_RUNNER_PROFILE", "0"), None);
        let w = p("CURL_RUNNER_PROFILE_WARMUP", "5").unwrap();
        assert_eq!((w.warmup, w.iterations), (Some(5), 10));
        assert_eq!(
            p("CURL_RUNNER_PROFILE_CONCURRENCY", "4")
                .unwrap()
                .concurrency,
            Some(4)
        );
        assert_eq!(
            p("CURL_RUNNER_PROFILE_HISTOGRAM", "true")
                .unwrap()
                .histogram,
            Some(true)
        );
        assert_eq!(
            p("CURL_RUNNER_PROFILE_EXPORT", "perf.json")
                .unwrap()
                .export_file
                .as_deref(),
            Some("perf.json")
        );
        let both = load(&[
            ("CURL_RUNNER_PROFILE", "7"),
            ("CURL_RUNNER_PROFILE_WARMUP", "2"),
        ])
        .profile
        .unwrap();
        assert_eq!((both.iterations, both.warmup), (7, Some(2)));
    }

    #[test]
    fn snapshot_vars() {
        let s = |k, v| load(&[(k, v)]).snapshot;
        assert_eq!(
            s("CURL_RUNNER_SNAPSHOT", "true").unwrap().enabled,
            Some(true)
        );
        for m in ["all", "failing", "none"] {
            assert_eq!(
                s("CURL_RUNNER_SNAPSHOT_UPDATE", m)
                    .unwrap()
                    .update_mode
                    .as_deref(),
                Some(m)
            );
        }
        assert_eq!(
            s("CURL_RUNNER_SNAPSHOT_UPDATE", "ALL")
                .unwrap()
                .update_mode
                .as_deref(),
            Some("all")
        );
        assert_eq!(s("CURL_RUNNER_SNAPSHOT_UPDATE", "invalid"), None);
        assert_eq!(
            s("CURL_RUNNER_SNAPSHOT_DIR", ".snapshots")
                .unwrap()
                .dir
                .as_deref(),
            Some(".snapshots")
        );
        assert_eq!(s("CURL_RUNNER_SNAPSHOT_CI", "true").unwrap().ci, Some(true));
    }

    #[test]
    fn diff_vars() {
        let d = |k, v| load(&[(k, v)]).diff;
        assert_eq!(d("CURL_RUNNER_DIFF", "true").unwrap().enabled, Some(true));
        assert_eq!(d("CURL_RUNNER_DIFF_SAVE", "true").unwrap().save, Some(true));
        assert_eq!(
            d("CURL_RUNNER_DIFF_LABEL", "production")
                .unwrap()
                .label
                .as_deref(),
            Some("production")
        );
        assert_eq!(
            d("CURL_RUNNER_DIFF_COMPARE", "baseline")
                .unwrap()
                .compare_with
                .as_deref(),
            Some("baseline")
        );
        assert_eq!(
            d("CURL_RUNNER_DIFF_DIR", ".baselines")
                .unwrap()
                .dir
                .as_deref(),
            Some(".baselines")
        );
        for f in ["terminal", "json", "markdown"] {
            assert_eq!(
                d("CURL_RUNNER_DIFF_OUTPUT", f)
                    .unwrap()
                    .output_format
                    .as_deref(),
                Some(f)
            );
        }
        assert_eq!(d("CURL_RUNNER_DIFF_OUTPUT", "invalid"), None);
    }

    #[test]
    fn timeout_and_retry_vars() {
        let d = |k, v| load(&[(k, v)]).defaults.unwrap();
        assert_eq!(d("CURL_RUNNER_TIMEOUT", "30").timeout, Some(30));
        assert_eq!(d("CURL_RUNNER_RETRIES", "5").retry.unwrap().count, Some(5));
        assert_eq!(
            d("CURL_RUNNER_RETRY_DELAY", "2000").retry.unwrap().delay,
            Some(2000)
        );
    }

    #[test]
    fn security_vars() {
        let s = load(&[
            ("CURL_RUNNER_ALLOWED_PROTOCOLS", " HTTPS, http ,,"),
            ("CURL_RUNNER_ALLOW_PATHS", "true"),
        ])
        .security
        .unwrap();
        assert_eq!(
            s.allowed_protocols,
            Some(vec!["https".to_string(), "http".to_string()])
        );
        assert_eq!(s.allow_paths, Some(true));
        assert_eq!(
            load(&[("CURL_RUNNER_ALLOWED_PROTOCOLS", " , ")]).security,
            None
        );
    }

    #[test]
    fn multiple_vars_together() {
        let c = load(&[
            ("CURL_RUNNER_EXECUTION", "parallel"),
            ("CURL_RUNNER_MAX_CONCURRENCY", "5"),
            ("CURL_RUNNER_HTTP2", "true"),
            ("CURL_RUNNER_VERBOSE", "true"),
            ("CURL_RUNNER_TIMEOUT", "30"),
        ]);
        assert_eq!(c.execution.as_deref(), Some("parallel"));
        assert_eq!(c.max_concurrency, Some(5));
        assert_eq!(c.http2, Some(true));
        assert_eq!(c.output.unwrap().verbose, Some(true));
        assert_eq!(c.defaults.unwrap().timeout, Some(30));
    }

    #[test]
    fn js_style_number_parsing() {
        assert_eq!(parse_int("30ms"), Some(30));
        assert_eq!(parse_int("  -4"), Some(-4));
        assert_eq!(parse_int("x"), None);
        assert_eq!(parse_float("25.5%"), Some(25.5));
        assert_eq!(parse_float(".5"), Some(0.5));
        assert_eq!(parse_float("1e2x"), Some(100.0));
        assert_eq!(parse_float("."), None);
        assert_eq!(parse_float("abc"), None);
    }

    // ---- apply_env_overrides ----

    #[test]
    fn apply_overrides_merges_like_ts() {
        let mut cfg = GlobalConfig {
            execution: Some("sequential".into()),
            max_concurrency: Some(2),
            output: Some(OutputConfig {
                verbose: Some(false),
                format: Some("pretty".into()),
                ..Default::default()
            }),
            profile: Some(ProfileConfig {
                iterations: 3,
                warmup: Some(1),
                ..Default::default()
            }),
            defaults: Some(RequestConfig {
                timeout: Some(1000),
                retry: Some(RetryConfig {
                    count: Some(1),
                    delay: Some(10),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        let m = map(&[
            ("CURL_RUNNER_EXECUTION", "parallel"),
            ("CURL_RUNNER_VERBOSE", "true"),
            ("CURL_RUNNER_PROFILE", "9"),
            ("CURL_RUNNER_RETRIES", "4"),
            ("CURL_RUNNER_DRY_RUN", "true"),
        ]);
        apply_env_overrides(&mut cfg, &|k: &str| m.get(k).cloned());
        assert_eq!(cfg.execution.as_deref(), Some("parallel"));
        assert_eq!(cfg.max_concurrency, Some(2));
        assert_eq!(cfg.dry_run, Some(true));
        let o = cfg.output.unwrap();
        assert_eq!(
            (o.verbose, o.format.as_deref()),
            (Some(true), Some("pretty"))
        );
        // profile is replaced wholesale (warmup from the file is gone)
        let p = cfg.profile.unwrap();
        assert_eq!((p.iterations, p.warmup), (9, None));
        let d = cfg.defaults.unwrap();
        assert_eq!(d.timeout, Some(1000));
        let r = d.retry.unwrap();
        assert_eq!((r.count, r.delay), (Some(4), Some(10)));
    }

    #[test]
    fn apply_overrides_noop_without_env() {
        let mut cfg = GlobalConfig {
            http2: Some(true),
            ..Default::default()
        };
        let before = cfg.clone();
        apply_env_overrides(&mut cfg, &|_: &str| None);
        assert_eq!(cfg, before);
    }
}
