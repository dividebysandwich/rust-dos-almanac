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
- DOS programs and Windows 3.1/95/98 programs alike: Rust-DOS runs Windows
  programs on its built-in Windows. Each version has a platform (`dos`,
  `win16`, `win32`, or none if unknown), and a Windows version is
  recommended `win32=true` unless reports or a blessing set `win32` themselves.

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
| `GET /schema` | all | Known settings, allowed values and form groups; `version` bumps when they change (2 added `win32_depth`, 3 `fps_cap`, 4 `win32_desktop`, `win32_locale`, `adlibgold`, `svga_toadforce` and `fps_cap=auto`, 5 `opl=auto`, `opl=dualopl2`, `sbtype=sbpro1` and `sbtype=gb`) |
| `GET /games?q=&platform=all\|dos\|windows&page=` | all | Search name, aliases and slug; `platform` keeps games with a version of that platform |
| `POST /games/lookup` | all | `{"ra_md5": "…", "files": [{"name", "size", "crc32", "md5", "sha1"}]}` → matching versions with their `platform`, best first |
| `GET /games/{slug}` | all | Game, versions, files, rating counts |
| `GET /games/{slug}/config?version=&format=json\|ini` | all | Recommended settings; `ini` is a fragment for `rust-dos.conf`. A setting with `"implied": true` comes from the platform, not reports. JSON also has `notes`: the blessed report's notes, or those of the reports the consensus came from (red ones too), newest first, at most 10 |
| `GET /games/{slug}/reports` | all | |
| `POST /games`, `POST /games/{slug}/versions`, `POST /reports` | all, rate limited | |
| `PATCH /games/{slug}`, `PATCH /versions/{id}` | staff, or the adder's `key` | Edit; staff may also `{"hidden": true}` |
| `PATCH /reports/{id}` | staff | Edit, or `{"hidden": true}` |
| `POST /reports/{id}/bless`, `DELETE /games/{slug}/blessing?version=` | staff | |
| `GET /moderation/reports`, `POST /moderation/hide-source`, `GET /moderation/log` | staff | |
| `POST /auth/login` → `{"token"}`, then `Authorization: Bearer <token>` | | |
| `GET/POST /users`, `PATCH /users/{id}` | admin | |

`POST /reports` takes `game`, `version_id`, `rating`, `settings`,
`rust_dos_version`, and optionally:

- `notes`: the player's own words, shown with the report (2000 characters).
- `key`: a secret of the sender's, 16 to 64 letters, digits and dashes, such
  as the UUID of the Rust-DOS game profile it comes from. A report with a key
  replaces the one sent with the same key on the same game before, and the
  answer says `"replaced": true`. A key keeps a report on each game, as one
  profile (a Windows 98 image, say) can run several. A report staff blessed is left as it
  is, and the key moves to a new report; a hidden one stays hidden. Only the
  key's SHA-256 is stored, and it is never shown.
- `year`, `publisher`: fill in what the game lacks. A game added with
  `POST /games` and a `key` takes its adder's corrections from reports with
  the same key, until staff edit it.

`POST /games` and `POST /games/{slug}/versions` take a `key` too (a
`version` in `POST /games` takes the game's unless it has its own). With
it, the adder corrects what they added, in the API or on the game's page:
`PATCH /games/{slug}` with `key` and `name`, `year`, `publisher` or
`aliases`, and `PATCH /versions/{id}` with `key` and `label`, `platform`,
`files` (each with a hash) or `ra_md5`. Such an edit spends from the write
budget and shows in the moderation log as `owner-edit`. Once staff edit the
game or version, the key no longer does; a wrong key gets 403.

`POST /games` for a name that is taken answers 409 with the existing
game's slug as `game`, to report on it or add a version to it instead.

The report and new game forms take a pasted game profile and fill
themselves in from it, in the browser (`model::parse_config`).

Report settings are `{"emulator": {"cpu": "486", "memsize": "16"}, "sound": {"sbtype": "sb16"}}`.
They are validated against the values Rust-DOS accepts (`src/settings_schema.rs`)
and normalised (`586` → `pentium`, `0x220` → `220`, `on` → `true`).
`[drives]`, `[autoexec]` and file paths are deliberately not accepted.

Versions take `"platform": "dos" | "win16" | "win32"` in `POST /games`
(`version`), `POST /games/{slug}/versions` and `PATCH /versions/{id}`
(`null` clears it). Windows settings are `emulator.win32`,
`win32_version` (`95`, `98`, `98se`), `win32_depth` (`auto`, `8`, `16`),
`win32_desktop` (`640x480`, `800x600`, `1024x768`) and `win32_locale`
(the built-in Windows' system locales, such as `en-US` or `ru-RU`).
Rust-DOS's `win32_dlls*` and `win32_trace`, and a profile's `[game] windows=`
and `[install]`, are host or launch matters and are not accepted.
So is Rust-DOS's `renderer` (`auto`, `opengl`, `software`: what draws the
3dfx cards and the ToadForce on the host), and the `voodoo_renderer` and
`toadforce_renderer` it replaces; a pasted profile lists any of them as
ignored, whichever form it uses.

`emulator.fps_cap` is Rust-DOS's frame rate cap for whichever card shows the
picture: `auto` (Rust-DOS's default: 60 for a program that doesn't wait for
the display's retrace, none for one that does), `off` (`0` is taken as
`off`) or 10 to 240, as Rust-DOS's parser accepts it. Benchmarks and
timedemos report at most the cap, so `off` is a recommendation of its own. The old
name `voodoo_fps_cap` is accepted in reports and pasted profiles and stored
as `fps_cap`; given under both names, `fps_cap` wins. Recommendations only
ever carry `fps_cap`.

Rust-DOS sends every one of its almanac keys (`KEYS` in
`rust-dos/src/almanac.rs`) with each report, and the almanac rejects a
report with a key or value it doesn't know. A key or value Rust-DOS adds
must be added to `src/settings_schema.rs` before a release that sends it;
`a_current_rust_dos_report_is_accepted` there holds a report as it sends it.
`RUST_DOS_TAKES` in its tests lists, for each key, the values Rust-DOS's
configuration parser takes (aliases too); the tests check that the almanac
takes every one of them and offers none Rust-DOS doesn't. Update it together
with the schema whenever Rust-DOS's parser or `KEYS` change.

## Tests

```sh
cargo test --features ssr
```

## Credits

IBM VGA 8x16 font by VileR, [int10h.org](https://int10h.org/oldschool-pc-fonts/),
CC BY-SA 4.0 (`public/fonts/LICENSE-font.txt`). Look and feel after rust-dos.com.
