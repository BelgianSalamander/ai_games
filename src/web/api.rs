use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use async_std::{net::{TcpListener, TcpStream},path::Path, sync::Mutex,};
use futures::AsyncReadExt;
use log::{info, error, warn, debug};
use rand::Rng;
use sea_orm::{DatabaseConnection, EntityTrait, ModelTrait, ActiveValue, ActiveModelTrait, QueryFilter, ColumnTrait, QueryOrder};
use serde_json::{json, Value, Map};

use crate::{
    games::Game,
    web::{http::{Method, Request, Response, Status}, web_errors::WebError}, langs::{language::{Language, PreparedProgram}, get_all_languages}, entities::{self, user, agent}, util::{temp_file::random_file, RUN_DIR}, players::auto_exec::GameRunner, cleanup_files,
};

use super::{profile::{generate_password, get_num_agents}, web_errors::{HttpResult, decode_utf8, ValueCast, parse_json_as_object, HttpErrorMap}, game_reporter::SharedInner};

trait IgnoreResult {
    fn ignore(self);
}

impl<T, E> IgnoreResult for Result<T, E> {
    fn ignore(self) {}
}

#[derive(Clone)]
pub struct PageInfo {
    title: String,
    filename: String,
    heading: String
}

impl PageInfo {
    pub fn from_json(value: &Value) -> Self {
        let obj = value.as_object().unwrap();

        Self {
            title: obj.get("title").unwrap().as_str().unwrap().to_string(),
            filename: obj.get("filename").unwrap().as_str().unwrap().to_string(),
            heading: obj.get("heading").unwrap().as_str().unwrap().to_string(),
        }
    }
}

#[derive(Clone)]
pub struct PageEngine {
    template: String,
    pages: HashMap<String, PageInfo>
}

impl PageEngine {
    pub fn load() -> Self {
        let template = std::fs::read_to_string("res/template.html").unwrap();

        let pages = std::fs::read_to_string("res/pages/pages.json").unwrap();
        let pages: Value = serde_json::from_str(&pages).unwrap();
        println!("Pages {:?}", pages);
        let pages = pages.as_object().unwrap();

        let pages: HashMap<_,_> = pages.into_iter().map(|(key, val)| {
            (key.clone(), PageInfo::from_json(val))
        }).collect();

        Self {
            template,
            pages
        }
    }

    pub fn get_page(&self, name: &str) -> Option<String> {
        if let Some(info) = self.pages.get(name) {
            let mut result = self.template.clone();

            let content = match std::fs::read_to_string(&format!("./res/pages/{}", name)) {
                Ok(x) => x,
                Err(e) => {
                    error!("Failed to read {name} {e:?}");
                    return None;
                }
            };

            result = result.replace("[TITLE]", &info.title);
            result = result.replace("[filename]", &info.filename);
            result = result.replace("[HEADING]", &info.heading);
            result = result.replace("[CONTENT]", &content);

            Some(result)
        } else {
            None
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    executor: Arc<GameRunner<Box<dyn Game>>>,
    super_secret_admin_password: String,
    languages: Arc<Vec<Arc<dyn Language>>>,
    reporter: Arc<Mutex<SharedInner>>,
    db: DatabaseConnection,

    page_engine: PageEngine,
}

async fn get_agent_leaderboard(state: AppState) -> HttpResult<String> {
    let mut json = Vec::new();

    let data = entities::prelude::Agent::find()
        .filter(agent::Column::Removed.eq(false))
        .filter(agent::Column::Partial.eq(false))
        .order_by_desc(agent::Column::Rating)
        .find_also_related(user::Entity)
        .all(&state.db).await?;


    for (agent, maybe_owner) in data {
        let mut val = json!({
            "id": agent.id,
            "name": agent.name,
            "rating": agent.rating as i32,
            "colour": agent.colour,
            "games_played": agent.num_games
        });

        if let Some(owner) = maybe_owner {
            val["owner_id"] = json!(owner.id);
            val["owner"] = json!(owner.username);
        }

        json.push(val);
    }

    Ok(serde_json::to_string(&json)?)
}

async fn get_all_profiles(state: AppState) -> HttpResult<String> {
    let mut json = Vec::new();

    for profile in entities::prelude::User::find().all(&state.db).await? {
        let val = json!({
            "id": profile.id,
            "username": profile.username,
            "password": profile.password,

            "num_agents_allowed": profile.num_agents_allowed
        });

        json.push(val);
    }

    Ok(serde_json::to_string(&json)?)
}

fn get_file_type(path: &str) -> &'static str {
    if !path.contains(".") {
        //Arbitrary binary data
        return "application/octet-stream";
    }

    let ext = match path.split(".").last() {
        Some(x) => x,
        None => "application/octet-stream"
    }.to_ascii_lowercase();

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
        return Err(WebError::InvalidData("Invalid file path".to_string()));
    }

    const BASE_PATH: &'static str = "public";

    let full_path = format!("{}/{}", BASE_PATH, path);

    let mut file = match async_std::fs::File::open(&full_path).await {
        Ok(file) => file,
        Err(e) => {
            println!("Error opening file {}: {}", full_path, e);

            return Err(WebError::NotFound(format!("File '{}' was not found", path)))
        }
    };

    let mut buf = Vec::new();

    if let Err(e) = file.read_to_end(&mut buf).await {
        println!("Error reading file {}: {}", full_path, e);

        return Err(WebError::InternalServerError("Error reading file".to_string()));
    }

    let file_type = get_file_type(&full_path);

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

fn is_user_authenticated(req: &Request, profile: &entities::user::Model) -> bool {
    let user_id = match req.cookies.get("id") {
        Some(id) => {
            match id.parse::<i32>() {
                Ok(id) => Some(id),
                Err(_) => None
            }
        }
        None => None
    };

    let user_password = req.cookies.get("password");

    if user_id == None || user_password == None {
        return false;
    }

    if profile.id != user_id.unwrap() || profile.password != *user_password.unwrap() {
        return false;
    }

    true
}

async fn get_user(req: &Request, state: &AppState) -> HttpResult<Option<entities::user::Model>> {
    let id: i32 = req.path.parse_query("id")?;

    let user = entities::prelude::User::find_by_id(id).one(&state.db).await?;

    Ok(user)
}

async fn get_agent_data_as_json(agent: &agent::Model, include_error: bool, include_src: bool, db: &DatabaseConnection) -> HttpResult<serde_json::Value> {
    let mut data = json!({
        "id": agent.id,
        "name": agent.name,
        "language": agent.language,
        "rating": agent.rating,
        "games_played": agent.num_games,
        "in_game": agent.in_game,
        "removed": agent.removed,
        "partial": agent.partial,
        "colour": agent.colour
    });

    if include_error {
        if let Some(error_file) = &agent.error_file {
            if Path::new(&error_file).exists().await {
                let error = async_std::fs::read(error_file).await?;
                let error = String::from_utf8(error).unwrap_or("Error file corrupted :(".to_string());

                data.as_object_mut().unwrap().insert("error".to_string(), Value::String(error));
            }
        }
    }

    if include_src {
        if let Some(src_file) = &agent.source_file {
            if Path::new(&src_file).exists().await {
                let src = async_std::fs::read(src_file).await?;
                let src = String::from_utf8(src).unwrap_or("Source file corrupted (Invalid UTF-8)".to_string());

                data.as_object_mut().unwrap().insert("src".to_string(), Value::String(src));
            }
        }
    }

    if let Some(owner_id) = agent.owner_id {
        if let Some(owner) = user::Entity::find_by_id(owner_id).one(db).await? {
            data.as_object_mut().unwrap().insert("owner_id".to_string(), json!(owner_id));
            data.as_object_mut().unwrap().insert("owner".to_string(), json!(owner.username));
        }
    }

    Ok(data)
}

async fn get_profile_data(req: &Request, state: &AppState) -> HttpResult<Response> {
    let id = match req.path.query.get("id") {
        Some(id) => {
            match id.parse::<i32>() {
                Ok(id) => Some(id),
                Err(_) => None
            }
        }
        None => None
    };

    let username = req.path.query.get("username");

    if username.is_none() && id.is_none() {
        return Err(WebError::InvalidData("Did not specify a username or a user id".to_string()));
    }

    let mut query = user::Entity::find();

    if let Some(id) = id {
        query = query.filter(user::Column::Id.eq(id));
    }

    if let Some(username) = username {
        query = query.filter(user::Column::Username.eq(username));
    }

    let profile = query.one(&state.db).await?;

    if profile.is_none() {
        return Err(WebError::NotFound("Could not find desired user".to_string()));
    }
    let profile = profile.unwrap();

    let logged = is_user_authenticated(req, &profile);
    let authenticated = authenticate_admin(req, state) || logged;

    let mut data = HashMap::new();

    data.insert("id", json!(profile.id));
    data.insert("username", json!(profile.username));
    data.insert("logged_in", json!(logged));
    data.insert("privileged", json!(authenticated));

    if authenticated {
        data.insert("max_agents", json!(profile.num_agents_allowed));

        let mut agents = Vec::new();

        let related = profile.find_related(entities::prelude::Agent).all(&state.db).await?;
        
        for agent in related {
            agents.push(get_agent_data_as_json(&agent, false, false, &state.db).await?);
        }

        data.insert("agents", json!(agents));
    }

    let mut res = Response::new();
    res.set_status(Status::Ok);
    res.set_header("Content-Type", "application/json");
    res.set_body(serde_json::to_string(&data)?.into_bytes());

    Ok(res)
}

async fn route_get(_addr: SocketAddr, req: Request, state: AppState) -> HttpResult<Response> {
    if req.matches_path_exact(&[]) {
        let mut res = Response::new();
        res.set_status(Status::PermanentRedirect);
        res.set_header("Location", "/pages/index.html");

        Ok(res)
    } else if req.matches_path(&["pages"]) && req.path.path.len() > 1 {
        let path = req.path.path.get(1).unwrap();
        match state.page_engine.get_page(&path) {
            Some(x) => {
                let mut res = Response::new();
                res.set_status(Status::Accepted);
                res.set_header("Content-Type", "text/html");
                res.set_body(x.into_bytes());

                Ok(res)
            },
            None => {
                Err(WebError::NotFound("Requested page was not found".to_string()))
            }
        }
    } else if req.matches_path(&["admin"]) {
        if !authenticate_admin(&req, &state) {
            Err(WebError::Unauthorized)
        } else if req.matches_path_exact(&["admin", "verify"]) {
            let mut res = Response::new();
            res.set_status(Status::Ok);
            
            Ok(res)
        } else if req.matches_path_exact(&["admin", "profiles"]) {
            let mut res = Response::new();
            res.set_status(Status::Ok);
            res.set_header("Content-Type", "application/json");
            res.set_body(get_all_profiles(state).await?.into_bytes());

            Ok(res)
        } else {
            Err(WebError::NotFound("Unknown admin route".to_string()))
        }
    } else if req.matches_path(&["public"]) {
        let path = req.path.path[1..].join("/");

        serve_file_to(&path).await
    } else if req.matches_path_exact(&["api", "agent_leaderboard"]) {
        let mut res = Response::new();
        res.set_status(Status::Ok);
        res.set_header("Content-Type", "application/json");
        res.set_body(get_agent_leaderboard(state).await?.into_bytes());

        Ok(res)
    } else if req.matches_path_exact(&["api", "profile"]) {
        get_profile_data(&req, &state).await
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
        //TODO: Extract this profile finding logic into a function
        let id = match req.path.query.get("id") {
            Some(id) => {
                match id.parse::<i32>() {
                    Ok(id) => Some(id),
                    Err(_) => None
                }
            }
            None => None
        };
    
        let username = req.path.query.get("username");
    
        if username.is_none() && id.is_none() {
            return Err(WebError::InvalidData("Could not find desired user (Missing parameters)".to_string()));
        }
    
        let mut query = user::Entity::find();
    
        if let Some(id) = id {
            query = query.filter(user::Column::Id.eq(id));
        }
    
        if let Some(username) = username {
            query = query.filter(user::Column::Username.eq(username));
        }
    
        let profile = query.one(&state.db).await?;

        if profile.is_none() {
            return Err(WebError::NotFound("Couldn't find user matching id or username".to_string()));
        }
        let profile = profile.unwrap();

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
        res.set_body(serde_json::to_string(&arr)?.into_bytes());

        Ok(res)
    } else if req.matches_path_exact(&["api", "agent"]) {
        info!("Querying agent!");
        let agent_id: i32 = req.path.parse_query("agent")?;

        let mut send_error: bool = req.path.parse_query("error").unwrap_or(false);
        let mut send_src: bool = req.path.parse_query("src").unwrap_or(false);

        let agent = agent::Entity::find_by_id(agent_id).one(&state.db).await?;

        if agent.is_none() {
            return Err(WebError::NotFound("Agent not found".to_string()));
        }
        let agent: agent::Model = agent.unwrap();

        if let Some(owner_id) = agent.owner_id {
            if let Some(owner) = user::Entity::find_by_id(owner_id).one(&state.db).await? {
                if !is_user_authenticated(&req, &owner) && !authenticate_admin(&req, &state) {
                    send_error = false;
                    send_src = false;
                }
            }
        }

        let data = get_agent_data_as_json(&agent, send_error, send_src, &state.db).await?;

        let mut res = Response::new();
        res.set_status(Status::Ok);
        res.set_header("Content-Type", "application/json");
        res.set_body(serde_json::to_string(&data)?.into_bytes());

        Ok(res)
    } else if req.matches_path_exact(&["api", "list_files"]) {
        let mut result = Map::new();

        for (lang, files) in &state.executor.languages {
            let mut files_json = vec![];

            for (name, file) in &files.files {
                if !file.hidden {
                    files_json.push(json!({
                        "name": name,
                        "description": file.description,
                        "display": file.download_name
                    }));
                }
            }
            
            result.insert(lang.name().to_string(), json!(files_json));
        }

        let mut res = Response::new();
        res.set_status(Status::Ok);
        res.set_header("Content-Type", "application/json");
        res.set_body(serde_json::to_string(&result)?.into_bytes());

        Ok(res)
    } else if req.matches_path_exact(&["api", "stats"]) {
        let id = match req.path.query.get("id") {
            Some(id) => {
                match id.parse::<i32>() {
                    Ok(id) => Some(id),
                    Err(_) => None
                }
            }
            None => None
        };
    
        let username = req.path.query.get("username");
    
        if username.is_none() && id.is_none() {
            return Err(WebError::InvalidData("Could not find desired user (Missing parameters)".to_string()));
        }
    
        let mut query = user::Entity::find();
    
        if let Some(id) = id {
            query = query.filter(user::Column::Id.eq(id));
        }
    
        if let Some(username) = username {
            query = query.filter(user::Column::Username.eq(username));
        }
    
        let profile = query.one(&state.db).await?;

        if profile.is_none() {
            return Err(WebError::NotFound("Couldn't find user matching id or username".to_string()));
        }
        let profile = profile.unwrap();

        let mut data = Map::new();

        let agents = profile.find_related(agent::Entity).all(&state.db).await?;
        let best_rating = agents.iter().map(|x| x.rating).max_by(|a, b| a.total_cmp(b)).unwrap_or(0.0);

        let active_agents = agents.iter().filter(|x| !x.partial && !x.removed).count();

        let total_games_played: i32 = agents.iter().map(|x| x.num_games).sum();

        data.insert("best_rating".to_string(), json!(best_rating as i32));
        data.insert("active_agents".to_string(), json!(active_agents));
        data.insert("total_games".to_string(), json!(total_games_played));

        let mut response = Response::new();
        response.set_status(Status::Ok);
        response.set_header("Content-Type", "application/json");
        response.set_body(serde_json::to_string(&data)?.into_bytes());

        Ok(response)
    } else if req.matches_path(&["client_files"]) && req.path.path.len() == 3 {
        let target_lang = urlencoding::decode(&req.path.path[1])?.to_string();
        let file = urlencoding::decode(&req.path.path[2])?.to_string();

        debug!("Client wants file {} {}", target_lang, file);

        for (lang, files) in &state.executor.languages {
            if lang.name() == target_lang {
                let res = match files.files.get(&file) {
                    Some(x) => x,
                    None => return Err(WebError::NotFound("Requested client file not found".to_string()))
                };

                let mut response = Response::new();
                response.set_status(Status::Ok);
                response.set_header("Content-Type", "application/octet-stream");
                response.set_header("Content-Disposition", &format!("attachment; filename=\"{}\"", res.download_name));
                response.set_body(res.content.clone().into_bytes());

                return Ok(response);
            }
        }

        Err(WebError::NotFound("Requested language not found".to_string()))
    } else {
        Err(WebError::NotFound("Route not found".to_string()))
    }
}

async fn reset_password(req: &Request, state: &AppState) -> HttpResult<Response> {
    let user = get_user(req, state).await?;

    if user.is_none() {
        return Err(WebError::NotFound("Couldn't find user with id".to_string()));
    }

    let user = user.unwrap();

    let mut active: entities::user::ActiveModel = user.into();
    active.password = ActiveValue::Set(generate_password());
    let user = active.update(&state.db).await?;

    let mut res = Response::new();
    res.set_status(Status::Ok);
    res.set_header("Content-Type", "text/plain");
    res.set_body(user.password.clone().into_bytes());

    Ok(res)
}

async fn route_post(_addr: SocketAddr, req: Request, state: AppState) -> HttpResult<Response> {
    if req.matches_path(&["admin"]) {
        if !authenticate_admin(&req, &state) {
            Err(WebError::Unauthorized)
        } else if req.matches_path_exact(&["admin", "new_profile"]) {
            let username = &req.path.get("username")?.replace("\n", "");

            if username.len() > 50 {
                return Err(WebError::InvalidData("username too long".to_string()));
            }

            let other = entities::prelude::User::find()
                .filter(user::Column::Username.eq(username))
                .one(&state.db)
                .await?;

            //Check if username is already taken
            if other.is_some() {
                return Err(WebError::InvalidData("already taken".to_string()));
            }

            let num_agents_allowed = req.path.parse_query("agents")?;

            let profile = user::ActiveModel {
                username: ActiveValue::Set(username.clone()),
                password: ActiveValue::Set(generate_password()),
                num_agents_allowed: ActiveValue::Set(num_agents_allowed),
                ..Default::default()
            };

            user::Entity::insert(profile).exec(&state.db).await?;

            let mut res = Response::new();
            res.set_status(Status::Ok);

            Ok(res)
        } else if req.matches_path_exact(&["admin", "delete_profile"]) {
            let profile = get_user(&req, &state).await?;
            if let Some(profile) = profile {
                info!("Deleting profile: id: {:?}, username: {:?}", profile.id, profile.username);

                agent::Entity::delete_many()
                    .filter(agent::Column::OwnerId.eq(profile.id))
                    .exec(&state.db)
                    .await?;

                let profile: user::ActiveModel = profile.into();
                user::Entity::delete(profile).exec(&state.db).await?;

                let mut res = Response::new();
                res.set_status(Status::Ok);
                    
                Ok(res)
            } else {
                Err(WebError::NotFound("User id not found".to_string()))
            }
        } else if req.matches_path_exact(&["admin", "set_profile_agents"]) {
            let profile = get_user(&req, &state).await?;
            let num_agents_allowed = req.path.parse_query("agents")?;
            if let Some(profile) = profile {
                let mut profile: user::ActiveModel = profile.into();
                profile.num_agents_allowed = ActiveValue::Set(num_agents_allowed);
                profile.update(&state.db).await?;

                let mut res = Response::new();
                res.set_status(Status::Ok);
                    
                Ok(res)
            } else {
                Err(WebError::NotFound("User id not found".to_string()))
            }
        } else if req.matches_path_exact(&["admin", "full_reset"]) {
            warn!("Doing full reset!");

            entities::agent::Entity::delete_many().exec(&state.db).await?;
            entities::user::Entity::delete_many().exec(&state.db).await?;

            let mut res = Response::new();
            res.set_status(Status::Ok);
            Ok(res)
        } else if req.matches_path_exact(&["admin", "agents_reset"]) {
            warn!("Deleting all agents!");

            entities::agent::Entity::delete_many().exec(&state.db).await?;

            let mut res = Response::new();
            res.set_status(Status::Ok);
            Ok(res)
        } else if req.matches_path_exact(&["admin", "ratings_reset"]) {
            warn!("Resetting rating!");

            let active = agent::ActiveModel {
                rating: ActiveValue::Set(1000.0),
                ..Default::default()
            };

            entities::agent::Entity::update_many()
                .set(active)
                .exec(&state.db)
                .await?;

            let mut res = Response::new();
            res.set_status(Status::Ok);
            Ok(res)
        } else if req.matches_path_exact(&["admin", "file_cleanup"]) {
            cleanup_files(&state.db).await;

            let mut res = Response::new();
            res.set_status(Status::Ok);
            Ok(res)
        } else {
            Err(WebError::NotFound("Admin route not found".to_string()))
        }
    } else if req.matches_path_exact(&["api", "reset_password"]) {
        reset_password(&req, &state).await
    } else if req.matches_path_exact(&["api", "add_agent"]) {
        let profile = get_user(&req, &state).await?;

        if profile.is_none() {
            return Err(WebError::NotFound("User id not found".to_string()));
        }
        let profile = profile.unwrap();

        if !is_user_authenticated(&req, &profile) {
            return Err(WebError::Unauthorized);
        }

        let num_agents = get_num_agents(&profile, &state.db).await;

        if num_agents >= profile.num_agents_allowed as _ {
            return Err(WebError::InvalidData(format!("You have already used {} out of your {} available agent slot(s)! You can delete some of your agents to free these up!", num_agents, profile.num_agents_allowed)));
        }

        let data = decode_utf8(req.body.clone())?;

        let data = parse_json_as_object(&data)?;

        let src = data.try_get("src")?.try_as_str()?.to_string();
        let language_id = data.try_get("lang")?.try_as_str()?;
        let name = data.try_get("name")?.try_as_str()?;

        if src.len() > 30000 {
            return Err(WebError::InvalidData(format!("Source code too long!")));
        }

        let language = state.languages.iter().filter(|l| l.id() == language_id).next();
        let language = match language {
            Some(l) => l,
            None => return Err(WebError::InvalidData(format!("Unknown language {}", language_id)))
        }.clone();

        let in_use = agent::Entity::find()
            .filter(agent::Column::Name.eq(name))
            .one(&state.db)
            .await?.is_some();

        if in_use {
            return Err(WebError::InvalidData(format!("Agent name already used!")));
        }

        let mut program = PreparedProgram::new();
        let src_file = random_file(RUN_DIR, ".src");

        async_std::fs::write(&src_file, &src).await?;

        let id = state.executor.add_player(
            name.to_string(), 
            language_id.to_string(), 
            program.dir_as_string(),
            Some(src_file),
            Some(profile.id),
            true
        ).await?;

        let mut res = Response::new();
        res.set_status(Status::Ok);
        res.set_header("Content-Type", "application/json");
        res.set_body(serde_json::to_string(&json!({
            "agent_id": id
        }))?.into_bytes());

        let itf = state.executor.itf.clone();
        let db = state.db.clone();
        async_std::task::spawn(async move {
            let result = language.prepare(&src, &mut program, &itf, state.executor.sandboxes.clone()).await;
            let mut agent: agent::ActiveModel = match agent::Entity::find_by_id(id).one(&db).await {
                Ok(Some(x)) => x,
                Ok(None) => {
                    error!("Couldn't find agent that needed to be compiled!");
                    return;
                },
                Err(e) => {
                    error!("Encountered error while finding agent that needed to be compiled! {}", e);
                    return;
                }
            }.into();

            match result {
                Ok(()) => {
                    agent.partial = ActiveValue::Set(false)
                },
                Err(e) => {
                    let error_file = random_file(RUN_DIR, ".compile-error");
                    
                    if let Err(e) = async_std::fs::write(&error_file, e).await {
                        error!("Encountered error while writing compile error! {}", e);
                    }

                    agent.removed = ActiveValue::Set(true);
                    agent.error_file = ActiveValue::Set(Some(error_file));
                }
            }

            if let Err(e) = agent.update(&db).await {
                error!("Encountered error while updating agent! {}", e);
            }
        });

        Ok(res)
    } else if req.matches_path_exact(&["api", "set_colour"]) {
        let profile = get_user(&req, &state).await?;

        if profile.is_none() {
            return Err(WebError::InvalidData("User id not found".to_string()));
        }
        let profile = profile.unwrap();

        if !is_user_authenticated(&req, &profile) {
            return Err(WebError::Unauthorized);
        }

        let agent_id: i32 = req.path.parse_query("agent")?;

        let r: u8 = req.path.parse_query("r")?;
        let g: u8 = req.path.parse_query("g")?;
        let b: u8 = req.path.parse_query("b")?;

        let agent = agent::Entity::find_by_id(agent_id).one(&state.db).await?;

        if agent.is_none() {
            return Err(WebError::NotFound("Agent not found".to_string()));
        }

        let color = format!("#{:02X}{:02X}{:02X}", r, g, b);
        println!("Color = {}", color);

        let mut active: agent::ActiveModel = agent.unwrap().into();

        active.colour = ActiveValue::Set(color);
        active.update(&state.db).await?;

        let mut res = Response::new();
        res.set_status(Status::Ok);

        Ok(res)
    } else if req.matches_path_exact(&["api", "delete_agent"]) {
        let profile = get_user(&req, &state).await?;

        if profile.is_none() {
            return Err(WebError::NotFound("Profile not found".to_string()));
        }
        let profile = profile.unwrap();

        if !is_user_authenticated(&req, &profile) {
            return Err(WebError::Unauthorized);
        }

        let agent_id: i32 = req.path.parse_query("agent")?;

        let agent = agent::Entity::find_by_id(agent_id).one(&state.db).await?;

        if agent.is_none() {
            return Err(WebError::NotFound("Agent not found".to_string()));
        }

        agent.unwrap().delete(&state.db).await?;

        let mut res = Response::new();
        res.set_status(Status::Ok);

        Ok(res)
    } else {
        Err(WebError::NotFound("Route not found".to_string()))
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

    if request.method == Method::Get && request.matches_path_exact(&["bruh"]) {
        let mut inner = state.reporter.lock().await;

        inner.handle_stream(stream, &request).await;
    } else {
        let result = match request.method {
            Method::Get => route_get(addr, request, state).await,
            Method::Post => route_post(addr, request, state).await,

            _ => Err(WebError::InvalidMethod)
        };

        match result {
            Ok(res) => res.write_async(&mut stream).await.ignore(),
            Err(res) => {
                let response = res.into_response();
                info!("Request [{}] was unsuccesful ({})", addr, response.status);
                response.write_async(&mut stream).await.ignore();
            }
        }
    }
}

fn generate_admin_password() -> String {
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

pub async fn launch_and_run_api(executor: Arc<GameRunner<Box<dyn Game>>>, reporter: Arc<Mutex<SharedInner>>, db: DatabaseConnection) -> std::io::Result<()> {
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));

    let listener = TcpListener::bind(addr).await?;

    let state = AppState {
        executor,
        super_secret_admin_password: generate_admin_password(),
        languages: Arc::new(get_all_languages()),
        reporter,
        db,
        page_engine: PageEngine::load()
    };

    println!("Admin password: {}", state.super_secret_admin_password);
    std::fs::write("./admin_password.txt", state.super_secret_admin_password.as_bytes()).unwrap();

    println!("Listening on {}", addr);

    loop {
        let (stream, addr) = listener.accept().await?;

        async_std::task::spawn(handle_conn(stream, addr, state.clone()));
    }
}
