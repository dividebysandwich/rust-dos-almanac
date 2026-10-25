pub mod forms;
pub mod game;
pub mod home;
pub mod report;
pub mod staff;

use crate::app::components::{Dialog, Light, Window};
use crate::model::Rating;
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::components::A;

#[component]
pub fn NotFound() -> impl IntoView {
    #[cfg(feature = "ssr")]
    if let Some(res) = use_context::<leptos_axum::ResponseOptions>() {
        res.set_status(axum::http::StatusCode::NOT_FOUND);
    }
    view! {
        <Title text="Not found" />
        <Dialog title="Error" class="narrow">
            <p class="center">"Path not found."</p>
            <p class="center">
                <A href="/" attr:class="btn is-default">
                    "OK"
                </A>
            </p>
        </Dialog>
    }
}

#[component]
pub fn AboutPage() -> impl IntoView {
    view! {
        <Title text="About" />
        <div class="row-top">
            <Window title="About" class="prose">
                <p>
                    <b>"Rust-DOS Almanac"</b>
                    " collects how DOS games run in "
                    <a href="https://rust-dos.com/">"Rust-DOS"</a>
                    " and which settings work best for each."
                </p>
                <p>"Anybody can add a game, a version of it, or a report of how it ran. Moderators keep the entries tidy and can bless one report per game and version as the recommended configuration."</p>
                <h3>"The stoplight"</h3>
                <ul class="legend">
                    {Rating::ALL.map(|r| view! { <li><Light rating=Some(r) label=true /></li> }).collect_view()}
                </ul>
                <h3>"Recommended settings"</h3>
                <p>"A blessed report wins. Otherwise each setting takes the value most reports chose. Rust-DOS offers fixed choices, so the almanac never averages 128 and 256 MB into something Rust-DOS doesn't have. Green reports count fully and yellow ones half. Red reports don't count, since they describe what didn't work."</p>
                <h3>"Identifying a game"</h3>
                <p>"A game has versions. A version is known by its files' names and checksums (CRC32, MD5 or SHA-1), and by the archive hash Rust-DOS computes for RetroAchievements."</p>
            </Window>
            <Window title="API" class="prose">
                <p>"Rust-DOS and other clients use the JSON API:"</p>
                <pre class="code">
                    "GET  /api/v1/games?q=keen\n"
                    "POST /api/v1/games/lookup\n"
                    "     {\"ra_md5\":\"…\",\"files\":[{\"name\":\"KEEN4E.EXE\",\"crc32\":\"…\"}]}\n"
                    "GET  /api/v1/games/{slug}\n"
                    "GET  /api/v1/games/{slug}/config?version=ID\n"
                    "GET  /api/v1/games/{slug}/config?format=ini\n"
                    "POST /api/v1/reports\n"
                    "GET  /api/v1/schema"
                </pre>
                <p>"Anonymous writes are rate limited per address."</p>
                <p class="muted">"IBM VGA font by VileR, int10h.org, CC BY-SA 4.0."</p>
            </Window>
        </div>
    }
}
