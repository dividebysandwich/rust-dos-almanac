use crate::app::components::{ActionError, ErrorLine, Light, RatingBar, RecommendedTable, SettingChips, Window, date, error_text};
use crate::app::fns::*;
use crate::app::use_session;
use crate::model::{GameDetail, Recommendation, RecommendationSource, ReportInfo, Scope, VersionInfo, file_lines};
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::components::A;
use leptos_router::hooks::{use_params_map, use_query_map};

/// The staff actions on this page; any of them finishing reloads the data.
#[derive(Clone, Copy)]
struct StaffActions {
    hide_report: ServerAction<SetReportHidden>,
    bless: ServerAction<BlessReport>,
    unbless: ServerAction<Unbless>,
    edit_game: ServerAction<EditGame>,
    hide_game: ServerAction<SetGameHidden>,
    edit_version: ServerAction<EditVersion>,
    hide_version: ServerAction<SetVersionHidden>,
}

impl StaffActions {
    fn new() -> Self {
        Self {
            hide_report: ServerAction::new(),
            bless: ServerAction::new(),
            unbless: ServerAction::new(),
            edit_game: ServerAction::new(),
            hide_game: ServerAction::new(),
            edit_version: ServerAction::new(),
            hide_version: ServerAction::new(),
        }
    }

    fn generation(&self) -> usize {
        self.hide_report.version().get()
            + self.bless.version().get()
            + self.unbless.version().get()
            + self.edit_game.version().get()
            + self.hide_game.version().get()
            + self.edit_version.version().get()
            + self.hide_version.version().get()
    }
}

#[component]
pub fn GamePage() -> impl IntoView {
    let params = use_params_map();
    let query = use_query_map();
    let slug = move || params.read().get("slug").unwrap_or_default();
    let version = move || query.read().get("v").and_then(|v| v.parse::<i64>().ok());
    let actions = StaffActions::new();
    provide_context(actions);

    let game = Resource::new(move || (slug(), actions.generation()), |(s, _)| get_game(s));
    let reports = Resource::new(move || (slug(), actions.generation()), |(s, _)| get_reports(s));
    let config = Resource::new(move || (slug(), version(), actions.generation()), |(s, v, _)| get_config(s, v));

    view! {
        <Transition fallback=|| view! { <p class="loading">"Loading…"</p> }>
            {move || Suspend::new(async move {
                let g = match game.await {
                    Ok(g) => g,
                    Err(e) => return view! { <Window title="Error"><ErrorLine message=error_text(&e) /></Window> }.into_any(),
                };
                let reports = reports.await;
                let config = config.await;
                view! { <GameView game=g reports config version=version() /> }.into_any()
            })}
        </Transition>
    }
}

#[component]
fn GameView(
    game: GameDetail,
    reports: Result<Vec<ReportInfo>, ServerFnError>,
    config: Result<Recommendation, ServerFnError>,
    version: Option<i64>,
) -> impl IntoView {
    let selected: Option<VersionInfo> = version.and_then(|v| game.versions.iter().find(|x| x.id == v).cloned());
    let slug = game.slug.clone();
    let report_href = match version {
        Some(v) => format!("/games/{slug}/report?v={v}"),
        None => format!("/games/{slug}/report"),
    };
    let title = game.name.clone();
    let reports = reports.map(|rs| match version {
        Some(v) => rs.into_iter().filter(|r| r.version_id == Some(v)).collect(),
        None => rs,
    });

    view! {
        <Title text=title.clone() />
        <div class="row-top">
            <GameHeader game=game.clone() report_href=report_href.clone() />
            <Versions game=game.clone() selected=version />
        </div>
        {selected.map(|v| view! { <VersionFiles slug=slug.clone() version=v /> })}
        <div class="row-2">
            <ConfigWindow slug=slug.clone() config />
            <Reports slug=slug.clone() reports report_href />
        </div>
    }
}

#[component]
fn GameHeader(game: GameDetail, report_href: String) -> impl IntoView {
    let session = use_session();
    let actions = expect_context::<StaffActions>();
    let slug = game.slug.clone();
    let version_href = format!("/games/{slug}/version");
    let meta: Vec<String> = [game.year.map(|y| y.to_string()), game.publisher.clone()].into_iter().flatten().collect();
    let aliases = game.aliases.join(" · ");
    let hidden = game.hidden;
    let edit = game.clone();
    view! {
        <Window title=game.name.to_uppercase() class="game-head">
            <div class="head-body">
                <h1 class="game-name">
                    {game.name.clone()}
                    {hidden.then(|| view! { <span class="badge">"hidden"</span> })}
                </h1>
                {(!meta.is_empty()).then(|| view! { <p class="meta">{meta.join(" · ")}</p> })}
                {(!aliases.is_empty()).then(|| view! { <p class="aliases">"Also known as: " {aliases}</p> })}
                <p class="rating-line">
                    <RatingBar summary=game.rating.clone() />
                </p>
                <p class="buttons">
                    <A href=report_href attr:class="btn is-default" attr:accesskey="r">
                        <span class="hk">"R"</span>
                        "eport how it runs"
                    </A>
                    <A href=version_href attr:class="btn" attr:accesskey="v">
                        "Add "
                        <span class="hk">"v"</span>
                        "ersion"
                    </A>
                </p>
                {move || {
                    session
                        .is_staff()
                        .then(|| {
                            let g = edit.clone();
                            let (slug_a, slug_b) = (g.slug.clone(), g.slug.clone());
                            let year = g.year.map(|y| y.to_string()).unwrap_or_default();
                            let publisher = g.publisher.clone().unwrap_or_default();
                            let aliases = g.aliases.join(", ");
                            let name = g.name.clone();
                            view! {
                                <div class="staff-bar">
                                    <ActionForm action=actions.hide_game attr:class="inline">
                                        <input type="hidden" name="slug" value=slug_a />
                                        <input type="hidden" name="hidden" value=(!hidden).to_string() />
                                        <button type="submit" class="btn small">
                                            {if hidden { "Unhide game" } else { "Hide game" }}
                                        </button>
                                    </ActionForm>
                                    <details class="edit">
                                        <summary class="btn small">"Edit game"</summary>
                                        <ActionForm action=actions.edit_game attr:class="form">
                                            <input type="hidden" name="slug" value=slug_b />
                                            <Field label="Name" name="name" value=name />
                                            <Field label="Year" name="year" value=year />
                                            <Field label="Publisher" name="publisher" value=publisher />
                                            <Field label="Aliases" name="aliases" value=aliases />
                                            <p class="form-buttons">
                                                <button type="submit" class="btn is-default">
                                                    "Save"
                                                </button>
                                            </p>
                                        </ActionForm>
                                    </details>
                                    <ActionError action=actions.edit_game />
                                    <ActionError action=actions.hide_game />
                                </div>
                            }
                        })
                }}
            </div>
        </Window>
    }
}

/// A labelled one-line input.
#[component]
pub fn Field(
    label: &'static str,
    name: &'static str,
    #[prop(optional, into)] value: String,
    #[prop(optional)] required: bool,
    #[prop(optional)] placeholder: &'static str,
    #[prop(optional, into)] kind: Option<&'static str>,
) -> impl IntoView {
    let id = format!("f-{name}");
    view! {
        <div class="field">
            <label for=id.clone()>{label}</label>
            <input
                id=id
                class="input"
                name=name
                type=kind.unwrap_or("text")
                value=value
                required=required
                placeholder=placeholder
                autocomplete="off"
            />
        </div>
    }
}

#[component]
fn Versions(game: GameDetail, selected: Option<i64>) -> impl IntoView {
    let slug = game.slug.clone();
    let all_href = format!("/games/{slug}");
    view! {
        <Window title="Versions" class="versions">
            <ul class="vlist" role="list">
                <li class:is-sel=selected.is_none()>
                    <A href=all_href>
                        <Light rating=game.rating.overall />
                        " All versions"
                    </A>
                </li>
                {game
                    .versions
                    .into_iter()
                    .map(|v| {
                        let href = format!("/games/{slug}?v={}", v.id);
                        view! {
                            <li class:is-sel=selected == Some(v.id) class:is-hidden=v.hidden>
                                <A href=href>
                                    <Light rating=v.rating.overall />
                                    " "
                                    {v.label}
                                    <span class="muted">{format!(" {} files", v.files.len())}</span>
                                </A>
                                {v.hidden.then(|| view! { <span class="badge">"hidden"</span> })}
                            </li>
                        }
                    })
                    .collect_view()}
            </ul>
        </Window>
    }
}

#[component]
fn VersionFiles(slug: String, version: VersionInfo) -> impl IntoView {
    let session = use_session();
    let actions = expect_context::<StaffActions>();
    let v = version.clone();
    view! {
        <Window title=format!("Files: {}", version.label) class="files-win">
            {if version.files.is_empty() {
                view! { <p class="muted">"No files listed for this version."</p> }.into_any()
            } else {
                view! {
                    <div class="scroll-x">
                        <table class="list files">
                            <thead>
                                <tr>
                                    <th scope="col">"File"</th>
                                    <th scope="col" class="num">"Size"</th>
                                    <th scope="col">"CRC32"</th>
                                    <th scope="col">"MD5"</th>
                                    <th scope="col">"SHA-1"</th>
                                </tr>
                            </thead>
                            <tbody>
                                {version
                                    .files
                                    .iter()
                                    .map(|f| {
                                        view! {
                                            <tr>
                                                <td>{f.name.to_uppercase()}</td>
                                                <td class="num">{f.size.map(|s| s.to_string()).unwrap_or_default()}</td>
                                                <td class="hash">{f.crc32.clone().unwrap_or_default()}</td>
                                                <td class="hash">{f.md5.clone().unwrap_or_default()}</td>
                                                <td class="hash">{f.sha1.clone().unwrap_or_default()}</td>
                                            </tr>
                                        }
                                    })
                                    .collect_view()}
                            </tbody>
                        </table>
                    </div>
                }
                    .into_any()
            }}
            {(!version.ra_md5.is_empty()).then(|| view! { <p class="ra">"Archive hash: " <span class="hash">{version.ra_md5.join(", ")}</span></p> })}
            {move || {
                session
                    .is_staff()
                    .then(|| {
                        let v = v.clone();
                        view! {
                            <div class="staff-bar">
                                <ActionForm action=actions.hide_version attr:class="inline">
                                    <input type="hidden" name="id" value=v.id.to_string() />
                                    <input type="hidden" name="hidden" value=(!v.hidden).to_string() />
                                    <button type="submit" class="btn small">
                                        {if v.hidden { "Unhide version" } else { "Hide version" }}
                                    </button>
                                </ActionForm>
                                <details class="edit">
                                    <summary class="btn small">"Edit version"</summary>
                                    <ActionForm action=actions.edit_version attr:class="form">
                                        <input type="hidden" name="id" value=v.id.to_string() />
                                        <Field label="Label" name="label" value=v.label.clone() required=true />
                                        <div class="field wide">
                                            <label for="f-files">"Files"</label>
                                            <textarea id="f-files" name="files" class="input" rows="8">
                                                {file_lines(&v.files)}
                                            </textarea>
                                        </div>
                                        <Field label="Archive hashes" name="ra_md5" value=v.ra_md5.join(", ") />
                                        <p class="form-buttons">
                                            <button type="submit" class="btn is-default">
                                                "Save"
                                            </button>
                                        </p>
                                    </ActionForm>
                                </details>
                                <ActionError action=actions.edit_version />
                                <ActionError action=actions.hide_version />
                            </div>
                        }
                    })
            }}
            <p class="muted small">{format!("Version id {} · /games/{slug}?v={}", version.id, version.id)}</p>
        </Window>
    }
}

#[component]
fn ConfigWindow(slug: String, config: Result<Recommendation, ServerFnError>) -> impl IntoView {
    let session = use_session();
    let actions = expect_context::<StaffActions>();
    let rec = match config {
        Ok(r) => r,
        Err(e) => return view! { <Window title="Recommended settings"><ErrorLine message=error_text(&e) /></Window> }.into_any(),
    };
    let scope = match (rec.scope, rec.version_id) {
        (Scope::Version, _) => "from reports for this version",
        (Scope::Game, Some(_)) => "from reports for all versions (too few for this one)",
        (Scope::Game, None) => "from reports for all versions",
    };
    let api = match rec.version_id {
        Some(v) => format!("/api/v1/games/{slug}/config?version={v}&format=ini"),
        None => format!("/api/v1/games/{slug}/config?format=ini"),
    };
    let blessed = matches!(rec.source, RecommendationSource::Blessed { .. });
    let version = rec.version_id;
    let badge = match &rec.source {
        RecommendationSource::Blessed { blessed_by, report_id } => view! {
            <p class="source blessed">
                "★ Blessed by "
                {blessed_by.clone().unwrap_or_else(|| "staff".into())}
                " · "
                <a href=format!("#report-{report_id}")>"report #"{*report_id}</a>
            </p>
        }
        .into_any(),
        RecommendationSource::Consensus { reports } => view! {
            <p class="source">{format!("Community consensus of {reports} report{}, {scope}", if *reports == 1 { "" } else { "s" })}</p>
        }
        .into_any(),
        RecommendationSource::None => view! { <p class="source">"No settings reported yet."</p> }.into_any(),
    };
    let has_settings = !rec.settings.is_empty();
    let ini = rec.ini.clone();
    view! {
        <Window title="Recommended settings" class="config">
            {badge}
            {has_settings
                .then(|| {
                    view! {
                        <RecommendedTable settings=rec.settings.clone() />
                        <h3 class="sub">"rust-dos.conf"</h3>
                        <pre class="code ini">{ini}</pre>
                        <p class="muted small">
                            <a href=api.clone()>{api.clone()}</a>
                        </p>
                    }
                })}
            {move || {
                let slug = slug.clone();
                (blessed && session.is_staff())
                    .then(move || {
                        view! {
                            <ActionForm action=actions.unbless attr:class="inline">
                                <input type="hidden" name="slug" value=slug.clone() />
                                {version.map(|v| view! { <input type="hidden" name="version" value=v.to_string() /> })}
                                <button type="submit" class="btn small">
                                    "Remove blessing"
                                </button>
                            </ActionForm>
                            <ActionError action=actions.unbless />
                        }
                    })
            }}
        </Window>
    }
    .into_any()
}

#[component]
fn Reports(slug: String, reports: Result<Vec<ReportInfo>, ServerFnError>, report_href: String) -> impl IntoView {
    let body = match reports {
        Ok(rs) if rs.is_empty() => view! {
            <p class="empty">
                "Nobody has reported on this yet. "
                <A href=report_href>"Be the first."</A>
            </p>
        }
        .into_any(),
        Ok(rs) => view! { <ol class="reports">{rs.into_iter().map(|r| view! { <ReportCard slug=slug.clone() report=r /> }).collect_view()}</ol> }
            .into_any(),
        Err(e) => view! { <ErrorLine message=error_text(&e) /> }.into_any(),
    };
    view! {
        <Window title="Reports" class="reports-win">
            {body}
        </Window>
    }
}

#[component]
fn ReportCard(slug: String, report: ReportInfo) -> impl IntoView {
    let session = use_session();
    let actions = expect_context::<StaffActions>();
    let r = report.clone();
    let anchor = format!("report-{}", report.id);
    view! {
        <li id=anchor class="report" class:is-hidden=report.hidden class:is-blessed=report.blessed>
            <div class="report-head">
                <Light rating=Some(report.rating) label=true />
                {report.blessed.then(|| view! { <span class="badge star" title="Recommended configuration">"★ blessed"</span> })}
                {report.hidden.then(|| view! { <span class="badge">"hidden"</span> })}
            </div>
            <p class="report-meta">
                {date(report.created_at)}
                {report.version_label.clone().map(|l| format!(" · {l}"))}
                {report.rust_dos_version.clone().map(|v| format!(" · Rust-DOS {v}"))}
                " · "
                {match &report.author {
                    Some(a) => view! { <span class="staff">{a.clone()}</span> }.into_any(),
                    None => view! { <span>"anonymous"</span> }.into_any(),
                }}
            </p>
            {report.description.clone().map(|d| view! { <p class="description">{d}</p> })}
            <SettingChips settings=report.settings.clone() />
            {move || {
                session
                    .is_staff()
                    .then(|| {
                        let r = r.clone();
                        let edit = format!("/reports/{}/edit?game={}", r.id, slug);
                        let source = r.source.clone().unwrap_or_default();
                        view! {
                            <div class="staff-bar">
                                <ActionForm action=actions.hide_report attr:class="inline">
                                    <input type="hidden" name="id" value=r.id.to_string() />
                                    <input type="hidden" name="hidden" value=(!r.hidden).to_string() />
                                    <button type="submit" class="btn small">
                                        {if r.hidden { "Unhide" } else { "Hide" }}
                                    </button>
                                </ActionForm>
                                {(!r.blessed && !r.hidden)
                                    .then(|| {
                                        view! {
                                            <ActionForm action=actions.bless attr:class="inline">
                                                <input type="hidden" name="id" value=r.id.to_string() />
                                                <button type="submit" class="btn small">
                                                    "★ Bless"
                                                </button>
                                            </ActionForm>
                                        }
                                    })}
                                <A href=edit attr:class="btn small">
                                    "Edit"
                                </A>
                                <A href=format!("/mod?source={source}") attr:class="source-link" attr:title="Everything from this source">
                                    {format!("src {}", source.chars().take(8).collect::<String>())}
                                </A>
                            </div>
                        }
                    })
            }}
        </li>
    }
}
