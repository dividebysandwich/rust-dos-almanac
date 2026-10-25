use super::game::Field;
use crate::app::components::{ActionError, Dialog, Window};
use crate::app::fns::{AddVersion, CreateGame};
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;

/// The textarea for a version's files, with how to fill it in.
#[component]
pub fn FilesField(#[prop(optional, into)] value: String) -> impl IntoView {
    view! {
        <div class="field wide">
            <label for="f-files">"Files"</label>
            <textarea id="f-files" name="files" class="input" rows="6" spellcheck="false" placeholder="KEEN4E.EXE 263046 1a2b3c4d">
                {value}
            </textarea>
            <p class="hint">"One file per line: NAME [SIZE] [CRC32] [MD5] [SHA1]. Hashes are recognised by length; write size= before an 8-digit size."</p>
        </div>
    }
}

#[component]
pub fn NewGamePage() -> impl IntoView {
    let create = ServerAction::<CreateGame>::new();
    view! {
        <Title text="New game" />
        <div class="row-form">
            <Dialog title="New game" class="form-dialog">
                <ActionForm action=create attr:class="form">
                    <fieldset>
                        <legend>"Game"</legend>
                        <Field label="Name" name="name" required=true placeholder="Commander Keen 4" />
                        <Field label="Year" name="year" kind="number" placeholder="1991" />
                        <Field label="Publisher" name="publisher" placeholder="Apogee" />
                        <Field label="Aliases" name="aliases" placeholder="Other names, comma-separated" />
                    </fieldset>
                    <fieldset>
                        <legend>"First version (optional)"</legend>
                        <Field label="Label" name="label" placeholder="1.4 shareware" />
                        <FilesField />
                        <Field label="Archive hash" name="ra_md5" placeholder="RetroAchievements MD5 from Rust-DOS" />
                    </fieldset>
                    <ActionError action=create />
                    <p class="form-buttons">
                        <button type="submit" class="btn is-default">
                            "Create"
                        </button>
                        <A href="/" attr:class="btn">
                            "Cancel"
                        </A>
                    </p>
                </ActionForm>
            </Dialog>
            <FormHelp />
        </div>
    }
}

#[component]
fn FormHelp() -> impl IntoView {
    view! {
        <Window title="Help" class="help">
            <p>"Search first, so the game isn't added twice. A different release (CD-ROM, a patch, a GOG copy) is a new version of the same game."</p>
            <p>"Checksums let Rust-DOS recognise the game on disk. Any of CRC32, MD5 or SHA-1 will do. List a few characteristic files, such as the main executable and its data files."</p>
        </Window>
    }
}

#[component]
pub fn NewVersionPage() -> impl IntoView {
    let params = use_params_map();
    let slug = move || params.read().get("slug").unwrap_or_default();
    let add = ServerAction::<AddVersion>::new();
    view! {
        <Title text="New version" />
        <div class="row-form">
            <Dialog title="New version" class="form-dialog">
                <ActionForm action=add attr:class="form">
                    <input type="hidden" name="slug" value=slug />
                    <Field label="Label" name="label" required=true placeholder="1.1 CD-ROM" />
                    <FilesField />
                    <Field label="Archive hash" name="ra_md5" placeholder="RetroAchievements MD5 from Rust-DOS" />
                    <ActionError action=add />
                    <p class="form-buttons">
                        <button type="submit" class="btn is-default">
                            "Add version"
                        </button>
                        <A href=move || format!("/games/{}", slug()) attr:class="btn">
                            "Cancel"
                        </A>
                    </p>
                </ActionForm>
            </Dialog>
            <FormHelp />
        </div>
    }
}
