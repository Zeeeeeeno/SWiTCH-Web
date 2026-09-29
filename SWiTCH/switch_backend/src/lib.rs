use axum::{
    extract::{Multipart, Query},
    response::Json,
    routing::{get, post},
    Router,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::Path, process::Command};
use sysinfo::Disks;
use tower_http::{cors::CorsLayer, services::ServeDir};

#[derive(Serialize, Deserialize, Clone)]
struct Game {
    appid: String,
    name: String,
    cover_url: String,
    launch_path: String,
    is_custom: bool,
}

#[derive(Serialize, Deserialize, Default, Clone)]
struct CustomData {
    custom_games: Vec<Game>,
    custom_covers: HashMap<String, String>,
    custom_names: HashMap<String, String>,
}

#[derive(Deserialize)]
struct SearchQuery {
    q: Option<String>,
}

#[derive(Deserialize)]
struct AppIdReq {
    appid: String,
}

#[derive(Deserialize)]
struct RenameReq {
    appid: String,
    name: String,
}

#[derive(Deserialize)]
struct AddGameReq {
    name: String,
    launch_path: String,
    cover_url: Option<String>,
    category: Option<String>,
}

#[derive(Deserialize)]
struct EpicManifest {
    #[serde(rename = "DisplayName")]
    display_name: String,
    #[serde(rename = "AppName")]
    app_name: String,
}


pub async fn run_backend() {
    let _ = fs::create_dir_all("uploads");

    let app = Router::new()
        .route("/api/games", get(get_steam_games))
        .route("/api/epic-games", get(get_epic_games))
        .route("/api/ubisoft-games", get(get_ubisoft_games))
        .route("/api/riot-games", get(get_riot_games))
        .route("/api/rockstar-games", get(get_rockstar_games))
        .route("/api/windows-apps", get(get_windows_apps))
        .route("/api/all-games", get(get_all_games))
        .route("/api/launch", get(launch_game))
        .route("/api/upload-image", post(upload_image))
        .route("/api/remove-image", post(remove_image))
        .route("/api/revert-image", post(revert_image))
        .route("/api/add-game", post(add_custom_game))
        .route("/api/delete-game", post(delete_custom_game))
        .route("/api/rename-game", post(rename_game))
        .nest_service("/uploads", ServeDir::new("uploads"))
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn load_custom_data() -> CustomData {
    if let Ok(content) = fs::read_to_string("custom_games.json") {
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        CustomData::default()
    }
}

fn save_custom_data(data: &CustomData) {
    if let Ok(content) = serde_json::to_string_pretty(data) {
        let _ = fs::write("custom_games.json", content);
    }
}

// Fügt Custom Games, Custom Names & Custom Covers zu allen Anfragen hinzu
fn apply_custom_overlays(mut games: Vec<Game>, query: &Option<String>, target_category: Option<&str>) -> Vec<Game> {
    let custom_data = load_custom_data();

    // Nur Custom Games hinzufügen, die zur aktuellen Kategorie passen (oder wenn target_category None ist -> für "all")
    for cg in &custom_data.custom_games {
        let is_matching_category = match target_category {
            Some(cat) => cg.appid.starts_with(&format!("custom_{}_", cat)),
            None => true, // Für die "All"-Seite alle anzeigen
        };

        if is_matching_category && !games.iter().any(|g| g.appid == cg.appid) {
            games.push(cg.clone());
        }
    }

    for game in games.iter_mut() {
        if let Some(custom_name) = custom_data.custom_names.get(&game.appid) {
            game.name = custom_name.clone();
        }
        if let Some(custom_cover) = custom_data.custom_covers.get(&game.appid) {
            game.cover_url = custom_cover.clone();
        }
    }

    if let Some(q) = query {
        let search_term = q.to_lowercase();
        games.retain(|g| g.name.to_lowercase().contains(&search_term));
    }
    games
}

async fn get_steam_games(Query(query): Query<SearchQuery>) -> Json<Vec<Game>> {
    let games = scan_steam_games();
    Json(apply_custom_overlays(games, &query.q, Some("steam")))
}

async fn get_epic_games(Query(query): Query<SearchQuery>) -> Json<Vec<Game>> {
    let games = scan_epic_games();
    Json(apply_custom_overlays(games, &query.q, Some("epic")))
}

async fn get_ubisoft_games(Query(query): Query<SearchQuery>) -> Json<Vec<Game>> {
    let games = scan_ubisoft_games();
    Json(apply_custom_overlays(games, &query.q, Some("ubisoft")))
}

async fn get_riot_games(Query(query): Query<SearchQuery>) -> Json<Vec<Game>> {
    let games = scan_riot_games();
    Json(apply_custom_overlays(games, &query.q, Some("riot")))
}

async fn get_rockstar_games(Query(query): Query<SearchQuery>) -> Json<Vec<Game>> {
    let games = scan_rockstar_games();
    Json(apply_custom_overlays(games, &query.q, Some("rockstar")))
}

async fn get_windows_apps(Query(query): Query<SearchQuery>) -> Json<Vec<Game>> {
    let games = scan_windows_apps();
    Json(apply_custom_overlays(games, &query.q, Some("windows")))
}

async fn get_all_games(Query(query): Query<SearchQuery>) -> Json<Vec<Game>> {
    let mut all = Vec::new();
    all.extend(scan_steam_games());
    all.extend(scan_epic_games());
    all.extend(scan_ubisoft_games());
    all.extend(scan_riot_games());
    all.extend(scan_rockstar_games());
    all.extend(scan_windows_apps());

    Json(apply_custom_overlays(all, &query.q, None)) // None = Alle Custom Games anzeigen
}

async fn launch_game(Query(query): Query<AppIdReq>) -> &'static str {
    let custom_data = load_custom_data();
    let game = custom_data
        .custom_games
        .iter()
        .find(|g| g.appid == query.appid);

    let launch_cmd = match game {
        Some(g) => g.launch_path.clone(),
        None => {
            if query.appid.starts_with("epic_") {
                let app_name = query.appid.trim_start_matches("epic_");
                format!("com.epicgames.launcher://apps/{}?action=launch&silent=true", app_name)
            } else if query.appid.starts_with("uplay_") {
                let id = query.appid.trim_start_matches("uplay_");
                format!("uplay://launch/{}/0", id)
            } else if query.appid.starts_with("riot_") {
                scan_riot_games()
                    .into_iter()
                    .find(|g| g.appid == query.appid)
                    .map(|g| g.launch_path)
                    .unwrap_or_default()
            } else if query.appid.starts_with("win_") {
                scan_windows_apps()
                    .into_iter()
                    .find(|g| g.appid == query.appid)
                    .map(|g| g.launch_path)
                    .unwrap_or_default()
            } else {
                format!("steam://run/{}", query.appid)
            }
        }
    };

    #[cfg(target_os = "windows")]
    {
        if !launch_cmd.is_empty() {
            Command::new("cmd").args(["/C", "start", "", &launch_cmd]).spawn().ok();
        }
    }
    "OK"
}

async fn rename_game(Json(payload): Json<RenameReq>) -> Json<serde_json::Value> {
    let mut custom_data = load_custom_data();
    if let Some(game) = custom_data.custom_games.iter_mut().find(|g| g.appid == payload.appid) {
        game.name = payload.name.clone();
    }
    custom_data.custom_names.insert(payload.appid, payload.name);
    save_custom_data(&custom_data);
    Json(serde_json::json!({ "success": true }))
}

async fn delete_custom_game(Json(payload): Json<AppIdReq>) -> Json<serde_json::Value> {
    let mut custom_data = load_custom_data();
    custom_data.custom_games.retain(|g| g.appid != payload.appid);
    custom_data.custom_covers.remove(&payload.appid);
    custom_data.custom_names.remove(&payload.appid);
    save_custom_data(&custom_data);
    Json(serde_json::json!({ "success": true }))
}

async fn remove_image(Json(payload): Json<AppIdReq>) -> Json<serde_json::Value> {
    let mut custom_data = load_custom_data();
    custom_data.custom_covers.insert(payload.appid, String::new());
    save_custom_data(&custom_data);
    Json(serde_json::json!({ "success": true }))
}

async fn revert_image(Json(payload): Json<AppIdReq>) -> Json<serde_json::Value> {
    let mut custom_data = load_custom_data();
    custom_data.custom_covers.remove(&payload.appid);
    save_custom_data(&custom_data);
    Json(serde_json::json!({ "success": true }))
}

async fn upload_image(mut multipart: Multipart) -> Json<serde_json::Value> {
    let mut appid = String::new();
    let mut file_path = String::new();

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or("").to_string();
        if name == "appid" {
            appid = field.text().await.unwrap_or_default();
        } else if name == "file" {
            let file_name = field.file_name().unwrap_or("cover.png").to_string();
            let data = field.bytes().await.unwrap_or_default();
            let save_path = format!("uploads/{}_{}", appid, file_name);
            if fs::write(&save_path, data).is_ok() {
                file_path = format!("http://127.0.0.1:3000/{}", save_path);
            }
        }
    }

    if !appid.is_empty() && !file_path.is_empty() {
        let mut custom_data = load_custom_data();
        custom_data.custom_covers.insert(appid, file_path.clone());
        save_custom_data(&custom_data);
        return Json(serde_json::json!({ "success": true, "url": file_path }));
    }

    Json(serde_json::json!({ "success": false }))
}

async fn add_custom_game(Json(payload): Json<AddGameReq>) -> Json<serde_json::Value> {
    let mut custom_data = load_custom_data();
    let category = payload.category.unwrap_or_else(|| "custom".to_string());
    
    // Präfix enthält jetzt die spezifische Kategorie (z. B. custom_riot_169... oder custom_steam_169...)
    let appid = format!("custom_{}_{}", category, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());

    let new_game = Game {
        appid: appid.clone(),
        name: payload.name,
        cover_url: payload.cover_url.unwrap_or_default(),
        launch_path: payload.launch_path,
        is_custom: true,
    };

    custom_data.custom_games.push(new_game);
    save_custom_data(&custom_data);

    Json(serde_json::json!({ "success": true, "appid": appid }))
}

/* Scanner Implementationen */

fn scan_steam_games() -> Vec<Game> {
    let mut games = Vec::new();
    let disks = Disks::new_with_refreshed_list();
    let name_regex = Regex::new(r#""name"\s+"([^"]+)""#).unwrap();
    let appid_regex = Regex::new(r#""appid"\s+"([^"]+)""#).unwrap();

    for disk in &disks {
        let mount = disk.mount_point();
        let possible_paths = vec![
            mount.join("Program Files (x86)").join("Steam"),
            mount.join("Program Files").join("Steam"),
            mount.join("SteamLibrary"),
            mount.join("Games").join("SteamLibrary"),
        ];

        for path in possible_paths {
            let steamapps = path.join("steamapps");
            if steamapps.exists() {
                if let Ok(entries) = fs::read_dir(&steamapps) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if p.extension().and_then(|s| s.to_str()) == Some("acf") {
                            if let Ok(content) = fs::read_to_string(&p) {
                                let name = name_regex.captures(&content).map(|c| c[1].to_string()).unwrap_or_default();
                                let appid = appid_regex.captures(&content).map(|c| c[1].to_string()).unwrap_or_default();

                                if !name.is_empty() && !appid.is_empty() {
                                    let cover_url = format!("https://cdn.cloudflare.steamstatic.com/steam/apps/{}/header.jpg", appid);
                                    games.push(Game {
                                        appid: appid.clone(),
                                        name,
                                        cover_url,
                                        launch_path: format!("steam://run/{}", appid),
                                        is_custom: false,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    games
}

fn scan_epic_games() -> Vec<Game> {
    let mut games = Vec::new();
    let manifest_dir = Path::new(r"C:\ProgramData\Epic\EpicGamesLauncher\Data\Manifests");

    if manifest_dir.exists() {
        if let Ok(entries) = fs::read_dir(manifest_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("item") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(manifest) = serde_json::from_str::<EpicManifest>(&content) {
                            let launch_cmd = format!("com.epicgames.launcher://apps/{}?action=launch&silent=true", manifest.app_name);
                            let appid = format!("epic_{}", manifest.app_name);

                            games.push(Game {
                                appid,
                                name: manifest.display_name,
                                cover_url: String::new(),
                                launch_path: launch_cmd,
                                is_custom: false,
                            });
                        }
                    }
                }
            }
        }
    }
    games
}

fn scan_ubisoft_games() -> Vec<Game> {
    let mut games = Vec::new();
    let ubisoft_dir = Path::new(r"C:\Program Files (x86)\Ubisoft\Ubisoft Game Launcher\games");

    if ubisoft_dir.exists() {
        if let Ok(entries) = fs::read_dir(ubisoft_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(folder_name) = path.file_name().and_then(|s| s.to_str()) {
                        games.push(Game {
                            appid: format!("uplay_{}", folder_name),
                            name: folder_name.to_string(),
                            cover_url: String::new(),
                            launch_path: format!("uplay://launch/{}", folder_name),
                            is_custom: false,
                        });
                    }
                }
            }
        }
    }
    games
}

// Erkennung für Riot Games erweitert (sucht via Json-Client-Installs & Standardpfaden)
fn scan_riot_games() -> Vec<Game> {
    let mut games = Vec::new();
    
    // Methde A: RiotClientInstalls.json
    let riot_installs = Path::new(r"C:\ProgramData\Riot Games\RiotClientInstalls.json");
    if riot_installs.exists() {
        if let Ok(content) = fs::read_to_string(riot_installs) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(associated) = json.get("associated_client") {
                    if let Some(map) = associated.as_object() {
                        for (_path_key, exe_val) in map {
                            let exe_path = exe_val.as_str().unwrap_or_default();
                            if exe_path.is_empty() { continue; }

                            let game_name = Path::new(exe_path)
                                .parent()
                                .and_then(|p| p.file_name())
                                .and_then(|s| s.to_str())
                                .unwrap_or("Riot Game");

                            games.push(Game {
                                appid: format!("riot_{}", game_name.to_lowercase()),
                                name: game_name.to_string(),
                                cover_url: String::new(),
                                launch_path: exe_path.to_string(),
                                is_custom: false,
                            });
                        }
                    }
                }
            }
        }
    }

    // Methode B: Direkter Fallback-Check auf VALORANT & League of Legends Pfade
    let valorant_path = Path::new(r"C:\Riot Games\VALORANT\live\VALORANT.exe");
    if valorant_path.exists() && !games.iter().any(|g| g.name.to_lowercase() == "valorant") {
        games.push(Game {
            appid: "riot_valorant".to_string(),
            name: "VALORANT".to_string(),
            cover_url: String::new(),
            launch_path: valorant_path.to_string_lossy().to_string(),
            is_custom: false,
        });
    }

    let lol_path = Path::new(r"C:\Riot Games\League of Legends\LeagueClient.exe");
    if lol_path.exists() && !games.iter().any(|g| g.name.to_lowercase().contains("league")) {
        games.push(Game {
            appid: "riot_lol".to_string(),
            name: "League of Legends".to_string(),
            cover_url: String::new(),
            launch_path: lol_path.to_string_lossy().to_string(),
            is_custom: false,
        });
    }

    games
}

fn scan_rockstar_games() -> Vec<Game> {
    let mut games = Vec::new();
    let rockstar_dir = Path::new(r"C:\Program Files\Rockstar Games");

    if rockstar_dir.exists() {
        if let Ok(entries) = fs::read_dir(rockstar_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(folder_name) = path.file_name().and_then(|s| s.to_str()) {
                        games.push(Game {
                            appid: format!("rockstar_{}", folder_name),
                            name: folder_name.to_string(),
                            cover_url: String::new(),
                            launch_path: path.to_string_lossy().to_string(),
                            is_custom: false,
                        });
                    }
                }
            }
        }
    }
    games
}

fn scan_windows_apps() -> Vec<Game> {
    let mut apps = Vec::new();
    let mut dirs_to_scan = Vec::new();

    if let Ok(appdata) = std::env::var("APPDATA") {
        dirs_to_scan.push(Path::new(&appdata).join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    if let Ok(programdata) = std::env::var("PROGRAMDATA") {
        dirs_to_scan.push(Path::new(&programdata).join(r"Microsoft\Windows\Start Menu\Programs"));
    }

    for base_dir in dirs_to_scan {
        if base_dir.exists() {
            scan_lnk_recursive(&base_dir, &mut apps);
        }
    }
    apps
}

fn scan_lnk_recursive(dir: &Path, apps: &mut Vec<Game>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                scan_lnk_recursive(&path, apps);
            } else if path.extension().and_then(|s| s.to_str()) == Some("lnk") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    let name = stem.to_string();
                    let lower_name = name.to_lowercase();
                    if lower_name.contains("uninstall") || lower_name.contains("hilfe") || lower_name.contains("documentation") {
                        continue;
                    }

                    if !apps.iter().any(|a| a.name == name) {
                        apps.push(Game {
                            appid: format!("win_{}", name.replace(' ', "_")),
                            name,
                            cover_url: String::new(),
                            launch_path: path.to_string_lossy().to_string(),
                            is_custom: false,
                        });
                    }
                }
            }
        }
    }
}