//! Server settings: one row in `server_settings`, read through a short cache.
//!
//! `SIGNUP`, `SIGNUP_DOMAINS` and `DEFAULT_PLAN` only seed the row on the
//! first start. After that the row wins. Every reader of a setting goes
//! through [`AppState::settings`], never through [`Config`].

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Duration, Utc};
use curl_runner_cloud_proto::{Plan, SignupMode};
use serde::Serialize;
use serde_json::{Value, json};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::auth::session::column_enum;
use crate::clock::Clock;
use crate::config::Config;
use crate::error::ApiError;
use crate::teams::enum_str;

/// How long a read stays fresh. A write in this process drops it at once.
pub const TTL: Duration = Duration::seconds(5);

/// The `server_settings` row.
#[derive(Debug, Clone, Serialize)]
pub struct Settings {
    pub signup_mode: SignupMode,
    pub signup_domains: Vec<String>,
    pub default_plan: Plan,
    pub free_member_limit: u32,
    /// Whether restricted collections are allowed on the Free plan.
    pub free_restricted: bool,
    pub grace_days: u32,
    pub banner_message: Option<String>,
    pub banner_tone: String,
    pub banner_audience: String,
    pub banner_link: Option<String>,
    pub banner_version: u32,
    pub publishing_paused: bool,
    pub env_seed: Option<Value>,
    pub updated_at: Option<DateTime<Utc>>,
    pub updated_by: Option<Uuid>,
    pub updated_by_email: Option<String>,
}

#[derive(FromRow)]
struct Row {
    signup_mode: String,
    signup_domains: Vec<String>,
    default_plan: String,
    free_member_limit: i32,
    free_restricted: bool,
    grace_days: i32,
    banner_message: Option<String>,
    banner_tone: String,
    banner_audience: String,
    banner_link: Option<String>,
    banner_version: i32,
    publishing_paused: bool,
    env_seed: Option<Value>,
    updated_at: Option<DateTime<Utc>>,
    updated_by: Option<Uuid>,
    updated_by_email: Option<String>,
}

const SELECT: &str = "SELECT s.signup_mode, s.signup_domains, s.default_plan, \
     s.free_member_limit, s.free_restricted, s.grace_days, s.banner_message, s.banner_tone, \
     s.banner_audience, s.banner_link, s.banner_version, s.publishing_paused, s.env_seed, \
     s.updated_at, s.updated_by, u.email::text AS updated_by_email \
     FROM server_settings s LEFT JOIN users u ON u.id = s.updated_by";

/// The column defaults of the migration.
impl Default for Settings {
    fn default() -> Self {
        Self {
            signup_mode: SignupMode::Domains,
            signup_domains: Vec::new(),
            default_plan: Plan::Team,
            free_member_limit: 3,
            free_restricted: false,
            grace_days: 14,
            banner_message: None,
            banner_tone: "info".into(),
            banner_audience: "everyone".into(),
            banner_link: None,
            banner_version: 0,
            publishing_paused: false,
            env_seed: None,
            updated_at: None,
            updated_by: None,
            updated_by_email: None,
        }
    }
}

impl Settings {
    fn from_row(row: Row) -> Result<Self, ApiError> {
        Ok(Self {
            signup_mode: column_enum(row.signup_mode)?,
            signup_domains: row.signup_domains,
            default_plan: column_enum(row.default_plan)?,
            free_member_limit: u32::try_from(row.free_member_limit).unwrap_or(3),
            free_restricted: row.free_restricted,
            grace_days: u32::try_from(row.grace_days).unwrap_or(14),
            banner_message: row.banner_message,
            banner_tone: row.banner_tone,
            banner_audience: row.banner_audience,
            banner_link: row.banner_link,
            banner_version: u32::try_from(row.banner_version).unwrap_or(0),
            publishing_paused: row.publishing_paused,
            env_seed: row.env_seed,
            updated_at: row.updated_at,
            updated_by: row.updated_by,
            updated_by_email: row.updated_by_email,
        })
    }

    /// The post-deploy grace period for a failed payment.
    pub fn grace(&self) -> Duration {
        Duration::days(i64::from(self.grace_days))
    }
}

/// Read the row, or `None` before the first start has seeded it.
pub async fn fetch(db: &PgPool) -> Result<Option<Settings>, ApiError> {
    let row: Option<Row> = sqlx::query_as(SELECT).fetch_optional(db).await?;
    row.map(Settings::from_row).transpose()
}

/// What [`seed`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedOutcome {
    /// This call created the row.
    pub inserted: bool,
    /// Env vars whose value now differs from what seeded the row. They are
    /// ignored: the row wins.
    pub drifted: Vec<&'static str>,
}

fn env_values(config: &Config) -> Value {
    json!({
        "signup": config.signup,
        "signup_domains": config.signup_domains,
        "default_plan": config.default_plan,
    })
}

/// Create the row from the env on the first start, then report any of the
/// three env vars that changed since. Safe to call on every start and from
/// concurrent processes.
pub async fn seed(
    db: &PgPool,
    config: &Config,
    now: DateTime<Utc>,
) -> Result<SeedOutcome, ApiError> {
    let env = env_values(config);
    let inserted = sqlx::query(
        "INSERT INTO server_settings (signup_mode, signup_domains, default_plan, env_seed, updated_at) \
         VALUES ($1, $2, $3, $4, $5) ON CONFLICT (singleton) DO NOTHING",
    )
    .bind(enum_str(config.signup))
    .bind(&config.signup_domains)
    .bind(enum_str(config.default_plan))
    .bind(&env)
    .bind(now)
    .execute(db)
    .await?
    .rows_affected()
        == 1;
    let seeded: Option<Value> = sqlx::query_scalar("SELECT env_seed FROM server_settings")
        .fetch_one(db)
        .await?;
    let mut drifted = Vec::new();
    if !inserted && let Some(seeded) = seeded {
        for (field, var) in [
            ("signup", "SIGNUP"),
            ("signup_domains", "SIGNUP_DOMAINS"),
            ("default_plan", "DEFAULT_PLAN"),
        ] {
            if seeded.get(field) != env.get(field) {
                drifted.push(var);
            }
        }
    }
    Ok(SeedOutcome { inserted, drifted })
}

/// When the row was read, and what it held.
type Cached = (DateTime<Utc>, Arc<Settings>);

/// The cached reader. Cloning shares the cache.
#[derive(Clone)]
pub struct SettingsCache {
    clock: Arc<dyn Clock>,
    cached: Arc<Mutex<Option<Cached>>>,
    /// Bumped by [`SettingsCache::invalidate`], so a read that began before a
    /// write cannot put its stale row back afterwards.
    generation: Arc<AtomicU64>,
}

impl SettingsCache {
    pub fn new(clock: Arc<dyn Clock>) -> Self {
        Self {
            clock,
            cached: Arc::default(),
            generation: Arc::default(),
        }
    }

    /// The cached settings if they are under [`TTL`] old.
    fn fresh(&self) -> Option<Arc<Settings>> {
        let guard = self.cached.lock().unwrap_or_else(|e| e.into_inner());
        let (at, settings) = guard.as_ref()?;
        let age = self.clock.now() - *at;
        (age >= Duration::zero() && age < TTL).then(|| settings.clone())
    }

    /// Settings now: from the cache, else from the database (seeding the row
    /// from `config` if this is the first start).
    pub async fn get(&self, db: &PgPool, config: &Config) -> Result<Arc<Settings>, ApiError> {
        if let Some(settings) = self.fresh() {
            return Ok(settings);
        }
        let generation = self.generation.load(Ordering::SeqCst);
        let now = self.clock.now();
        let settings = match fetch(db).await? {
            Some(settings) => settings,
            None => {
                seed(db, config, now).await?;
                fetch(db).await?.ok_or_else(ApiError::unavailable)?
            }
        };
        let settings = Arc::new(settings);
        let mut cached = self.cached.lock().unwrap_or_else(|e| e.into_inner());
        if self.generation.load(Ordering::SeqCst) == generation {
            *cached = Some((now, settings.clone()));
        }
        Ok(settings)
    }

    /// Drop the cache. A write calls this after it commits.
    pub fn invalidate(&self) {
        let mut cached = self.cached.lock().unwrap_or_else(|e| e.into_inner());
        self.generation.fetch_add(1, Ordering::SeqCst);
        *cached = None;
    }
}
