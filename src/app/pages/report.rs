use crate::app::components::{ActionError, Dialog, ErrorLine, Light, Window, error_text};
use crate::app::fns::{EditReport, SubmitReport, get_game, get_report};
use crate::app::use_session;
use crate::model::{GameDetail, Rating, ReportInfo};
use crate::settings_schema::{Domain, SECTIONS, SETTINGS, SettingDef};
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::components::A;
use leptos_router::hooks::{use_params_map, use_query_map};

const SECTION_TITLES: [(&str, &str); 3] = [("emulator", "Machine"), ("sound", "Sound"), ("joystick", "Input")];

/// The fields of a report, empty or filled from `existing`.
#[component]
fn ReportFields(game: GameDetail, version: Option<i64>, existing: Option<ReportInfo>) -> impl IntoView {
    let session = use_session();
    let value_of = {
        let existing = existing.clone();
        move |d: &SettingDef| {
            existing
                .as_ref()
                .and_then(|r| r.settings.iter().find(|s| s.section == d.section && s.key == d.key))
                .map(|s| s.value.clone())
                .unwrap_or_default()
        }
    };
    let rating = existing.as_ref().map(|r| r.rating);
    let version = existing.as_ref().map_or(version, |r| r.version_id);
    let rdv = existing.as_ref().and_then(|r| r.rust_dos_version.clone()).unwrap_or_default();
    let description = existing.as_ref().and_then(|r| r.description.clone()).unwrap_or_default();

    view! {
        <input type="hidden" name="game" value=game.slug.clone() />
        <fieldset class="stoplight">
            <legend>"How well does it run?"</legend>
            <div class="cluster">
            {Rating::ALL
                .map(|r| {
                    view! {
                        <label class="radio">
                            <input type="radio" name="rating" value=r.as_str() required=true checked=rating == Some(r) />
                            <span>
                                <Light rating=Some(r) label=true />
                            </span>
                        </label>
                    }
                })
                .collect_view()}
            </div>
        </fieldset>
        <div class="field">
            <label for="f-version">"Version"</label>
            <span class="select">
                <select id="f-version" name="version" class="input">
                    <option value="" selected=version.is_none()>
                        "Any / not sure"
                    </option>
                    {game
                        .versions
                        .iter()
                        .map(|v| view! { <option value=v.id.to_string() selected=version == Some(v.id)>{v.label.clone()}</option> })
                        .collect_view()}
                </select>
            </span>
        </div>
        <div class="field">
            <label for="f-rdv">"Rust-DOS version"</label>
            <input id="f-rdv" name="rust_dos_version" class="input" value=rdv placeholder="1.7.5" autocomplete="off" />
        </div>
        <p class="hint">"Settings you leave at “—” count as not given. Only fill in what you actually used."</p>
        <div class="settings-grid">
            {SECTIONS
                .iter()
                .map(|section| {
                    let title = SECTION_TITLES.iter().find(|(s, _)| s == section).map_or(*section, |(_, t)| *t);
                    view! {
                        <fieldset class="settings">
                            <legend>{title} <span class="key">{format!(" [{section}]")}</span></legend>
                            {SETTINGS
                                .iter()
                                .filter(|d| d.section == *section)
                                .map(|d| view! { <SettingInput def=*d value=value_of(d) /> })
                                .collect_view()}
                        </fieldset>
                    }
                })
                .collect_view()}
        </div>
        {move || {
            session
                .is_staff()
                .then(|| {
                    view! {
                        <div class="field wide">
                            <label for="f-desc">"Experience (staff)"</label>
                            <textarea id="f-desc" name="description" class="input" rows="5">
                                {description.clone()}
                            </textarea>
                        </div>
                    }
                })
        }}
    }
}

#[component]
fn SettingInput(def: SettingDef, value: String) -> impl IntoView {
    let id = format!("s-{}-{}", def.section, def.key);
    let name = format!("settings[{}.{}]", def.section, def.key);
    let control = match (def.options(), def.domain) {
        (Some(options), _) => {
            let known = value.is_empty() || options.contains(&value);
            view! {
                <span class="select">
                    <select id=id.clone() name=name class="input">
                        <option value="" selected=value.is_empty()>
                            "—"
                        </option>
                        // A stored value outside the presets (e.g. memsize 24) stays selectable.
                        {(!known).then(|| view! { <option value=value.clone() selected=true>{value.clone()}</option> })}
                        {options
                            .into_iter()
                            .map(|o| {
                                let sel = o == value;
                                let text = o.clone();
                                view! { <option value=o selected=sel>{text}</option> }
                            })
                            .collect_view()}
                    </select>
                </span>
            }
            .into_any()
        }
        (None, Domain::Cycles) => view! {
            <input id=id.clone() name=name class="input" value=value list="cycles-presets" placeholder="max, auto, 20000" autocomplete="off" />
            <datalist id="cycles-presets">
                {["max", "auto", "auto 20000", "3000", "8000", "20000", "50000", "100000"].map(|c| view! { <option value=c /> }).collect_view()}
            </datalist>
        }
        .into_any(),
        (None, _) => view! { <input id=id.clone() name=name class="input" value=value autocomplete="off" /> }.into_any(),
    };
    view! {
        <div class="field">
            <label for=id>{def.label}</label>
            {control}
        </div>
    }
}

#[component]
pub fn NewReportPage() -> impl IntoView {
    let params = use_params_map();
    let query = use_query_map();
    let slug = move || params.read().get("slug").unwrap_or_default();
    let version = move || query.read().get("v").and_then(|v| v.parse::<i64>().ok());
    let game = Resource::new(slug, get_game);
    let submit = ServerAction::<SubmitReport>::new();

    view! {
        <Title text="Report" />
        <Transition fallback=|| view! { <p class="loading">"Loading…"</p> }>
            {move || Suspend::new(async move {
                match game.await {
                    Ok(g) => {
                        let back = format!("/games/{}", g.slug);
                        view! {
                            <Dialog title=format!("Report: {}", g.name) class="form-dialog report-form">
                                <ActionForm action=submit attr:class="form">
                                    <ReportFields game=g.clone() version=version() existing=None />
                                    <ActionError action=submit />
                                    <p class="form-buttons">
                                        <button type="submit" class="btn is-default">
                                            "Submit report"
                                        </button>
                                        <A href=back attr:class="btn">
                                            "Cancel"
                                        </A>
                                    </p>
                                </ActionForm>
                            </Dialog>
                        }
                            .into_any()
                    }
                    Err(e) => view! { <Window title="Error"><ErrorLine message=error_text(&e) /></Window> }.into_any(),
                }
            })}
        </Transition>
    }
}

#[component]
pub fn EditReportPage() -> impl IntoView {
    let params = use_params_map();
    let query = use_query_map();
    let id = move || params.read().get("id").and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);
    let slug = move || query.read().get("game").unwrap_or_default();
    let report = Resource::new(id, get_report);
    let game = Resource::new(slug, get_game);
    let edit = ServerAction::<EditReport>::new();

    view! {
        <Title text="Edit report" />
        <Transition fallback=|| view! { <p class="loading">"Loading…"</p> }>
            {move || Suspend::new(async move {
                match (report.await, game.await) {
                    (Ok(r), Ok(g)) => {
                        let back = format!("/games/{}#report-{}", g.slug, r.id);
                        view! {
                            <Dialog title=format!("Edit report #{}: {}", r.id, g.name) class="form-dialog report-form">
                                <ActionForm action=edit attr:class="form">
                                    <input type="hidden" name="id" value=r.id.to_string() />
                                    <ReportFields game=g.clone() version=r.version_id existing=Some(r.clone()) />
                                    <ActionError action=edit />
                                    <p class="form-buttons">
                                        <button type="submit" class="btn is-default">
                                            "Save"
                                        </button>
                                        <A href=back attr:class="btn">
                                            "Cancel"
                                        </A>
                                    </p>
                                </ActionForm>
                            </Dialog>
                        }
                            .into_any()
                    }
                    (Err(e), _) | (_, Err(e)) => view! { <Window title="Error"><ErrorLine message=error_text(&e) /></Window> }.into_any(),
                }
            })}
        </Transition>
    }
}
