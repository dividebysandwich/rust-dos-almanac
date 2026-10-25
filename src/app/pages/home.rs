use crate::app::components::{Counts, ErrorLine, Light, StopLight, Window, date, error_text};
use crate::app::fns::search_games;
use crate::model::GameSummary;
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::components::A;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_navigate, use_query_map};
use std::time::Duration;

/// How long typing has to pause before the results follow.
const SEARCH_DELAY: Duration = Duration::from_millis(250);

#[component]
pub fn HomePage() -> impl IntoView {
    let query = use_query_map();
    let q = move || query.read().get("q").unwrap_or_default();
    let page = move || query.read().get("page").and_then(|p| p.parse::<u64>().ok()).unwrap_or(0);
    let results = Resource::new(move || (q(), page()), |(q, p)| search_games(q, p));
    let pending = RwSignal::new(false);

    // Search as you type: the query lives in the URL, so results stay
    // linkable and the back button works. Typing replaces the history entry
    // rather than adding one per keystroke.
    let navigate = use_navigate();
    let input = NodeRef::<leptos::html::Input>::new();
    let timer = StoredValue::new(None::<TimeoutHandle>);
    let sent = StoredValue::new(None::<String>);
    let search = move |text: String| {
        let text = text.trim().to_string();
        sent.set_value(Some(text.clone()));
        let url = if text.is_empty() { "/".to_string() } else { format!("/?q={}", urlencode(&text)) };
        navigate(&url, NavigateOptions { replace: true, scroll: false, ..Default::default() });
    };
    let cancel = move || {
        if let Some(t) = timer.get_value() {
            t.clear();
        }
    };
    let on_input = {
        let search = search.clone();
        move |ev: leptos::ev::Event| {
            cancel();
            let text = event_target_value(&ev);
            let search = search.clone();
            timer.set_value(set_timeout_with_handle(move || search(text), SEARCH_DELAY).ok());
        }
    };
    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        cancel();
        if let Some(el) = input.get() {
            search(el.value());
        }
    };
    // When the URL changes some other way (back, forward, a link), show its
    // query in the box; never overwrite what is being typed.
    Effect::new(move |_| {
        let q = q();
        if sent.get_value().as_deref() != Some(q.as_str())
            && let Some(el) = input.get()
        {
            el.set_value(&q);
        }
    });

    view! {
        <Title text="" />
        <div class="row-top">
            <Window title="RUST-DOS ALMANAC" class="hero">
                <div class="hero-body">
                    <h1 class="logo">"Rust-DOS Almanac"</h1>
                    <p class="tagline">
                        "How DOS games run, and the settings that make them run well"
                        <span class="cursor" aria-hidden="true">
                            "_"
                        </span>
                    </p>
                    <form method="get" action="/" class="search" role="search" on:submit=on_submit>
                        <label for="q">
                            <span class="hk">"S"</span>
                            "earch"
                        </label>
                        <input
                            id="q"
                            name="q"
                            class="input"
                            type="search"
                            value=q
                            accesskey="s"
                            placeholder="Game name"
                            autocomplete="off"
                            aria-controls="results"
                            node_ref=input
                            on:input=on_input
                        />
                        <button class="btn is-default" type="submit">
                            "Search"
                        </button>
                    </form>
                    <p class="buttons">
                        <A href="/new" attr:class="btn">
                            <span class="hk">"N"</span>
                            "ew game"
                        </A>
                        <A href="/about" attr:class="btn">
                            <span class="hk">"A"</span>
                            "bout"
                        </A>
                    </p>
                </div>
            </Window>
            <Window title="Stoplight" class="legend-win">
                <StopLight />
                <p class="muted">"Settings are what most players found best. A ★ means a moderator picked them."</p>
            </Window>
        </div>
        <Window
            title=Signal::derive(move || {
                let title = if q().is_empty() { "Recently updated".to_string() } else { format!("Search: {}", q()) };
                if pending.get() { format!("{title} …") } else { title }
            })
            class="results"
        >
            <div id="results" class="results-body" class:is-pending=move || pending.get()>
            <Transition fallback=|| view! { <p class="muted">"Loading…"</p> } set_pending=pending>
                {move || Suspend::new(async move {
                    match results.await {
                        Ok(games) if games.is_empty() => {
                            view! {
                                <p class="sr-only" role="status">"No games found"</p>
                                <p class="empty">
                                    "No games found. "
                                    <A href="/new">"Add one?"</A>
                                </p>
                            }
                                .into_any()
                        }
                        Ok(games) => {
                            let full = games.len() as u64 >= 50;
                            let count = if full { "50 or more games".to_string() } else { format!("{} game{}", games.len(), if games.len() == 1 { "" } else { "s" }) };
                            view! {
                                <p class="sr-only" role="status">{count}</p>
                                <GameList games />
                                <Pager q=q() page=page() more=full />
                            }
                                .into_any()
                        }
                        Err(e) => view! { <ErrorLine message=error_text(&e) /> }.into_any(),
                    }
                })}
            </Transition>
            </div>
        </Window>
    }
}

#[component]
fn Pager(q: String, page: u64, more: bool) -> impl IntoView {
    let link = move |p: u64| format!("/?q={}&page={p}", urlencode(&q));
    let prev = (page > 0).then(|| link(page - 1));
    let next = more.then(|| link(page + 1));
    view! {
        <p class="pager">
            {prev.map(|h| view! { <A href=h attr:class="btn">"◄ Previous"</A> })}
            {next.map(|h| view! { <A href=h attr:class="btn">"Next ►"</A> })}
        </p>
    }
}

pub fn urlencode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            b' ' => "+".into(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[component]
pub fn GameList(games: Vec<GameSummary>) -> impl IntoView {
    view! {
        <table class="list games">
            <thead>
                <tr>
                    <th scope="col" class="lamp-col">
                        <span class="sr-only">"Rating"</span>
                    </th>
                    <th scope="col">"Game"</th>
                    <th scope="col">"Year"</th>
                    <th scope="col">"Publisher"</th>
                    <th scope="col">"Reports"</th>
                    <th scope="col">"Updated"</th>
                </tr>
            </thead>
            <tbody>
                {games
                    .into_iter()
                    .map(|g| {
                        let href = format!("/games/{}", g.slug);
                        view! {
                            <tr class:is-hidden=g.hidden>
                                <td class="lamp-col">
                                    <Light rating=g.rating.overall />
                                </td>
                                <td class="name">
                                    <A href=href>{g.name}</A>
                                    {g.hidden.then(|| view! { <span class="badge">"hidden"</span> })}
                                </td>
                                <td data-label="Year">{g.year.map(|y| y.to_string()).unwrap_or_default()}</td>
                                <td data-label="Publisher">{g.publisher.unwrap_or_default()}</td>
                                <td data-label="Reports">
                                    <Counts summary=g.rating />
                                </td>
                                <td data-label="Updated">{date(g.updated_at)}</td>
                            </tr>
                        }
                    })
                    .collect_view()}
            </tbody>
        </table>
    }
}
