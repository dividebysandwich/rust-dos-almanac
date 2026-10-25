use clap::{Args, Parser, Subcommand};
use ipnet::IpNet;

/// Cloudflare's published edge ranges (https://www.cloudflare.com/ips/).
/// Only requests from these may set the client address with CF-Connecting-IP.
pub const CLOUDFLARE_RANGES: &[&str] = &[
    "173.245.48.0/20",
    "103.21.244.0/22",
    "103.22.200.0/22",
    "103.31.4.0/22",
    "141.101.64.0/18",
    "108.162.192.0/18",
    "190.93.240.0/20",
    "188.114.96.0/20",
    "197.234.240.0/22",
    "198.41.128.0/17",
    "162.158.0.0/15",
    "104.16.0.0/13",
    "104.24.0.0/14",
    "172.64.0.0/13",
    "131.0.72.0/22",
    "2400:cb00::/32",
    "2606:4700::/32",
    "2803:f800::/32",
    "2405:b500::/32",
    "2405:8100::/32",
    "2a06:98c0::/29",
    "2c0f:f248::/32",
];

#[derive(Parser, Debug)]
#[command(name = "rust-dos-almanac", version, about = "Game compatibility database for Rust-DOS")]
pub struct Cli {
    #[command(flatten)]
    pub config: Config,
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Run the web server (the default).
    Serve,
    /// Apply database migrations and exit.
    Migrate,
    /// Create a staff account. The password is read from the terminal, or from stdin when piped.
    CreateUser {
        username: String,
        #[arg(long, default_value = "moderator", value_parser = ["admin", "moderator"])]
        role: String,
    },
    /// Set a new password for a staff account.
    SetPassword { username: String },
}

#[derive(Args, Debug, Clone)]
pub struct Config {
    /// SeaORM database URL (sqlite://almanac.db?mode=rwc or mysql://user:pass@host/db).
    #[arg(long, env = "DATABASE_URL", default_value = "sqlite://almanac.db?mode=rwc", global = true)]
    pub database_url: String,

    /// Secret for hashing client addresses. Generated and stored in the database when unset.
    #[arg(long, env = "ALMANAC_SECRET", hide_env_values = true, global = true)]
    pub secret: Option<String>,

    /// Comma-separated CIDRs allowed to set CF-Connecting-IP. Defaults to Cloudflare's ranges;
    /// add 127.0.0.1/32 when cloudflared runs on the same host.
    #[arg(long, env = "ALMANAC_TRUSTED_PROXIES", value_delimiter = ',', global = true)]
    pub trusted_proxies: Vec<IpNet>,

    /// Mark the session cookie Secure. Turn off only for plain-HTTP development.
    #[arg(long, env = "ALMANAC_SECURE_COOKIES", default_value_t = true, action = clap::ArgAction::Set, global = true)]
    pub secure_cookies: bool,

    #[command(flatten)]
    pub limits: Limits,
}

#[derive(Args, Debug, Clone)]
pub struct Limits {
    /// Anonymous writes (games, versions, reports) one address may make in a burst.
    #[arg(long, env = "ALMANAC_WRITE_BURST", default_value_t = 5, global = true)]
    pub write_burst: u32,
    /// Seconds until one more anonymous write is allowed after the burst.
    #[arg(long, env = "ALMANAC_WRITE_REFILL_SECS", default_value_t = 360, global = true)]
    pub write_refill_secs: u64,
    /// Anonymous writes one address may make per day.
    #[arg(long, env = "ALMANAC_WRITES_PER_DAY", default_value_t = 40, global = true)]
    pub writes_per_day: u32,
    /// API reads one address may make per minute.
    #[arg(long, env = "ALMANAC_READS_PER_MINUTE", default_value_t = 120, global = true)]
    pub reads_per_minute: u32,
    /// Login attempts one address may make in a burst; then one per minute.
    #[arg(long, env = "ALMANAC_LOGIN_BURST", default_value_t = 5, global = true)]
    pub login_burst: u32,
}

impl Config {
    pub fn trusted_proxies(&self) -> Vec<IpNet> {
        if self.trusted_proxies.is_empty() {
            CLOUDFLARE_RANGES.iter().map(|r| r.parse().expect("valid built-in CIDR")).collect()
        } else {
            self.trusted_proxies.clone()
        }
    }

    /// Defaults with an in-memory database, for tests.
    pub fn for_tests() -> Self {
        Cli::parse_from(["rust-dos-almanac", "--database-url", "sqlite::memory:", "--secure-cookies", "false"]).config
    }
}
