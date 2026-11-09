//! The JSON API end to end, against an in-memory SQLite database.

use axum::Router;
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use leptos::config::LeptosOptions;
use rust_dos_almanac::model::Role;
use rust_dos_almanac::server::{self, config::Config, service::users};
use serde_json::{Value, json};
use std::net::SocketAddr;
use tower::ServiceExt;

struct Client {
    app: Router,
}

impl Client {
    async fn new() -> (Self, rust_dos_almanac::server::state::AppState) {
        let leptos = LeptosOptions::builder().output_name("rust-dos-almanac").build();
        let state = server::build_state(&Config::for_tests(), leptos).await.unwrap();
        (Client { app: server::router(state.clone()) }, state)
    }

    async fn call(&self, method: Method, uri: &str, body: Option<Value>, token: Option<&str>, peer: &str, cf_ip: Option<&str>) -> (StatusCode, Value) {
        let mut req = Request::builder().method(method).uri(uri);
        if body.is_some() {
            req = req.header("content-type", "application/json");
        }
        if let Some(t) = token {
            req = req.header("authorization", format!("Bearer {t}"));
        }
        if let Some(ip) = cf_ip {
            req = req.header("cf-connecting-ip", ip);
        }
        let mut req = req.body(body.map_or_else(Body::empty, |b| Body::from(b.to_string()))).unwrap();
        let addr: SocketAddr = format!("{peer}:443").parse().unwrap();
        req.extensions_mut().insert(ConnectInfo(addr));
        let res = self.app.clone().oneshot(req).await.unwrap();
        let status = res.status();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        let value = serde_json::from_slice(&bytes).unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into()));
        (status, value)
    }

    async fn get(&self, uri: &str) -> (StatusCode, Value) {
        self.call(Method::GET, uri, None, None, "198.51.100.1", None).await
    }

    async fn post(&self, uri: &str, body: Value, peer: &str) -> (StatusCode, Value) {
        self.call(Method::POST, uri, Some(body), None, peer, None).await
    }
}

fn setting<'a>(config: &'a Value, key: &str) -> &'a Value {
    config["settings"].as_array().unwrap().iter().find(|s| s["key"] == key).unwrap_or_else(|| panic!("no {key} in {config}"))
}

#[tokio::test]
async fn full_flow() {
    let (c, state) = Client::new().await;
    users::create_direct(&state.db, "admin", "correct horse battery", Role::Admin).await.unwrap();

    // Staff log in for a bearer token; a wrong password doesn't.
    let (s, _) = c.post("/api/v1/auth/login", json!({"username": "admin", "password": "nope nope nope"}), "198.51.100.9").await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    let (s, login) = c.post("/api/v1/auth/login", json!({"username": "Admin", "password": "correct horse battery"}), "198.51.100.9").await;
    assert_eq!(s, StatusCode::OK, "{login}");
    let token = login["token"].as_str().unwrap().to_string();

    // Anyone can add a game with a first version.
    let (s, game) = c
        .post(
            "/api/v1/games",
            json!({
                "name": "Commander Keen 4: Secret of the Oracle",
                "year": 1991,
                "publisher": "Apogee",
                "aliases": ["Keen 4"],
                "version": {
                    "label": "1.4 shareware",
                    "files": [{"name": "C:\\KEEN4\\KEEN4E.EXE", "size": 263046, "crc32": "0x1A2B3C4D"}],
                    "ra_md5": "0123456789ABCDEF0123456789abcdef"
                }
            }),
            "203.0.113.1",
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{game}");
    let slug = game["slug"].as_str().unwrap().to_string();
    assert_eq!(slug, "commander-keen-4-secret-of-the-oracle");
    let version = game["versions"][0]["id"].as_i64().unwrap();
    assert_eq!(game["versions"][0]["files"][0]["name"], "keen4e.exe");

    let (s, _) = c.post("/api/v1/games", json!({"name": "Commander Keen 4 - Secret of the Oracle"}), "203.0.113.1").await;
    assert_eq!(s, StatusCode::CONFLICT, "the same slug is the same game");

    // Search by alias.
    let (_, found) = c.get("/api/v1/games?q=keen%204").await;
    assert_eq!(found[0]["slug"], slug);

    // Look the game up by what's on disk.
    let (_, m) = c.post("/api/v1/games/lookup", json!({"files": [{"name": "RENAMED.EXE", "crc32": "1a2b3c4d"}]}), "203.0.113.50").await;
    assert_eq!(m[0]["version_id"], version);
    assert_eq!(m[0]["hash_matches"], 1);
    let (_, m) = c.post("/api/v1/games/lookup", json!({"ra_md5": "0123456789abcdef0123456789abcdef"}), "203.0.113.50").await;
    assert_eq!(m[0]["ra_match"], true);
    let (_, m) = c.post("/api/v1/games/lookup", json!({"files": [{"name": "keen4e.exe", "crc32": "ffffffff"}]}), "203.0.113.50").await;
    assert_eq!(m, json!([]), "a name with a different checksum is a different file");

    // Three players, one of them an outlier; a red report doesn't vote.
    let report = |mem: &str, rating: &str| {
        json!({
            "game": slug, "version_id": version, "rating": rating,
            "settings": {"emulator": {"memsize": mem, "cpu": "486"}, "sound": {"sbtype": "SB16"}},
            "rust_dos_version": "v1.7.5",
            "description": "anonymous text is dropped"
        })
    };
    let mut ids = Vec::new();
    for (i, (mem, rating)) in [("128", "green"), ("128MB", "green"), ("256", "green"), ("512", "red")].iter().enumerate() {
        let (s, r) = c.post("/api/v1/reports", report(mem, rating), &format!("203.0.113.{}", 10 + i)).await;
        assert_eq!(s, StatusCode::OK, "{r}");
        assert_eq!(r["description"], Value::Null);
        ids.push(r["id"].as_i64().unwrap());
    }

    let uri = format!("/api/v1/games/{slug}/config?version={version}");
    let (_, config) = c.get(&uri).await;
    assert_eq!(config["source"]["kind"], "consensus");
    assert_eq!(config["scope"], "version");
    let mem = setting(&config, "memsize");
    assert_eq!((mem["value"].as_str(), mem["agree"].as_u64(), mem["voters"].as_u64()), (Some("128"), Some(2), Some(3)));
    assert_eq!(setting(&config, "sbtype")["value"], "sb16");
    assert_eq!(config["rating"]["overall"], "green");

    let (_, ini) = c.get(&format!("/api/v1/games/{slug}/config?format=ini")).await;
    assert_eq!(ini, Value::String("[emulator]\ncpu=486\nmemsize=128\n\n[sound]\nsbtype=sb16\n".into()));

    // Bad input is refused.
    let mut bad = report("128", "green");
    bad["settings"]["drives"] = json!({"c": "/etc"});
    let (s, _) = c.post("/api/v1/reports", bad, "203.0.113.30").await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);

    // Staff only.
    let hide = json!({"hidden": true});
    let (s, _) = c.call(Method::PATCH, &format!("/api/v1/reports/{}", ids[0]), Some(hide.clone()), None, "203.0.113.10", None).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let (s, _) = c.call(Method::PATCH, &format!("/api/v1/reports/{}", ids[0]), Some(hide), Some(&token), "198.51.100.9", None).await;
    assert_eq!(s, StatusCode::OK);
    let (s, _) = c.get(&format!("/api/v1/reports/{}", ids[0])).await;
    assert_eq!(s, StatusCode::NOT_FOUND, "hidden from the public");
    let (_, config) = c.get(&format!("/api/v1/games/{slug}/config")).await;
    assert_eq!(setting(&config, "memsize")["voters"], 2);

    // A blessing overrides the consensus, for its version.
    let (s, _) = c.call(Method::POST, &format!("/api/v1/reports/{}/bless", ids[2]), None, Some(&token), "198.51.100.9", None).await;
    assert_eq!(s, StatusCode::OK);
    let (_, config) = c.get(&uri).await;
    assert_eq!(config["source"]["kind"], "blessed");
    assert_eq!(config["source"]["blessed_by"], "admin");
    assert_eq!(setting(&config, "memsize")["value"], "256");
    let (_, config) = c.get(&format!("/api/v1/games/{slug}/config")).await;
    assert_eq!(config["source"]["kind"], "consensus", "a version's blessing isn't the game-wide one");

    // Staff can write a description.
    let (_, r) = c
        .call(Method::PATCH, &format!("/api/v1/reports/{}", ids[2]), Some(json!({"description": "Needs EMS for music."})), Some(&token), "198.51.100.9", None)
        .await;
    assert_eq!(r["description"], "Needs EMS for music.");

    // Hidden games are gone for the public, not for staff.
    let (s, _) = c.call(Method::PATCH, &format!("/api/v1/games/{slug}"), Some(json!({"hidden": true})), Some(&token), "198.51.100.9", None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(c.get(&format!("/api/v1/games/{slug}")).await.0, StatusCode::NOT_FOUND);
    let (s, _) = c.call(Method::GET, &format!("/api/v1/games/{slug}"), None, Some(&token), "198.51.100.9", None).await;
    assert_eq!(s, StatusCode::OK);

    let (_, log) = c.call(Method::GET, "/api/v1/moderation/log", None, Some(&token), "198.51.100.9", None).await;
    assert!(log.as_array().unwrap().len() >= 4, "{log}");
}

#[tokio::test]
async fn anonymous_writes_are_rate_limited_per_client() {
    let (c, _) = Client::new().await;
    let cf = "162.158.0.10"; // a Cloudflare edge address
    for i in 0..5 {
        let (s, r) = c.call(Method::POST, "/api/v1/games", Some(json!({"name": format!("Game {i}")})), None, cf, Some("203.0.113.77")).await;
        assert_eq!(s, StatusCode::OK, "{r}");
    }
    let (s, _) = c.call(Method::POST, "/api/v1/games", Some(json!({"name": "Game 5"})), None, cf, Some("203.0.113.77")).await;
    assert_eq!(s, StatusCode::TOO_MANY_REQUESTS);

    // Another visitor behind the same Cloudflare edge isn't affected.
    let (s, _) = c.call(Method::POST, "/api/v1/games", Some(json!({"name": "Game 6"})), None, cf, Some("203.0.113.78")).await;
    assert_eq!(s, StatusCode::OK);

    // Going around Cloudflare with a made-up header doesn't reset the budget.
    let direct = "198.51.100.200";
    for i in 0..5 {
        let fake = format!("192.0.2.{i}");
        c.call(Method::POST, "/api/v1/games", Some(json!({"name": format!("Direct {i}")})), None, direct, Some(&fake)).await;
    }
    let (s, _) = c.call(Method::POST, "/api/v1/games", Some(json!({"name": "Direct 5"})), None, direct, Some("192.0.2.99")).await;
    assert_eq!(s, StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn windows_flow() {
    let (c, state) = Client::new().await;
    users::create_direct(&state.db, "admin", "correct horse battery", Role::Admin).await.unwrap();
    let (_, login) = c.post("/api/v1/auth/login", json!({"username": "admin", "password": "correct horse battery"}), "198.51.100.9").await;
    let token = login["token"].as_str().unwrap().to_string();

    // A Windows 95 game, its version marked as such.
    let (s, game) = c
        .post(
            "/api/v1/games",
            json!({
                "name": "Moto Racer", "year": 1997,
                "version": {"label": "1.0", "platform": "win32", "files": [{"name": "MOTORACE.EXE", "size": 1404928, "crc32": "9f3e0c21"}]}
            }),
            "203.0.113.1",
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{game}");
    let slug = game["slug"].as_str().unwrap().to_string();
    let version = game["versions"][0]["id"].as_i64().unwrap();
    assert_eq!(game["versions"][0]["platform"], "win32");
    assert_eq!(game["platforms"], json!(["win32"]));

    // A DOS game, from an old client that sends no platform, then one that does.
    let (s, keen) = c.post("/api/v1/games", json!({"name": "Keen", "version": {"label": "1.4", "files": [{"name": "KEEN4E.EXE", "crc32": "1a2b3c4d"}]}}), "203.0.113.2").await;
    assert_eq!(s, StatusCode::OK, "{keen}");
    assert_eq!(keen["versions"][0]["platform"], Value::Null);
    assert_eq!(keen["platforms"], json!([]));
    let (s, v) = c.post("/api/v1/games/keen/versions", json!({"label": "1.4 CD", "platform": "dos", "files": [{"name": "KEEN4C.EXE", "crc32": "2a2b3c4d"}]}), "203.0.113.3").await;
    assert_eq!(s, StatusCode::OK, "{v}");
    assert_eq!(v["platform"], "dos");

    // With no reports, a Windows version still gets the built-in Windows.
    let (_, config) = c.get(&format!("/api/v1/games/{slug}/config?version={version}")).await;
    assert_eq!(config["source"]["kind"], "none");
    let w = setting(&config, "win32");
    assert_eq!((w["value"].as_str(), w["implied"].as_bool()), (Some("true"), Some(true)));
    assert_eq!(config["ini"], "[emulator]\nwin32=true\n");
    let (_, config) = c.get(&format!("/api/v1/games/{slug}/config")).await;
    assert_eq!(setting(&config, "win32")["value"], "true", "every version is a Windows one");
    let (_, config) = c.get("/api/v1/games/keen/config").await;
    assert_eq!(config["settings"], json!([]), "DOS games get nothing implied");

    // Windows settings in a report.
    let report = |depth: &str| json!({"game": slug, "version_id": version, "rating": "green",
        "settings": {"emulator": {"win32": "on", "win32_version": "98SE", "win32_depth": depth, "cpu": "pentium_ii"}}});
    let (s, r) = c.post("/api/v1/reports", report("24"), "203.0.113.20").await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY, "{r}");
    let (s, r) = c.post("/api/v1/reports", report("16"), "203.0.113.21").await;
    assert_eq!(s, StatusCode::OK, "{r}");
    let (_, config) = c.get(&format!("/api/v1/games/{slug}/config?version={version}")).await;
    assert_eq!(config["source"]["kind"], "consensus");
    assert_eq!(setting(&config, "win32")["implied"], Value::Null, "reported, not implied");
    assert_eq!(setting(&config, "win32_depth")["value"], "16");
    assert_eq!(config["ini"], "[emulator]\ncpu=pentium_ii\nwin32=true\nwin32_version=98se\nwin32_depth=16\n");

    // Searching by platform.
    let names = |v: &Value| v.as_array().unwrap().iter().map(|g| g["slug"].as_str().unwrap().to_string()).collect::<Vec<_>>();
    let (_, found) = c.get("/api/v1/games?platform=windows").await;
    assert_eq!(names(&found), std::slice::from_ref(&slug));
    assert_eq!(found[0]["platforms"], json!(["win32"]));
    let (_, found) = c.get("/api/v1/games?platform=dos").await;
    assert_eq!(names(&found), ["keen"]);
    let (_, found) = c.get("/api/v1/games").await;
    assert_eq!(found.as_array().unwrap().len(), 2);
    let (s, _) = c.get("/api/v1/games?platform=amiga").await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);

    // Lookup tells the platform.
    let (_, m) = c.post("/api/v1/games/lookup", json!({"files": [{"name": "motorace.exe", "crc32": "9f3e0c21"}]}), "203.0.113.50").await;
    assert_eq!(m[0]["platform"], "win32");

    // Staff correct a version's platform, and can clear it.
    let keen_v = keen["versions"][0]["id"].as_i64().unwrap();
    let (s, v) = c.call(Method::PATCH, &format!("/api/v1/versions/{keen_v}"), Some(json!({"platform": "win16"})), Some(&token), "198.51.100.9", None).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    assert_eq!(v["platform"], "win16");
    let (_, v) = c.call(Method::PATCH, &format!("/api/v1/versions/{keen_v}"), Some(json!({"label": "1.4"})), Some(&token), "198.51.100.9", None).await;
    assert_eq!(v["platform"], "win16", "a patch without platform leaves it");
    let (_, schema) = c.get("/api/v1/schema").await;
    assert_eq!(schema["version"], 2);
    assert!(schema["settings"].as_array().unwrap().iter().any(|s| s["key"] == "win32_depth" && s["group"] == "windows"));
}
