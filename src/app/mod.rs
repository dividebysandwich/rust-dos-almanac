//! The web UI: a Turbo Vision desktop, as on rust-dos.com.

pub mod components;
pub mod fns;
mod pages;

use crate::model::{Role, UserInfo};
use fns::{Login, Logout, current_user};
use leptos::prelude::*;
use leptos_meta::{Link, Meta, MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::components::{A, Route, Router, Routes};
use leptos_router::path;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <meta name="theme-color" content="#0000aa" />
                <link rel="preload" href="/fonts/WebPlus_IBM_VGA_8x16.woff" r#as="font" r#type="font/woff" crossorigin="" />
                <AutoReload options=options.clone() />
                <HydrationScripts options />
                <MetaTags />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

/// The signed-in staff member, shared with every page.
#[derive(Clone, Copy)]
pub struct Session {
    pub user: Resource<Result<Option<UserInfo>, ServerFnError>>,
    pub login: ServerAction<Login>,
    pub logout: ServerAction<Logout>,
}

impl Session {
    /// The user, once loaded; None while loading or when signed out.
    pub fn get(&self) -> Option<UserInfo> {
        self.user.get().and_then(|r| r.ok()).flatten()
    }

    pub fn is_staff(&self) -> bool {
        self.get().is_some()
    }

    pub fn is_admin(&self) -> bool {
        self.get().is_some_and(|u| u.role == Role::Admin)
    }
}

pub fn use_session() -> Session {
    expect_context::<Session>()
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    let login = ServerAction::<Login>::new();
    let logout = ServerAction::<Logout>::new();
    let user = Resource::new(move || (login.version().get(), logout.version().get()), |_| current_user());
    provide_context(Session { user, login, logout });

    view! {
        <Stylesheet id="leptos" href="/pkg/rust-dos-almanac.css" />
        <Link rel="icon" href="/favicon.svg" type_="image/svg+xml" />
        <Title formatter=|t: String| if t.is_empty() { "Rust-DOS Almanac".to_string() } else { format!("{t} · Rust-DOS Almanac") } />
        <Meta name="description" content="Which Rust-DOS settings make a DOS game run well: community reports, a stoplight rating and recommended configurations." />
        <Router>
            <MenuBar />
            <main class="desktop">
                <div class="deskgrid">
                    <Routes fallback=|| view! { <pages::NotFound /> }>
                        <Route path=path!("/") view=pages::home::HomePage />
                        <Route path=path!("/new") view=pages::forms::NewGamePage />
                        <Route path=path!("/games/:slug") view=pages::game::GamePage />
                        <Route path=path!("/games/:slug/version") view=pages::forms::NewVersionPage />
                        <Route path=path!("/games/:slug/report") view=pages::report::NewReportPage />
                        <Route path=path!("/reports/:id/edit") view=pages::report::EditReportPage />
                        <Route path=path!("/login") view=pages::staff::LoginPage />
                        <Route path=path!("/mod") view=pages::staff::ModerationPage />
                        <Route path=path!("/mod/log") view=pages::staff::LogPage />
                        <Route path=path!("/admin/users") view=pages::staff::UsersPage />
                        <Route path=path!("/about") view=pages::AboutPage />
                    </Routes>
                </div>
            </main>
            <StatusLine />
        </Router>
    }
}

/// A menu title with its hot key in red, as Turbo Vision draws them.
#[component]
fn MenuItem(href: &'static str, hot: &'static str, rest: &'static str, #[prop(optional)] key: &'static str) -> impl IntoView {
    view! {
        <A href=href attr:class="mb-title" attr:accesskey=key>
            <span class="hk">{hot}</span>
            {rest}
        </A>
    }
}

#[component]
fn MenuBar() -> impl IntoView {
    let session = use_session();
    view! {
        <nav class="menubar" aria-label="Menu">
            <div class="mb">
                <A href="/about" attr:class="mb-title" attr:aria-label="About">
                    <span class="hk">"≡"</span>
                </A>
                <MenuItem href="/" hot="G" rest="ames" key="g" />
                <MenuItem href="/new" hot="N" rest="ew game" key="n" />
                <Transition fallback=|| ()>
                    {move || {
                        session
                            .get()
                            .map(|u| {
                                view! {
                                    <MenuItem href="/mod" hot="M" rest="oderate" key="m" />
                                    {(u.role == Role::Admin)
                                        .then(|| view! { <MenuItem href="/admin/users" hot="U" rest="sers" key="u" /> })}
                                }
                            })
                    }}
                </Transition>
            </div>
            <div class="mb mb-right">
                <Transition fallback=|| ()>
                    {move || match session.get() {
                        Some(u) => {
                            view! {
                                <span class="mb-user">{u.username}</span>
                                <ActionForm action=session.logout attr:class="mb-inline">
                                    <button type="submit" class="mb-title">
                                        <span class="hk">"L"</span>
                                        "ogout"
                                    </button>
                                </ActionForm>
                            }
                                .into_any()
                        }
                        None => view! { <MenuItem href="/login" hot="L" rest="ogin" key="l" /> }.into_any(),
                    }}
                </Transition>
            </div>
        </nav>
    }
}

#[component]
fn StatusLine() -> impl IntoView {
    view! {
        <footer class="statusline">
            <span class="sl-items">
                <A href="/about">
                    <span class="hk">"F1"</span>
                    " Help"
                </A>
                <A href="/">
                    <span class="hk">"Alt-G"</span>
                    " Games"
                </A>
                <A href="/new">
                    <span class="hk">"Alt-N"</span>
                    " New"
                </A>
                <a href="/api/v1/schema">
                    <span class="hk">"API"</span>
                    " Schema"
                </a>
            </span>
            <span class="sl-hint">"Game compatibility for Rust-DOS"</span>
        </footer>
    }
}
