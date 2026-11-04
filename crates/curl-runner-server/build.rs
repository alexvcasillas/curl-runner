// `sqlx::migrate!()` embeds ./migrations; rebuild when a migration is added.
fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
