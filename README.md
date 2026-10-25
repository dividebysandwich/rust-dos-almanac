# Rust-DOS Almanac

Game compatibility reports and recommended settings for the
[Rust-DOS](https://rust-dos.com/) emulator: a web UI styled like
rust-dos.com's Turbo Vision desktop, and a JSON API Rust-DOS can query.

- Anyone can add games, versions (identified by file names + CRC32/MD5/SHA-1,
  or Rust-DOS's RetroAchievements archive MD5) and reports: a stoplight rating
  (green, yellow or red) plus the Rust-DOS settings they used.
- Each setting is recommended by **mode**, not average. Green reports count 1,
  yellow 0.5 and red 0. Ties go to the newest green report, then to the value
  nearest the median.
- Moderators and admins can edit or hide anything, write experience
  descriptions, and **bless** one report per game + version, which then
  overrides the consensus.
- Anonymous writes are rate limited per client address.

Stack: Leptos 0.8 (SSR + hydration), Axum 0.8, SeaORM 2 on SQLite (MariaDB
works via `DATABASE_URL=mysql://…`).

## Running

```sh
cargo install cargo-leptos
rustup target add wasm32-unknown-unknown

# Create the first admin (the password is prompted for, or read from stdin).
cargo run --features ssr -- create-user admin --role admin

cargo leptos watch        # development, http://127.0.0.1:3000
cargo leptos build --release
```

The release build is `target/release/rust-dos-almanac` and the static site is
`target/site`. In production, set:

| Variable | Meaning |
|---|---|
| `DATABASE_URL` | Default `sqlite://almanac.db?mode=rwc` |
| `LEPTOS_SITE_ADDR` | Address to listen on, e.g. `127.0.0.1:3000` |
| `LEPTOS_SITE_ROOT` | Path to the `site` directory |
| `LEPTOS_OUTPUT_NAME` | `rust-dos-almanac` |
| `ALMANAC_SECRET` | Key for hashing client addresses. If unset, one is generated and stored in the DB. |
| `ALMANAC_TRUSTED_PROXIES` | CIDRs allowed to set `CF-Connecting-IP`. Default: Cloudflare's ranges. Add `127.0.0.1/32` when `cloudflared` runs on the same host. |
| `ALMANAC_SECURE_COOKIES` | `true` (default); `false` only for plain-HTTP development |
| `ALMANAC_WRITE_BURST`, `ALMANAC_WRITE_REFILL_SECS`, `ALMANAC_WRITES_PER_DAY`, `ALMANAC_READS_PER_MINUTE`, `ALMANAC_LOGIN_BURST` | Rate limits (5, 360 s, 40, 120, 5) |

`CF-Connecting-IP` is only believed when the TCP peer is a trusted proxy.
Otherwise, anyone reaching the server directly could fake an address and dodge
the limits. Client addresses are never stored, only an HMAC pseudonym, which
lets moderators hide everything from one source.

Other commands: `migrate`, `set-password <user>`.

## API (`/api/v1`)

| Method & path | Who | |
|---|---|---|
| `GET /schema` | all | Known settings and allowed values |
| `GET /games?q=&page=` | all | Search name, aliases and slug |
| `POST /games/lookup` | all | `{"ra_md5": "…", "files": [{"name", "size", "crc32", "md5", "sha1"}]}` → matching versions, best first |
| `GET /games/{slug}` | all | Game, versions, files, rating counts |
| `GET /games/{slug}/config?version=&format=json\|ini` | all | Recommended settings; `ini` is a fragment for `rust-dos.conf` |
| `GET /games/{slug}/reports` | all | |
| `POST /games`, `POST /games/{slug}/versions`, `POST /reports` | all, rate limited | |
| `PATCH /games/{slug}`, `PATCH /versions/{id}`, `PATCH /reports/{id}` | staff | Edit, or `{"hidden": true}` |
| `POST /reports/{id}/bless`, `DELETE /games/{slug}/blessing?version=` | staff | |
| `GET /moderation/reports`, `POST /moderation/hide-source`, `GET /moderation/log` | staff | |
| `POST /auth/login` → `{"token"}`, then `Authorization: Bearer <token>` | | |
| `GET/POST /users`, `PATCH /users/{id}` | admin | |

Report settings are `{"emulator": {"cpu": "486", "memsize": "16"}, "sound": {"sbtype": "sb16"}}`.
They are validated against the values Rust-DOS accepts (`src/settings_schema.rs`)
and normalised (`586` → `pentium`, `0x220` → `220`, `on` → `true`).
`[drives]`, `[autoexec]` and file paths are deliberately not accepted.

## Tests

```sh
cargo test --features ssr
```

## Credits

IBM VGA 8x16 font by VileR, [int10h.org](https://int10h.org/oldschool-pc-fonts/),
CC BY-SA 4.0 (`public/fonts/LICENSE-font.txt`). Look and feel after rust-dos.com.
