use crate::model::{Rating, RatingSummary, RecommendedSetting, Setting};
use crate::settings_schema;
use leptos::prelude::*;

/// A blue Turbo Vision window with a double frame and a centred title.
#[component]
pub fn Window(#[prop(into)] title: Signal<String>, #[prop(optional, into)] class: String, children: Children) -> impl IntoView {
    view! {
        <section class=format!("win is-active {class}")>
            <h2 class="win-title">{move || title.get()}</h2>
            <div class="win-body">{children()}</div>
        </section>
    }
}

/// A gray dialog window, for forms.
#[component]
pub fn Dialog(#[prop(into)] title: Signal<String>, #[prop(optional, into)] class: String, children: Children) -> impl IntoView {
    view! {
        <section class=format!("win dialog is-active {class}")>
            <h2 class="win-title">{move || title.get()}</h2>
            <div class="win-body">{children()}</div>
        </section>
    }
}

/// One stoplight lamp, with its meaning spelled out for screen readers
/// and for anyone who can't tell the colours apart.
#[component]
pub fn Light(rating: Option<Rating>, #[prop(optional)] label: bool) -> impl IntoView {
    let (class, text) = match rating {
        Some(r) => (format!("light light-{}", r.as_str()), r.label()),
        None => ("light light-none".to_string(), "No reports yet"),
    };
    view! {
        <span class="lamp-wrap" title=text>
            <span class=class aria-hidden="true">
                "■"
            </span>
            {if label { view! { <span class="lamp-text">{text}</span> }.into_any() } else { view! { <span class="sr-only">{text}</span> }.into_any() }}
        </span>
    }
}

/// A traffic light drawn in box characters, each lamp beside its meaning.
#[component]
pub fn StopLight() -> impl IntoView {
    let lamp = |r: Rating, name: &'static str| {
        let class = format!("light-{}", r.as_str());
        view! {
            <span class="tl-frame">"║  "</span>
            <span class=class.clone()>"▄███▄"</span>
            <span class="tl-frame">"  ║"</span>
            "  "
            <span class=format!("tl-name {class}")>{name}</span>
            "\n"
            <span class="tl-frame">"║  "</span>
            <span class=class>"▀███▀"</span>
            <span class="tl-frame">"  ║"</span>
            "  "
            <span class="tl-text">{r.label()}</span>
            "\n"
        }
    };
    let divider = || view! { <span class="tl-frame">"╟─────────╢\n"</span> };
    view! {
        <pre class="traffic-light" role="img" aria-label="Stoplight: green runs great, yellow is playable with some problems, red is not playable">
            <span class="tl-frame">"   ▄▄▄▄▄\n╔═════════╗\n"</span>
            {lamp(Rating::Green, "GREEN")}
            {divider()}
            {lamp(Rating::Yellow, "YELLOW")}
            {divider()}
            {lamp(Rating::Red, "RED")}
            <span class="tl-frame">"╚════╤════╝\n     │\n     │\n   ▄▄█▄▄"</span>
        </pre>
    }
}

/// The overall lamp and how many reports gave each colour.
#[component]
pub fn RatingBar(summary: RatingSummary) -> impl IntoView {
    view! {
        <span class="rating-bar">
            <Light rating=summary.overall label=true />
            <Counts summary />
        </span>
    }
}

/// How many reports gave each colour.
#[component]
pub fn Counts(summary: RatingSummary) -> impl IntoView {
    view! {
        <span class="counts" aria-label="Reports by rating">
            <span class="c-green" title="Runs great">
                "■ "
                {summary.green}
            </span>
            <span class="c-yellow" title="Playable">
                "■ "
                {summary.yellow}
            </span>
            <span class="c-red" title="Not playable">
                "■ "
                {summary.red}
            </span>
        </span>
    }
}

pub fn label_of(section: &str, key: &str) -> String {
    settings_schema::find(section, key).map(|d| d.label.to_string()).unwrap_or_else(|| format!("{section}.{key}"))
}

/// The recommended settings, with how many reports agree on each.
#[component]
pub fn RecommendedTable(settings: Vec<RecommendedSetting>) -> impl IntoView {
    view! {
        <table class="list kv">
            <thead>
                <tr>
                    <th scope="col">"Setting"</th>
                    <th scope="col">"Value"</th>
                    <th scope="col" class="num">
                        "Agree"
                    </th>
                </tr>
            </thead>
            <tbody>
                {settings
                    .into_iter()
                    .map(|s| {
                        let agree = match (s.agree, s.voters) {
                            (Some(a), Some(v)) => format!("{a}/{v}"),
                            _ => "★".to_string(),
                        };
                        view! {
                            <tr>
                                <td data-label="Setting">
                                    {label_of(&s.section, &s.key)}
                                    " "
                                    <span class="key">{s.key.clone()}</span>
                                </td>
                                <td data-label="Value" class="val">
                                    {s.value}
                                </td>
                                <td data-label="Agree" class="num">
                                    {agree}
                                </td>
                            </tr>
                        }
                    })
                    .collect_view()}
            </tbody>
        </table>
    }
}

/// A report's settings as compact `key=value` chips.
#[component]
pub fn SettingChips(settings: Vec<Setting>) -> impl IntoView {
    if settings.is_empty() {
        return view! { <p class="muted">"No settings given."</p> }.into_any();
    }
    view! {
        <ul class="chips">
            {settings
                .into_iter()
                .map(|s| {
                    let title = label_of(&s.section, &s.key);
                    view! {
                        <li title=title>
                            <span class="key">{s.key}</span>
                            "="
                            <span class="val">{s.value}</span>
                        </li>
                    }
                })
                .collect_view()}
        </ul>
    }
    .into_any()
}

/// A server error, as a red Turbo Vision message box line.
#[component]
pub fn ErrorLine(#[prop(into)] message: String) -> impl IntoView {
    let message = message.trim_start_matches("error running server function: ").to_string();
    view! {
        <p class="error" role="alert">
            <span class="hk">"Error: "</span>
            {message}
        </p>
    }
}

pub fn error_text(e: &ServerFnError) -> String {
    match e {
        ServerFnError::ServerError(m) => m.clone(),
        other => other.to_string(),
    }
}

/// The error of the last submission of `action`, if it failed.
#[component]
pub fn ActionError<I, O>(action: ServerAction<I>) -> impl IntoView
where
    I: leptos::server_fn::ServerFn<Output = O, Error = ServerFnError> + Clone + Send + Sync + 'static,
    O: Clone + Send + Sync + 'static,
{
    move || match action.value().get() {
        Some(Err(e)) => Some(view! { <ErrorLine message=error_text(&e) /> }),
        _ => None,
    }
}

pub fn date(ts: i64) -> String {
    crate::model::format_date(ts)
}
