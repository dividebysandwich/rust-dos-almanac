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
                        let agree = match (s.agree, s.voters, s.implied) {
                            (_, _, true) => "platform".to_string(),
                            (Some(a), Some(v), _) => format!("{a}/{v}"),
                            _ => "★".to_string(),
                        };
                        let title = s.implied.then_some("A Windows program needs the built-in Windows; no report said otherwise");
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
                                <td data-label="Agree" class="num" class:implied=s.implied title=title>
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

/// "DOS", "Win 3.1" or "Win 9x", as a small badge.
#[component]
pub fn PlatformBadge(platform: crate::model::Platform) -> impl IntoView {
    let class = format!("badge platform platform-{}", platform.as_str());
    view! { <span class=class title=platform.label()>{platform.badge()}</span> }
}

/// The platform drop-down of the version forms; empty means unknown.
#[component]
pub fn PlatformSelect(#[prop(default = None)] value: Option<crate::model::Platform>) -> impl IntoView {
    view! {
        <div class="field">
            <label for="f-platform">"Platform"</label>
            <span class="select">
                <select id="f-platform" name="platform" class="input">
                    <option value="" selected=value.is_none()>
                        "Unknown"
                    </option>
                    {crate::model::Platform::ALL
                        .map(|p| view! { <option value=p.as_str() selected=value == Some(p)>{p.label()}</option> })
                        .collect_view()}
                </select>
            </span>
        </div>
    }
}

/// A box to paste a Rust-DOS game profile (`games/<id>.conf`) or
/// configuration file into, which fills in the form around it: the
/// fields with the ids `PastedConfig` has values for (`f-name`, `f-year`,
/// `f-publisher`, `f-notes`, `f-key`, and `s-<section>-<key>` for the
/// settings). The text itself is never sent.
#[component]
pub fn PasteConfig() -> impl IntoView {
    let text = RwSignal::new(String::new());
    let outcome = RwSignal::new(None::<(String, bool)>);
    let fill = move |_| {
        let pasted = crate::model::parse_config(&text.get_untracked());
        outcome.set(Some(fill_form(&pasted)));
    };
    view! {
        <details class="paste-config">
            <summary>"Paste your Rust-DOS game profile to fill in the form"</summary>
            <div class="field wide">
                <label for="f-paste">"Profile"</label>
                <textarea
                    id="f-paste"
                    class="input"
                    rows="8"
                    spellcheck="false"
                    placeholder="[game]
name=Commander Keen 4
year=1991

[emulator]
cpu=486"
                    on:input=move |ev| text.set(event_target_value(&ev))
                ></textarea>
                <p class="hint">
                    "The game's .conf from the games folder beside rust-dos.conf. Only the game's name, year, publisher, notes and UUID and the machine's settings are taken; drives, commands and paths stay with you. With the profile's UUID, this report replaces the one the profile sent before."
                </p>
            </div>
            <p class="form-buttons">
                <button type="button" class="btn" on:click=fill>
                    "Fill in the form"
                </button>
            </p>
            {move || {
                outcome
                    .get()
                    .map(|(text, problem)| {
                        view! {
                            <p class=if problem { "error" } else { "hint" } role="status">
                                {text}
                            </p>
                        }
                    })
            }}
        </details>
    }
}

/// Put `pasted` into the form's fields, the settings' all set again so
/// the form has the profile's and no others. What it did, and whether
/// something couldn't be taken.
#[cfg(feature = "hydrate")]
fn fill_form(pasted: &crate::model::PastedConfig) -> (String, bool) {
    use leptos::web_sys::{HtmlInputElement, HtmlOptionElement, HtmlSelectElement, HtmlTextAreaElement};
    use wasm_bindgen::JsCast;

    let doc = document();
    let set = |id: &str, value: &str| -> bool {
        let Some(el) = doc.get_element_by_id(id) else { return false };
        if let Some(input) = el.dyn_ref::<HtmlInputElement>() {
            input.set_value(value);
        } else if let Some(area) = el.dyn_ref::<HtmlTextAreaElement>() {
            area.set_value(value);
        } else if let Some(select) = el.dyn_ref::<HtmlSelectElement>() {
            select.set_value(value);
            if select.value() != value {
                // A value outside the presets, such as memsize=24.
                if let Ok(option) = HtmlOptionElement::new_with_text_and_value(value, value) {
                    let _ = select.add_with_html_option_element(&option);
                }
                select.set_value(value);
            }
        } else {
            return false;
        }
        true
    };
    let has_settings = settings_schema::SETTINGS.iter().any(|d| doc.get_element_by_id(&format!("s-{}-{}", d.section, d.key)).is_some());
    if has_settings {
        for d in settings_schema::SETTINGS {
            set(&format!("s-{}-{}", d.section, d.key), "");
        }
    }
    let mut filled = Vec::new();
    let settings = pasted.settings.iter().filter(|(s, k, v)| set(&format!("s-{s}-{k}"), v)).count();
    if settings > 0 {
        filled.push(format!("{settings} setting{}", if settings == 1 { "" } else { "s" }));
    }
    let year = pasted.year.map(|y| y.to_string());
    let fields = [
        ("f-name", pasted.name.as_deref(), "the name"),
        ("f-year", year.as_deref(), "the year"),
        ("f-publisher", pasted.publisher.as_deref(), "the publisher"),
        ("f-notes", pasted.notes.as_deref(), "the notes"),
        ("f-key", pasted.key.as_deref(), "the UUID"),
    ];
    for (id, value, what) in fields {
        if let Some(v) = value
            && set(id, v)
        {
            filled.push(what.to_string());
        }
    }
    let mut text = if filled.is_empty() {
        "Nothing in it fits this form.".to_string()
    } else {
        format!("Filled in {}.", list(&filled))
    };
    if !pasted.ignored.is_empty() {
        let names: Vec<&str> = pasted.ignored.iter().map(|i| i.split_once('.').map_or(i.as_str(), |(_, k)| k)).collect();
        text.push_str(&format!(" Not for the almanac: {}.", names.join(", ")));
    }
    if !pasted.problems.is_empty() {
        text.push_str(&format!(" Left out: {}.", pasted.problems.join("; ")));
    }
    (text, !pasted.problems.is_empty() || filled.is_empty())
}

#[cfg(not(feature = "hydrate"))]
fn fill_form(_: &crate::model::PastedConfig) -> (String, bool) {
    ("The page is still loading.".to_string(), true)
}

/// "a, b and c".
#[cfg_attr(not(feature = "hydrate"), allow(dead_code))]
fn list(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}
