use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use anyhow::Result;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

pub type SharedState = Arc<RwLock<State>>;

pub struct State {
    pub files: HashMap<String, Vec<u8>>,
}

#[get("{filename:.*}")]
async fn handle_file(filename: web::Path<String>, data: web::Data<SharedState>) -> impl Responder {
    let state = data.read().await;
    let filename = format!("/{}", filename);

    let mut is_html = false;

    let mut file = state.files.get(&filename);
    if file.is_none() {
        file = state
            .files
            .get(&format!("{}/index.html", filename.trim_end_matches('/')));

        if file.is_some() {
            is_html = true;
        }
    }

    if let Some(file) = file {
        let content_type = if is_html {
            "text/html".to_string()
        } else {
            mime_guess::from_path(filename)
                .first_or_text_plain()
                .to_string()
        };

        return HttpResponse::Ok()
            .content_type(content_type)
            .body(file.to_owned());
    }

    return HttpResponse::NotFound().body("404 Not Found");
}

#[tokio::main]
async fn main() -> Result<()> {
    let state = Arc::new(RwLock::new(State {
        files: read_static_files("/")?,
    }));
    println!("Loaded {} static files!", state.read().await.files.len());

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .service(handle_file)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await?;

    Ok(())
}

pub fn read_static_files(prefix: &str) -> Result<HashMap<String, Vec<u8>>> {
    let mut tmp_files = HashMap::new();
    let mut dir_contents = std::fs::read_dir(format!("./public{}", prefix))?;

    while let Some(Ok(entry)) = dir_contents.next() {
        let file_name = entry.file_name().into_string().unwrap();

        if entry.file_type()?.is_dir() {
            let mut sub_files = read_static_files(&format!("{}{}/", prefix, file_name))?;
            tmp_files.extend(sub_files.drain());

            continue;
        }

        let file_contents = std::fs::read(entry.path())?;
        tmp_files.insert(format!("{}{}", prefix, file_name), file_contents);
    }

    Ok(tmp_files)
}
