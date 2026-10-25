use super::{id32, log_action, ratings_by};
use crate::model::*;
use crate::server::auth::{Actor, now};
use crate::server::entities::{game, game_version, report, version_archive, version_file};
use crate::server::error::{Error, Result};
use crate::server::state::AppState;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait,
};
use std::collections::HashMap;

pub const PAGE_SIZE: u64 = 50;
const MAX_FILES: usize = 500;

fn text(field: &str, value: &str, max: usize) -> Result<String> {
    let v = value.trim();
    if v.is_empty() {
        return Err(Error::invalid(format!("{field} is required")));
    }
    if v.chars().count() > max {
        return Err(Error::invalid(format!("{field} is longer than {max} characters")));
    }
    if v.chars().any(|c| c.is_control()) {
        return Err(Error::invalid(format!("{field} contains control characters")));
    }
    Ok(v.to_string())
}

fn opt_text(field: &str, value: Option<&str>, max: usize) -> Result<Option<String>> {
    match value.map(str::trim).filter(|v| !v.is_empty()) {
        Some(v) => text(field, v, max).map(Some),
        None => Ok(None),
    }
}

fn year(y: Option<i32>) -> Result<Option<i32>> {
    match y {
        Some(y) if !(1970..=2035).contains(&y) => Err(Error::invalid("year must be between 1970 and 2035")),
        y => Ok(y),
    }
}

fn aliases(list: &[String]) -> Result<Vec<String>> {
    let out: Vec<String> = list
        .iter()
        .filter(|a| !a.trim().is_empty())
        .map(|a| text("alias", &a.replace(';', ","), 200))
        .collect::<Result<_>>()?;
    if out.len() > 20 {
        return Err(Error::invalid("at most 20 aliases"));
    }
    Ok(out)
}

fn search_text(name: &str, slug: &str, aliases: &[String]) -> String {
    search_key(&format!("{name} {} {}", slug.replace('-', " "), aliases.join(" ")))
}

fn hex_field(field: &str, value: Option<&str>, len: usize, pad: bool) -> Result<Option<String>> {
    let Some(v) = value.map(str::trim).filter(|v| !v.is_empty()) else { return Ok(None) };
    let v = v.to_ascii_lowercase();
    let v = v.strip_prefix("0x").unwrap_or(&v);
    if !v.bytes().all(|b| b.is_ascii_hexdigit()) || v.len() > len || (!pad && v.len() != len) {
        return Err(Error::invalid(format!("{field} must be {len} hex digits")));
    }
    Ok(Some(format!("{v:0>len$}")))
}

/// A file as stored: lowercase name without a directory, checked hashes.
pub fn normalize_file(f: &FileInfo) -> Result<FileInfo> {
    let base = f.name.trim().rsplit(['/', '\\']).next().unwrap_or("");
    let name = text("file name", base, 255)?.to_lowercase();
    if f.size.is_some_and(|s| s < 0) {
        return Err(Error::invalid("file size can't be negative"));
    }
    Ok(FileInfo {
        name,
        size: f.size,
        crc32: hex_field("crc32", f.crc32.as_deref(), 8, true)?,
        md5: hex_field("md5", f.md5.as_deref(), 32, false)?,
        sha1: hex_field("sha1", f.sha1.as_deref(), 40, false)?,
    })
}

pub fn normalize_ra(v: &str) -> Result<String> {
    hex_field("archive hash", Some(v), 32, false)?.ok_or_else(|| Error::invalid("archive hash is empty"))
}

fn summary(g: &game::Model, rating: RatingSummary) -> GameSummary {
    GameSummary {
        id: g.id.into(),
        slug: g.slug.clone(),
        name: g.name.clone(),
        year: g.year,
        publisher: g.publisher.clone(),
        hidden: g.hidden,
        rating,
        updated_at: g.updated_at,
    }
}

async fn summaries(db: &DatabaseConnection, games: Vec<game::Model>) -> Result<Vec<GameSummary>> {
    let ids: Vec<i32> = games.iter().map(|g| g.id).collect();
    let mut ratings = ratings_by(db, report::Column::GameId.is_in(ids), |r| Some(r.game_id)).await?;
    Ok(games.iter().map(|g| summary(g, ratings.remove(&g.id).unwrap_or_default())).collect())
}

/// Games whose name, slug or aliases contain every word of `q`; the most
/// recently changed ones when `q` is empty.
pub async fn search(db: &DatabaseConnection, actor: &Actor, q: &str, page: u64) -> Result<Vec<GameSummary>> {
    let key = search_key(q);
    let mut query = game::Entity::find();
    if !actor.is_staff() {
        query = query.filter(game::Column::Hidden.eq(false));
    }
    for word in key.split(' ').filter(|w| !w.is_empty()).take(8) {
        query = query.filter(game::Column::SearchText.contains(word));
    }
    query = if key.is_empty() { query.order_by_desc(game::Column::UpdatedAt) } else { query.order_by_asc(game::Column::Name) };
    let games = query.limit(PAGE_SIZE).offset(page.min(10_000) * PAGE_SIZE).all(db).await?;
    summaries(db, games).await
}

pub async fn hidden_games(db: &DatabaseConnection, actor: &Actor) -> Result<Vec<GameSummary>> {
    actor.staff()?;
    let games = game::Entity::find().filter(game::Column::Hidden.eq(true)).order_by_desc(game::Column::UpdatedAt).all(db).await?;
    summaries(db, games).await
}

/// A game by slug, if the actor may see it.
pub async fn find_visible(db: &DatabaseConnection, actor: &Actor, slug: &str) -> Result<game::Model> {
    let g = game::Entity::find().filter(game::Column::Slug.eq(slug)).one(db).await?.ok_or(Error::NotFound)?;
    if g.hidden && !actor.is_staff() {
        return Err(Error::NotFound);
    }
    Ok(g)
}

pub async fn find_version(db: &DatabaseConnection, actor: &Actor, game_id: i32, version_id: i64) -> Result<game_version::Model> {
    let v = game_version::Entity::find_by_id(id32(version_id)?).one(db).await?.ok_or(Error::NotFound)?;
    if v.game_id != game_id || (v.hidden && !actor.is_staff()) {
        return Err(Error::NotFound);
    }
    Ok(v)
}

pub async fn detail(db: &DatabaseConnection, actor: &Actor, g: game::Model) -> Result<GameDetail> {
    let mut vq = game_version::Entity::find().filter(game_version::Column::GameId.eq(g.id));
    if !actor.is_staff() {
        vq = vq.filter(game_version::Column::Hidden.eq(false));
    }
    let versions = vq.order_by_asc(game_version::Column::Id).all(db).await?;
    let vids: Vec<i32> = versions.iter().map(|v| v.id).collect();
    let mut files: HashMap<i32, Vec<FileInfo>> = HashMap::new();
    let mut archives: HashMap<i32, Vec<String>> = HashMap::new();
    if !vids.is_empty() {
        let rows = version_file::Entity::find()
            .filter(version_file::Column::VersionId.is_in(vids.clone()))
            .order_by_asc(version_file::Column::Filename)
            .all(db)
            .await?;
        for f in rows {
            files.entry(f.version_id).or_default().push(FileInfo { name: f.filename, size: f.size, crc32: f.crc32, md5: f.md5, sha1: f.sha1 });
        }
        for a in version_archive::Entity::find().filter(version_archive::Column::VersionId.is_in(vids)).all(db).await? {
            archives.entry(a.version_id).or_default().push(a.ra_md5);
        }
    }
    let game_rating = ratings_by(db, report::Column::GameId.eq(g.id), |_| Some(g.id)).await?.remove(&g.id).unwrap_or_default();
    let mut by_version = ratings_by(db, report::Column::GameId.eq(g.id), |r| r.version_id).await?;
    Ok(GameDetail {
        id: g.id.into(),
        slug: g.slug,
        name: g.name,
        aliases: g.aliases.split(';').filter(|a| !a.is_empty()).map(String::from).collect(),
        year: g.year,
        publisher: g.publisher,
        hidden: g.hidden,
        versions: versions
            .into_iter()
            .map(|v| VersionInfo {
                id: v.id.into(),
                rating: by_version.remove(&v.id).unwrap_or_default(),
                files: files.remove(&v.id).unwrap_or_default(),
                ra_md5: archives.remove(&v.id).unwrap_or_default(),
                label: v.label,
                hidden: v.hidden,
            })
            .collect(),
        rating: game_rating,
        created_at: g.created_at,
        updated_at: g.updated_at,
    })
}

pub async fn get(db: &DatabaseConnection, actor: &Actor, slug: &str) -> Result<GameDetail> {
    let g = find_visible(db, actor, slug).await?;
    detail(db, actor, g).await
}

async fn insert_version<C: sea_orm::ConnectionTrait>(db: &C, actor: &Actor, game_id: i32, v: &NewVersion) -> Result<game_version::Model> {
    let label = text("version label", &v.label, 100)?;
    if v.files.len() > MAX_FILES {
        return Err(Error::invalid(format!("at most {MAX_FILES} files per version")));
    }
    let files: Vec<FileInfo> = v.files.iter().map(normalize_file).collect::<Result<_>>()?;
    if files.iter().any(|f| f.crc32.is_none() && f.md5.is_none() && f.sha1.is_none()) {
        return Err(Error::invalid("every file needs a crc32, md5 or sha1"));
    }
    let ra = v.ra_md5.as_deref().filter(|s| !s.trim().is_empty()).map(normalize_ra).transpose()?;
    if let Some(ra) = &ra
        && version_archive::Entity::find().filter(version_archive::Column::RaMd5.eq(ra.as_str())).one(db).await?.is_some()
    {
        return Err(Error::Conflict("that archive hash already belongs to a version".into()));
    }
    let version = game_version::ActiveModel {
        game_id: Set(game_id),
        label: Set(label),
        hidden: Set(false),
        source_hash: Set((!actor.is_staff()).then(|| actor.source.clone())),
        created_at: Set(now()),
        ..Default::default()
    }
    .insert(db)
    .await?;
    for f in files {
        version_file::ActiveModel {
            version_id: Set(version.id),
            filename: Set(f.name),
            size: Set(f.size),
            crc32: Set(f.crc32),
            md5: Set(f.md5),
            sha1: Set(f.sha1),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }
    if let Some(ra) = ra {
        version_archive::ActiveModel { version_id: Set(version.id), ra_md5: Set(ra), ..Default::default() }.insert(db).await?;
    }
    Ok(version)
}

pub async fn create(state: &AppState, actor: &Actor, input: NewGame) -> Result<GameDetail> {
    let name = text("name", &input.name, 200)?;
    let slug = slugify(&name);
    if slug.is_empty() {
        return Err(Error::invalid("the name needs at least one letter or digit"));
    }
    let publisher = opt_text("publisher", input.publisher.as_deref(), 200)?;
    let year = year(input.year)?;
    let aliases = aliases(&input.aliases)?;
    let db = &state.db;
    if game::Entity::find().filter(game::Column::Slug.eq(slug.as_str())).one(db).await?.is_some() {
        return Err(Error::Conflict(format!("the game already exists as '{slug}'; add a version to it instead")));
    }
    actor.spend_write(state)?;

    let txn = db.begin().await?;
    let t = now();
    let g = game::ActiveModel {
        search_text: Set(search_text(&name, &slug, &aliases)),
        name: Set(name),
        slug: Set(slug),
        aliases: Set(aliases.join(";")),
        year: Set(year),
        publisher: Set(publisher),
        hidden: Set(false),
        source_hash: Set((!actor.is_staff()).then(|| actor.source.clone())),
        created_at: Set(t),
        updated_at: Set(t),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    if let Some(v) = &input.version {
        insert_version(&txn, actor, g.id, v).await?;
    }
    txn.commit().await?;
    if let Some(uid) = actor.user_id() {
        log_action(db, uid, "create", "game", g.id, None).await?;
    }
    detail(db, actor, g).await
}

async fn touch(db: &impl sea_orm::ConnectionTrait, game_id: i32) -> Result<()> {
    game::Entity::update_many()
        .col_expr(game::Column::UpdatedAt, sea_orm::sea_query::Expr::value(now()))
        .filter(game::Column::Id.eq(game_id))
        .exec(db)
        .await?;
    Ok(())
}

pub async fn add_version(state: &AppState, actor: &Actor, slug: &str, input: NewVersion) -> Result<VersionInfo> {
    let db = &state.db;
    let g = find_visible(db, actor, slug).await?;
    // Validate before spending the budget, so typos don't cost a write.
    text("version label", &input.label, 100)?;
    input.files.iter().map(normalize_file).collect::<Result<Vec<_>>>()?;
    actor.spend_write(state)?;
    let txn = db.begin().await?;
    let v = insert_version(&txn, actor, g.id, &input).await?;
    touch(&txn, g.id).await?;
    txn.commit().await?;
    if let Some(uid) = actor.user_id() {
        log_action(db, uid, "create", "version", v.id, None).await?;
    }
    let detail = detail(db, actor, g).await?;
    detail.versions.into_iter().find(|x| x.id == i64::from(v.id)).ok_or(Error::NotFound)
}

pub async fn update(state: &AppState, actor: &Actor, slug: &str, patch: GamePatch) -> Result<GameDetail> {
    let staff = actor.staff()?.clone();
    let db = &state.db;
    let g = find_visible(db, actor, slug).await?;
    let mut changes = Vec::new();
    let mut m = g.clone().into_active_model();
    let mut name = g.name.clone();
    let mut alias_list: Vec<String> = g.aliases.split(';').filter(|a| !a.is_empty()).map(String::from).collect();
    if let Some(n) = &patch.name {
        name = text("name", n, 200)?;
        m.name = Set(name.clone());
        changes.push("name");
    }
    if let Some(y) = patch.year {
        m.year = Set(year(y)?);
        changes.push("year");
    }
    if let Some(p) = &patch.publisher {
        m.publisher = Set(opt_text("publisher", p.as_deref(), 200)?);
        changes.push("publisher");
    }
    if let Some(a) = &patch.aliases {
        alias_list = aliases(a)?;
        m.aliases = Set(alias_list.join(";"));
        changes.push("aliases");
    }
    if let Some(h) = patch.hidden {
        m.hidden = Set(h);
        changes.push(if h { "hidden" } else { "shown" });
    }
    m.search_text = Set(search_text(&name, &g.slug, &alias_list));
    m.updated_at = Set(now());
    let g = m.update(db).await?;
    let action = match patch.hidden {
        Some(true) => "hide",
        Some(false) => "unhide",
        None => "edit",
    };
    log_action(db, id32(staff.id)?, action, "game", g.id, Some(changes.join(", "))).await?;
    detail(db, actor, g).await
}

pub async fn update_version(state: &AppState, actor: &Actor, version_id: i64, patch: VersionPatch) -> Result<VersionInfo> {
    let staff = actor.staff()?.clone();
    let db = &state.db;
    let v = game_version::Entity::find_by_id(id32(version_id)?).one(db).await?.ok_or(Error::NotFound)?;
    let g = game::Entity::find_by_id(v.game_id).one(db).await?.ok_or(Error::NotFound)?;
    let txn = db.begin().await?;
    let mut m = v.clone().into_active_model();
    let mut changes = Vec::new();
    if let Some(l) = &patch.label {
        m.label = Set(text("version label", l, 100)?);
        changes.push("label");
    }
    if let Some(h) = patch.hidden {
        m.hidden = Set(h);
        changes.push(if h { "hidden" } else { "shown" });
    }
    m.update(&txn).await?;
    if let Some(files) = &patch.files {
        let files: Vec<FileInfo> = files.iter().map(normalize_file).collect::<Result<_>>()?;
        version_file::Entity::delete_many().filter(version_file::Column::VersionId.eq(v.id)).exec(&txn).await?;
        for f in files {
            version_file::ActiveModel {
                version_id: Set(v.id),
                filename: Set(f.name),
                size: Set(f.size),
                crc32: Set(f.crc32),
                md5: Set(f.md5),
                sha1: Set(f.sha1),
                ..Default::default()
            }
            .insert(&txn)
            .await?;
        }
        changes.push("files");
    }
    if let Some(list) = &patch.ra_md5 {
        let list: Vec<String> = list.iter().filter(|s| !s.trim().is_empty()).map(|s| normalize_ra(s)).collect::<Result<_>>()?;
        version_archive::Entity::delete_many().filter(version_archive::Column::VersionId.eq(v.id)).exec(&txn).await?;
        for ra in list {
            if version_archive::Entity::find().filter(version_archive::Column::RaMd5.eq(ra.as_str())).one(&txn).await?.is_some() {
                return Err(Error::Conflict(format!("archive hash {ra} already belongs to another version")));
            }
            version_archive::ActiveModel { version_id: Set(v.id), ra_md5: Set(ra), ..Default::default() }.insert(&txn).await?;
        }
        changes.push("archive hashes");
    }
    touch(&txn, g.id).await?;
    txn.commit().await?;
    let action = match patch.hidden {
        Some(true) => "hide",
        Some(false) => "unhide",
        None => "edit",
    };
    log_action(db, id32(staff.id)?, action, "version", v.id, Some(changes.join(", "))).await?;
    let detail = detail(db, actor, g).await?;
    detail.versions.into_iter().find(|x| x.id == version_id).ok_or(Error::NotFound)
}

/// Versions whose files or archive hash match what a client has on disk,
/// best first.
pub async fn lookup(db: &DatabaseConnection, actor: &Actor, req: LookupRequest) -> Result<Vec<LookupMatch>> {
    if req.files.len() > 2000 {
        return Err(Error::invalid("at most 2000 files per lookup"));
    }
    let files: Vec<FileInfo> = req.files.iter().map(normalize_file).collect::<Result<_>>()?;
    let ra = req.ra_md5.as_deref().filter(|s| !s.trim().is_empty()).map(normalize_ra).transpose()?;

    let mut cond = Condition::any();
    let names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
    let list = |pick: fn(&FileInfo) -> Option<&String>| files.iter().filter_map(pick).cloned().collect::<Vec<_>>();
    let (md5s, sha1s, crcs) = (list(|f| f.md5.as_ref()), list(|f| f.sha1.as_ref()), list(|f| f.crc32.as_ref()));
    let any_files = !names.is_empty();
    if any_files {
        cond = cond.add(version_file::Column::Filename.is_in(names));
    }
    if !md5s.is_empty() {
        cond = cond.add(version_file::Column::Md5.is_in(md5s));
    }
    if !sha1s.is_empty() {
        cond = cond.add(version_file::Column::Sha1.is_in(sha1s));
    }
    if !crcs.is_empty() {
        cond = cond.add(version_file::Column::Crc32.is_in(crcs));
    }
    let rows = if any_files { version_file::Entity::find().filter(cond).limit(20_000).all(db).await? } else { Vec::new() };

    #[derive(Default)]
    struct Score {
        hash: u32,
        name: u32,
        ra: bool,
    }
    let mut by_version: HashMap<i32, Vec<&version_file::Model>> = HashMap::new();
    for r in &rows {
        by_version.entry(r.version_id).or_default().push(r);
    }
    let mut scores: HashMap<i32, Score> = HashMap::new();
    for (vid, stored) in &by_version {
        let s = scores.entry(*vid).or_default();
        for f in &files {
            let mut best = 0; // 2 = hash match, 1 = name match without contradiction
            for st in stored {
                let pairs = [(&f.md5, &st.md5), (&f.sha1, &st.sha1), (&f.crc32, &st.crc32)];
                let equal = pairs.iter().any(|(a, b)| a.is_some() && a == b);
                let conflict = pairs.iter().any(|(a, b)| a.is_some() && b.is_some() && a != b);
                let size_conflict = f.size.is_some() && st.size.is_some() && f.size != st.size;
                if equal && !conflict {
                    best = 2;
                    break;
                }
                if f.name == st.filename && !conflict && !size_conflict {
                    best = best.max(1);
                }
            }
            match best {
                2 => s.hash += 1,
                1 => s.name += 1,
                _ => {}
            }
        }
    }
    if let Some(ra) = &ra {
        for a in version_archive::Entity::find().filter(version_archive::Column::RaMd5.eq(ra.as_str())).all(db).await? {
            scores.entry(a.version_id).or_default().ra = true;
        }
    }
    scores.retain(|_, s| s.ra || s.hash > 0 || s.name > 0);
    if scores.is_empty() {
        return Ok(Vec::new());
    }

    let versions = game_version::Entity::find().filter(game_version::Column::Id.is_in(scores.keys().copied().collect::<Vec<_>>())).all(db).await?;
    let game_ids: Vec<i32> = versions.iter().map(|v| v.game_id).collect();
    let games: HashMap<i32, game::Model> =
        game::Entity::find().filter(game::Column::Id.is_in(game_ids)).all(db).await?.into_iter().map(|g| (g.id, g)).collect();
    let mut out: Vec<LookupMatch> = versions
        .into_iter()
        .filter_map(|v| {
            let g = games.get(&v.game_id)?;
            if (g.hidden || v.hidden) && !actor.is_staff() {
                return None;
            }
            let s = &scores[&v.id];
            Some(LookupMatch {
                game: g.slug.clone(),
                name: g.name.clone(),
                version_id: v.id.into(),
                version_label: v.label,
                score: if s.ra { 100 } else { 0 } + s.hash * 3 + s.name,
                ra_match: s.ra,
                hash_matches: s.hash,
                name_matches: s.name,
            })
        })
        .collect();
    out.sort_by(|a, b| b.score.cmp(&a.score).then(a.version_id.cmp(&b.version_id)));
    out.truncate(10);
    Ok(out)
}
