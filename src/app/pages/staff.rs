use super::game::Field;
use super::home::{GameList, urlencode};
use crate::app::components::{ActionError, Dialog, ErrorLine, Light, SettingChips, Window, date, error_text};
use crate::app::fns::*;
use crate::app::use_session;
use crate::model::{ModReport, Role};
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::components::A;
use leptos_router::hooks::use_query_map;

#[component]
pub fn LoginPage() -> impl IntoView {
    let session = use_session();
    let query = use_query_map();
    let next = move || query.read().get("next").unwrap_or_default();
    view! {
        <Title text="Login" />
        <div class="row-form narrow-form">
            <Dialog title="Login" class="form-dialog login">
                <ActionForm action=session.login attr:class="form">
                    <input type="hidden" name="next" value=next />
                    <Field label="Username" name="username" required=true />
                    <Field label="Password" name="password" kind="password" required=true />
                    <ActionError action=session.login />
                    <p class="form-buttons">
                        <button type="submit" class="btn is-default">
                            "OK"
                        </button>
                        <A href="/" attr:class="btn">
                            "Cancel"
                        </A>
                    </p>
                </ActionForm>
                <p class="muted small center">"For moderators and admins. Everybody else can report without an account."</p>
            </Dialog>
        </div>
    }
}

/// What staff pages show to everybody else.
#[component]
fn NeedsLogin() -> impl IntoView {
    view! {
        <Dialog title="Login required" class="narrow">
            <p class="center">"This page is for moderators."</p>
            <p class="center">
                <A href="/login" attr:class="btn is-default">
                    "Login"
                </A>
            </p>
        </Dialog>
    }
}

#[component]
pub fn ModerationPage() -> impl IntoView {
    let session = use_session();
    let query = use_query_map();
    let hidden_only = move || query.read().get("hidden").is_some_and(|v| v == "1");
    let source = move || query.read().get("source").unwrap_or_default();
    let page = move || query.read().get("page").and_then(|p| p.parse::<u64>().ok()).unwrap_or(0);

    let hide = ServerAction::<SetReportHidden>::new();
    let hide_src = ServerAction::<HideSource>::new();
    let generation = move || hide.version().get() + hide_src.version().get();
    let reports = Resource::new(move || (hidden_only(), source(), page(), generation()), |(h, s, p, _)| mod_reports(h, s, p));
    let games = Resource::new(generation, |_| hidden_games());

    view! {
        <Title text="Moderation" />
        <Transition fallback=|| ()>
            {move || {
                Suspend::new(async move {
                    if session.user.await.ok().flatten().is_none() {
                        return view! { <NeedsLogin /> }.into_any();
                    }
                    let reports = reports.await;
                    let games = games.await;
                    view! {
                        <div class="row-top">
                            <Dialog title="Filter" class="filter">
                                <form method="get" action="/mod" class="form">
                                    <Field label="Source" name="source" value=source() placeholder="hashed address" />
                                    <label class="check">
                                        <input type="checkbox" name="hidden" value="1" checked=hidden_only() />
                                        <span>"Hidden only"</span>
                                    </label>
                                    <p class="form-buttons">
                                        <button type="submit" class="btn is-default">
                                            "Filter"
                                        </button>
                                        <A href="/mod" attr:class="btn">
                                            "Clear"
                                        </A>
                                        <A href="/mod/log" attr:class="btn">
                                            "Log"
                                        </A>
                                    </p>
                                </form>
                                {(!source().is_empty())
                                    .then(|| {
                                        view! {
                                            <ActionForm action=hide_src attr:class="form danger-zone">
                                                <input type="hidden" name="source" value=source() />
                                                <p>"Hide every anonymous report, game and version from this source:"</p>
                                                <button type="submit" class="btn">
                                                    "Hide all from source"
                                                </button>
                                            </ActionForm>
                                        }
                                    })}
                                {move || hide_src.value().get().and_then(|r| r.ok()).map(|n| view! { <p class="ok">{format!("{n} entries hidden.")}</p> })}
                                <ActionError action=hide_src />
                            </Dialog>
                            <Window title="Hidden games">
                                {match games {
                                    Ok(g) if g.is_empty() => view! { <p class="muted">"None."</p> }.into_any(),
                                    Ok(g) => view! { <GameList games=g /> }.into_any(),
                                    Err(e) => view! { <ErrorLine message=error_text(&e) /> }.into_any(),
                                }}
                            </Window>
                        </div>
                        <Window title=if hidden_only() { "Hidden reports" } else { "Latest reports" }>
                            <ActionError action=hide />
                            {match reports {
                                Ok(rs) if rs.is_empty() => view! { <p class="muted">"Nothing here."</p> }.into_any(),
                                Ok(rs) => {
                                    let more = rs.len() >= 50;
                                    view! {
                                        <ol class="reports">{rs.into_iter().map(|r| view! { <ModRow item=r hide /> }).collect_view()}</ol>
                                        <p class="pager">
                                            {(page() > 0)
                                                .then(|| {
                                                    let h = format!("/mod?source={}&hidden={}&page={}", urlencode(&source()), u8::from(hidden_only()), page() - 1);
                                                    view! { <A href=h attr:class="btn">"◄ Newer"</A> }
                                                })}
                                            {more
                                                .then(|| {
                                                    let h = format!("/mod?source={}&hidden={}&page={}", urlencode(&source()), u8::from(hidden_only()), page() + 1);
                                                    view! { <A href=h attr:class="btn">"Older ►"</A> }
                                                })}
                                        </p>
                                    }
                                        .into_any()
                                }
                                Err(e) => view! { <ErrorLine message=error_text(&e) /> }.into_any(),
                            }}
                        </Window>
                    }
                        .into_any()
                })
            }}
        </Transition>
    }
}

#[component]
fn ModRow(item: ModReport, hide: ServerAction<SetReportHidden>) -> impl IntoView {
    let r = item.report;
    let game_href = format!("/games/{}#report-{}", item.game_slug, r.id);
    let source = r.source.clone().unwrap_or_default();
    let source_href = format!("/mod?source={source}");
    view! {
        <li class="report" class:is-hidden=r.hidden>
            <div class="report-head">
                <Light rating=Some(r.rating) />
                " "
                <A href=game_href>{item.game_name}</A>
                {r.version_label.clone().map(|l| view! { <span class="muted">{format!(" · {l}")}</span> })}
                {r.blessed.then(|| view! { <span class="badge star">"★"</span> })}
                {r.hidden.then(|| view! { <span class="badge">"hidden"</span> })}
            </div>
            <p class="report-meta">
                {date(r.created_at)}
                " · "
                {r.author.clone().unwrap_or_else(|| "anonymous".into())}
                " · "
                <A href=source_href attr:class="source-link">
                    {format!("src {}", source.chars().take(8).collect::<String>())}
                </A>
            </p>
            {r.description.clone().map(|d| view! { <p class="description">{d}</p> })}
            <SettingChips settings=r.settings.clone() />
            <div class="staff-bar">
                <ActionForm action=hide attr:class="inline">
                    <input type="hidden" name="id" value=r.id.to_string() />
                    <input type="hidden" name="hidden" value=(!r.hidden).to_string() />
                    <button type="submit" class="btn small">
                        {if r.hidden { "Unhide" } else { "Hide" }}
                    </button>
                </ActionForm>
            </div>
        </li>
    }
}

#[component]
pub fn LogPage() -> impl IntoView {
    let query = use_query_map();
    let page = move || query.read().get("page").and_then(|p| p.parse::<u64>().ok()).unwrap_or(0);
    let log = Resource::new(page, mod_log);
    view! {
        <Title text="Moderation log" />
        <Window title="Moderation log">
            <Transition fallback=|| view! { <p class="loading">"Loading…"</p> }>
                {move || Suspend::new(async move {
                    match log.await {
                        Ok(entries) => {
                            let more = entries.len() >= 50;
                            view! {
                                <div class="scroll-x">
                                    <table class="list log">
                                        <thead>
                                            <tr>
                                                <th scope="col">"When"</th>
                                                <th scope="col">"Who"</th>
                                                <th scope="col">"What"</th>
                                                <th scope="col">"Detail"</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {entries
                                                .into_iter()
                                                .map(|e| {
                                                    view! {
                                                        <tr>
                                                            <td>{date(e.at)}</td>
                                                            <td>{e.user}</td>
                                                            <td>{format!("{} {} #{}", e.action, e.target_type, e.target_id)}</td>
                                                            <td>{e.detail.unwrap_or_default()}</td>
                                                        </tr>
                                                    }
                                                })
                                                .collect_view()}
                                        </tbody>
                                    </table>
                                </div>
                                <p class="pager">
                                    {(page() > 0).then(|| view! { <A href=format!("/mod/log?page={}", page() - 1) attr:class="btn">"◄ Newer"</A> })}
                                    {more.then(|| view! { <A href=format!("/mod/log?page={}", page() + 1) attr:class="btn">"Older ►"</A> })}
                                </p>
                            }
                                .into_any()
                        }
                        Err(e) => view! { <ErrorLine message=error_text(&e) /> }.into_any(),
                    }
                })}
            </Transition>
        </Window>
    }
}

#[component]
pub fn UsersPage() -> impl IntoView {
    let create = ServerAction::<CreateUser>::new();
    let update = ServerAction::<UpdateUser>::new();
    let users = Resource::new(move || create.version().get() + update.version().get(), |_| list_users());
    view! {
        <Title text="Users" />
        <div class="row-top">
            <Window title="Staff">
                <ActionError action=update />
                <Transition fallback=|| view! { <p class="loading">"Loading…"</p> }>
                    {move || Suspend::new(async move {
                        match users.await {
                            Ok(list) => view! {
                                <ul class="users">
                                    {list
                                        .into_iter()
                                        .map(|u| {
                                            let other = if u.role == Role::Admin { Role::Moderator } else { Role::Admin };
                                            let id = u.id;
                                            view! {
                                                <li class:is-hidden=u.disabled>
                                                    <span class="name">{u.username.clone()}</span>
                                                    <span class="badge">{u.role.as_str()}</span>
                                                    {u.disabled.then(|| view! { <span class="badge">"disabled"</span> })}
                                                    <span class="muted">{format!(" since {}", date(u.created_at))}</span>
                                                    <div class="staff-bar">
                                                        <button
                                                            class="btn small"
                                                            on:click=move |_| {
                                                                update.dispatch(UpdateUser { id, role: Some(other), disabled: None, password: None });
                                                            }
                                                        >
                                                            {format!("Make {}", other.as_str())}
                                                        </button>
                                                        <button
                                                            class="btn small"
                                                            on:click=move |_| {
                                                                update.dispatch(UpdateUser { id, role: None, disabled: Some(!u.disabled), password: None });
                                                            }
                                                        >
                                                            {if u.disabled { "Enable" } else { "Disable" }}
                                                        </button>
                                                        <details class="edit">
                                                            <summary class="btn small">"Password"</summary>
                                                            <ActionForm action=update attr:class="form">
                                                                <input type="hidden" name="id" value=id.to_string() />
                                                                <Field label="New password" name="password" kind="password" required=true />
                                                                <button type="submit" class="btn is-default">
                                                                    "Set"
                                                                </button>
                                                            </ActionForm>
                                                        </details>
                                                    </div>
                                                </li>
                                            }
                                        })
                                        .collect_view()}
                                </ul>
                            }
                                .into_any(),
                            Err(e) => view! { <ErrorLine message=error_text(&e) /> }.into_any(),
                        }
                    })}
                </Transition>
            </Window>
            <Dialog title="New staff account" class="form-dialog">
                <ActionForm action=create attr:class="form">
                    <Field label="Username" name="username" required=true />
                    <Field label="Password" name="password" kind="password" required=true />
                    <div class="field">
                        <label for="f-role">"Role"</label>
                        <span class="select">
                            <select id="f-role" name="role" class="input">
                                <option value="moderator">"moderator"</option>
                                <option value="admin">"admin"</option>
                            </select>
                        </span>
                    </div>
                    <ActionError action=create />
                    <p class="form-buttons">
                        <button type="submit" class="btn is-default">
                            "Create"
                        </button>
                    </p>
                </ActionForm>
            </Dialog>
        </div>
    }
}
