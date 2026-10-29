// Some parsed options are consumed only by later ports (watch, profile, ...).
#[allow(dead_code)]
mod args;
mod colors;
mod convert_cmd;
mod diff_mode;
mod exit;
mod install;
mod net;
mod output;
mod profile;
mod run;
mod stats;
mod upgrade;
mod validate_cmd;
mod version_check;
mod watch;
mod wizard;

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let code = run::main(argv);
    std::process::exit(code);
}
