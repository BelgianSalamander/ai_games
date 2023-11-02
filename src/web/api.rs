use std::{
    collections::HashMap, convert::Infallible, mem::MaybeUninit, net::SocketAddr, pin::pin,
    sync::Arc, str::FromStr,
};

use async_std::{
    net::{TcpListener, TcpStream},
    stream::Map,
    sync::Mutex,
};
use futures::{AsyncRead, AsyncReadExt, AsyncWrite};
use gamedef::game_interface::GameInterface;
use log::{info, debug};
use rand::Rng;
use serde_json::{json, Value};

use crate::{
    games::Game,
    players::{auto_exec::GameRunner, player::Player},
    web::http::{Method, Request, Response, Status}, langs::language::{Language, PreparedProgram},
};

use super::{profile::{Profile, AgentInfo}, http::HttpError};

trait IgnoreResult {
    fn ignore(self);
}

impl<T, E> IgnoreResult for Result<T, E> {
    fn ignore(self) {}
}

type HttpResult<T> = Result<T, Response>;


#[derive(Clone)]
pub struct AppState {
    executor: Arc<GameRunner<Box<dyn Game>>>,
    profiles: Arc<Mutex<Vec<Profile>>>,
    super_secret_admin_password: String,
    languages: Arc<Vec<Arc<dyn Language>>>,
    itf: GameInterface
}

async fn get_all_players(state: AppState) -> String {
    let lock = state.executor.scores.lock().await;

    let mut json = Vec::new();

    for (id, data) in lock.iter() {
        let val = json!({
            "id": id.get(),
            "name": data.name,
            "rating": data.rating as i32,
            "removed": data.removed
        });

        json.push(val);
    }

    serde_json::to_string(&json).unwrap()
}

async fn get_all_profiles(state: AppState) -> String {
    let lock = state.profiles.lock().await;

    let mut json = Vec::new();

    for profile in lock.iter() {
        let val = json!({
            "id": profile.id,
            "username": profile.username,
            "password": profile.password,

            "num_agents_allowed": profile.num_agents_allowed
        });

        json.push(val);
    }

    serde_json::to_string(&json).unwrap()
}

fn get_file_type(path: &str) -> &'static str {
    if !path.contains(".") {
        //Arbitrary binary data
        return "application/octet-stream";
    }

    let ext = path.split(".").last().unwrap().to_ascii_lowercase();

    match ext.as_str() {
        "html" => "text/html",
        "css" => "text/css",
        "js" => "text/javascript",
        "json" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "txt" => "text/plain",
        "md" => "text/markdown",
        "pdf" => "application/pdf",
        "mp3" => "audio/mpeg",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "ogg" => "audio/ogg",
        "wav" => "audio/wav",
        "zip" => "application/zip",
        "tar" => "application/x-tar",
        "gz" => "application/gzip",
        "bz2" => "application/x-bzip2",
        "7z" => "application/x-7z-compressed",
        "rar" => "application/x-rar-compressed",

        _ => "application/octet-stream",
    }
}

async fn serve_file_to(path: &str) -> HttpResult<Response> {
    if path.contains("..") {
        return Err(Response::basic_error(Status::BadRequest, "Invalid path"));
    }

    const BASE_PATH: &'static str = "public";

    let path = format!("{}/{}", BASE_PATH, path);

    let mut file = match async_std::fs::File::open(&path).await {
        Ok(file) => file,
        Err(e) => {
            println!("Error opening file {}: {}", path, e);

            return Err(Response::basic_error(Status::NotFound, "File not found"))
        }
    };

    let mut buf = Vec::new();

    if let Err(e) = file.read_to_end(&mut buf).await {
        println!("Error reading file {}: {}", path, e);

        return Err(Response::basic_error(Status::InternalServerError, "Error reading file"));
    }

    let file_type = get_file_type(&path);

    let mut response = Response::new();
    response.set_status(Status::Ok);
    response.set_header("Content-Type", file_type);
    response.set_body(buf);

    Ok(response)
}

fn authenticate_admin(req: &Request, state: &AppState) -> bool {
    if let Some(auth) = req.cookies.get("admin") {
        if *auth == state.super_secret_admin_password {
            return true;
        }
    }

    false
}

async fn get_user(req: &Request, state: &AppState, admin_only: bool) -> (bool, Option<u32>) {
    let id = match req.path.query.get("id") {
        Some(id) => {
            match id.parse::<u32>() {
                Ok(id) => Some(id),
                Err(e) => None
            }
        }
        None => None
    };

    if authenticate_admin(req, state) {
        return (true, id);
    } else if admin_only {
        return (false, None)
    }

    if id == None {
        return (false, None);
    }

    let lock = state.profiles.lock().await;

    let profile = lock.iter().find(|p| p.id == id.unwrap());

    if profile.is_none() {
        return (false, None);
    }

    let profile = profile.unwrap();

    let user_id = match req.cookies.get("id") {
        Some(id) => {
            match id.parse::<u32>() {
                Ok(id) => Some(id),
                Err(e) => None
            }
        }
        None => None
    };

    let user_password = req.cookies.get("password");

    if user_id == None || user_password == None {
        return (false, id);
    }

    let user_id = user_id.unwrap();
    let user_password = user_password.unwrap();

    if user_id != profile.id {
        return (false, id);
    }

    if profile.password != *user_password {
        return (false, id);
    }

    (true, id)
}

// If the profile was not found, an error message is sent
fn find_profile<'a>(req: &Request, profiles: &'a Vec<Profile>) -> HttpResult<usize> {
    let id = match req.path.query.get("id") {
        Some(id) => {
            match id.parse::<u32>() {
                Ok(id) => Some(id),
                Err(e) => {
                    return Err(Response::basic_error(Status::BadRequest, &format!("Invalid id: {}", e)));
                }
            }
        }
        None => None
    };

    let username = req.path.query.get("username");

    if id == None && username == None {
        return Err(Response::basic_error(Status::BadRequest, "Missing id or username"));
    }

    for i in 0..profiles.len() {
        let profile = &profiles[i];

        if (id == None || Some(profile.id) == id) && (username == None || Some(&profile.username) == username) {
            return Ok(i)
        }
    }

    return Err(Response::basic_error(Status::NotFound, "Profile not found"));
}

fn logged_in(profile: &Profile, req: &Request) -> bool {
    let user_id = match req.cookies.get("id") {
        Some(id) => {
            match id.parse::<u32>() {
                Ok(id) => Some(id),
                Err(e) => None
            }
        }
        None => None
    };

    let user_password = req.cookies.get("password");

    if user_id == None || user_password == None {
        return false;
    }

    let user_id = user_id.unwrap();
    let user_password = user_password.unwrap();

    if user_id != profile.id {
        return false;
    }

    if profile.password != *user_password {
        return false;
    }

    true
}

async fn get_profile(req: &Request, state: &AppState) -> HttpResult<Response> {
    let profiles = state.profiles.lock().await;
    let idx = find_profile(&req, &profiles)?;
    let profile = &profiles[idx];

    let logged = logged_in(profile, req);
    let authenticated = authenticate_admin(req, state) || logged;

    let mut data = HashMap::new();

    data.insert("id", json!(profile.id));
    data.insert("username", json!(profile.username));
    data.insert("logged_in", json!(logged));
    data.insert("privileged", json!(authenticated));

    if authenticated {
        data.insert("max_agents", json!(profile.num_agents_allowed));

        let mut agents = Vec::new();
        
        for agent in profile.agents.iter() {
            agents.push(json!({}));
        }

        data.insert("agents", json!(agents));
    }

    let mut res = Response::new();
    res.set_status(Status::Ok);
    res.set_header("Content-Type", "application/json");
    res.set_body(serde_json::to_string(&data).unwrap().into_bytes());

    Ok(res)
}

async fn route_get(addr: SocketAddr, req: Request, state: AppState) -> HttpResult<Response> {
    if req.matches_path_exact(&[]) {
        let mut res = Response::new();
        res.set_status(Status::PermanentRedirect);
        res.set_header("Location", "/public/index.html");

        Ok(res)
    } else if req.matches_path(&["admin"]) {
        if !authenticate_admin(&req, &state) {
            Err(Response::basic_error(Status::Unauthorized, "Unauthorized"))
        } else if req.matches_path_exact(&["admin", "verify"]) {
            let mut res = Response::new();
            res.set_status(Status::Ok);
            
            Ok(res)
        } else if req.matches_path_exact(&["admin", "profiles"]) {
            let mut res = Response::new();
            res.set_status(Status::Ok);
            res.set_header("Content-Type", "application/json");
            res.set_body(get_all_profiles(state).await.into_bytes());

            Ok(res)
        } else {
            Err(Response::basic_error(Status::NotFound, "Not found"))
        }
    } else if req.matches_path(&["public"]) {
        let path = req.path.path[1..].join("/");

        serve_file_to(&path).await
    } else if req.matches_path_exact(&["api", "players"]) {
        let mut res = Response::new();
        res.set_status(Status::Ok);
        res.set_header("Content-Type", "application/json");
        res.set_body(get_all_players(state).await.into_bytes());

        Ok(res)
    } else if req.matches_path_exact(&["api", "profile"]) {
        get_profile(&req, &state).await
    } else if req.matches_path_exact(&["api", "game"]) {
        let mut res = Response::new();
        res.set_status(Status::Ok);
        res.set_header("Content-Type", "application/json");
        res.set_body(json!({
            "name": state.executor.game.name(),
            "num_players": state.executor.game.num_players()
        }).to_string().into_bytes());

        Ok(res)
    } else if req.matches_path_exact(&["api", "auth"]) {
        let profiles = state.profiles.lock().await;
        let idx = find_profile(&req, &profiles)?;
        let profile = &profiles[idx];

        let password = req.path.get("password")?;
        let mut res = Response::new();
        res.set_status(Status::Ok);

        res.set_header("Content-Type", "application/json");

        let correct = profile.password == *password;

        res.set_body(json!({
            "correct": correct
        }).to_string().into_bytes());

        Ok(res)
    } else if req.matches_path_exact(&["api", "lang"]) {
        let values: Vec<_> = state.languages.iter().map(|l| {
            json!({
                "name": l.name(),
                "id": l.id()
            })
        }).collect();

        let arr = Value::Array(values);

        let mut res = Response::new();
        res.set_status(Status::Ok);
        res.set_header("Content-Type", "application/json");
        res.set_body(serde_json::to_string(&arr).unwrap().into_bytes());

        Ok(res)
    } else {
        Err(Response::basic_error(Status::NotFound, "Not found"))
    }
}

fn make_user(profiles: &mut Vec<Profile>, username: &String, num_agents: usize) {
    let mut rng = rand::thread_rng();
    let id = loop {
        let id = rng.gen_range(0..u32::MAX);

        if !profiles.iter().any(|p| p.id == id) {
            break id;
        }
    };

    profiles.push(Profile::new(id, username.clone(), num_agents));
}

async fn reset_password(req: &Request, state: &AppState) -> HttpResult<Response> {
    let (authenticated, id) = get_user(req, state, false).await;

    if id == None {
        return Err(Response::basic_error(Status::BadRequest, "Invalid id"));
    }

    let id = id.unwrap();
    let mut lock = state.profiles.lock().await;
    let profile = lock.iter_mut().find(|p| p.id == id).unwrap();

    profile.regenerate_password();

    let mut res = Response::new();
    res.set_status(Status::Ok);
    res.set_header("Content-Type", "text/plain");
    res.set_body(profile.password.clone().into_bytes());

    Ok(res)
}

async fn route_post(addr: SocketAddr, req: Request, state: AppState) -> HttpResult<Response>{
    if req.matches_path(&["admin"]) {
        if !authenticate_admin(&req, &state) {
            Err(Response::basic_error(Status::Unauthorized, "Unauthorized"))
        }else if req.matches_path_exact(&["admin", "new_profile"]) {
            let mut profiles = state.profiles.lock().await;
            let username = req.path.get("username")?;

            //Check if username is already taken
            if profiles.iter().any(|p| p.username == *username) {
                return Err(Response::basic_error(Status::BadRequest, "Username already taken"));
            }

            let num_agents_allowed = req.path.parse_query("agents")?;

            make_user(&mut profiles, username, num_agents_allowed);

            let mut res = Response::new();
            res.set_status(Status::Ok);

            Ok(res)
        } else if req.matches_path_exact(&["admin", "delete_profile"]) {
            let mut profiles = state.profiles.lock().await;
            let target_idx = find_profile(&req, &profiles)?;

            let profile = &profiles[target_idx];
            info!("Deleting profile: id: {:?}, username: {:?}", profile.id, profile.username);

            profiles.swap_remove(target_idx);

            let mut res = Response::new();
            res.set_status(Status::Ok);
                
            Ok(res)
        } else if req.matches_path_exact(&["admin", "set_profile_agents"]) {
            let num_agents_allowed = req.path.parse_query("agents")?;

            let mut lock = state.profiles.lock().await;
            let target_idx = find_profile(&req, &lock)?;
            
            lock[target_idx].num_agents_allowed = num_agents_allowed;

            let mut res = Response::new();
            res.set_status(Status::Ok);

            Ok(res)
        } else {
            Err(Response::basic_error(Status::NotFound, "Not found"))
        }
    } else if req.matches_path_exact(&["api", "reset_password"]) {
        reset_password(&req, &state).await
    } else if req.matches_path_exact(&["api", "add_agent"]) {
        let mut profiles = state.profiles.lock().await;
        let profile = find_profile(&req, &profiles)?;
        let profile = &mut profiles[profile];

        if !logged_in(profile, &req) {
            return Err(Response::basic_error(Status::Unauthorized, "Unauthorized"));
        }

        if profile.agents.len() >= profile.num_agents_allowed {
            return Err(Response::basic_error(Status::Conflict, &format!("User already has {}/{} agents!", profile.agents.len(), profile.num_agents_allowed)));
        }

        let data = match String::from_utf8(req.body.clone()) {
            Err(e) => return Err(Response::basic_error(Status::BadRequest, "Couldn't decode body. body should be UTF-8")),
            Ok(s) => s
        };

        let data = match Value::from_str(&data) {
            Ok(Value::Object(map)) => map,
            _ => return Err(Response::basic_error(Status::BadRequest, "Couldn't parse json body"))
        };

        let src = match data.get("src") {
            Some(Value::String(d)) => d,
            _ => return Err(Response::basic_error(Status::BadRequest, "Expected key 'src'"))
        };

        let language_id = match data.get("lang") {
            Some(Value::String(d)) => d,
            _ => return Err(Response::basic_error(Status::BadRequest, "Expected key 'lang'"))
        };

        let name = match data.get("name") {
            Some(Value::String(d)) => d,
            _ => return Err(Response::basic_error(Status::BadRequest, "Expected key 'name'"))
        };

        let language = state.languages.iter().filter(|l| l.id() == language_id).next();
        let language = match language {
            Some(l) => l,
            None => return Err(Response::basic_error(Status::BadRequest, &format!("Unknown language {}", language_id)))
        };

        let id = state.executor.make_id();

        let mut in_use = false;
        for player in state.executor.scores.lock().await.values() {
            if player.name.to_lowercase() == name.to_lowercase() {
                in_use = true;
                break;
            }
        }

        if in_use {
            return Err(Response::basic_error(Status::BadRequest, &format!("Agent name already used!")));
        }

        let mut program = PreparedProgram::new();
        match language.prepare(&src, &mut program, &state.itf) {
            Err(s) => return Err(Response::basic_error(Status::BadRequest, &format!("Failed to prepare program! {}", s))),
            _ => {}
        }

        let player = Player::new(id, name.clone(), program, language.clone());

        state.executor.add_player(player).await;

        profile.agents.push(AgentInfo {
            id
        });

        let mut res = Response::new();
        res.set_status(Status::Ok);
        res.set_header("Content-Type", "application/json");
        res.set_body(serde_json::to_string(&json!({
            "agent_id": id.get()
        })).unwrap().into_bytes());

        Ok(res)
    } else {
        Err(Response::basic_error(Status::NotFound, "Not found"))
    }
}

async fn handle_conn(mut stream: TcpStream, addr: SocketAddr, state: AppState) {
    let request = match Request::parse_async(&mut stream).await {
        Ok(request) => request,
        Err(e) => {
            println!("Error parsing request from {}: {}", addr, e);

            Response::basic_error(Status::BadRequest, &format!("Error parsing request: {}", e))
                .write_async(&mut stream)
                .await
                .ignore();

            return;
        }
    };

    info!("Received request [{} {} {}]", addr, request.method, request.path);

    let result = match request.method {
        Method::Get => route_get(addr, request, state).await,
        Method::Post => route_post(addr, request, state).await,

        _ => Err(Response::basic_error(Status::NotImplemented, "Method not implemented"))
    };

    match result {
        Ok(res) => res.write_async(&mut stream).await.ignore(),
        Err(res) => {
            info!("Request [{}] was unsuccesful ({})", addr, res.status);
            res.write_async(&mut stream).await.ignore();
        }
    }
}

fn generate_admin_password() -> String {
    use rand::Rng;

    let mut rng = rand::thread_rng();

    //rand::distributions::Alphanumeric

    const CHARSET: &[u8] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789)(*&^%$#@!~";
    const PASSWORD_LEN: usize = 50;

    (0..PASSWORD_LEN)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

pub async fn launch_and_run_api(executor: Arc<GameRunner<Box<dyn Game>>>, languages: Vec<Arc<dyn Language>>, itf: GameInterface) -> std::io::Result<()> {
    let addr = SocketAddr::from(([0, 0, 0, 0], 42069));

    let listener = TcpListener::bind(addr).await.unwrap();

    let state = AppState {
        executor,
        profiles: Arc::new(Mutex::new(Vec::new())),
        super_secret_admin_password: generate_admin_password(),
        languages: Arc::new(languages),
        itf
    };

    println!("Admin password: {}", state.super_secret_admin_password);

    println!("Listening on {}", addr);

    loop {
        let (stream, addr) = listener.accept().await?;

        async_std::task::spawn(handle_conn(stream, addr, state.clone()));
    }
}
