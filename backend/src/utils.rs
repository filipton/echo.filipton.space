use anyhow::Result;
use hyper::{Body, Request};
use std::{collections::HashMap, net::SocketAddr};

pub async fn request_to_raw_http(req: Request<Body>, client_ip: &SocketAddr) -> Result<String> {
    let mut raw = format!(
        "{} {} {:?}\r\n",
        req.method(),
        req.uri().path_and_query().unwrap(),
        req.version()
    );

    raw.push_str(&format!("{}: {}\r\n", "X-Real-IP", client_ip.ip()));
    for (name, value) in req.headers() {
        raw.push_str(&format!("{}: {}\r\n", name, value.to_str()?));
    }

    let body = hyper::body::to_bytes(req.into_body()).await;
    raw.push_str(&format!("\r\n{}", String::from_utf8(body?.to_vec())?));

    Ok(raw)
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
