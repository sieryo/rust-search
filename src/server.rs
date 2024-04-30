use std::{fs::File, io, str::from_utf8};

use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

use crate::model::{self, Model};

fn serve_static_file(request: Request, file_path: &str, content_type: &str) -> io::Result<()> {
    let header_content_type = Header::from_bytes("Content-Type", content_type).expect("Error wir");
    let file = File::open(file_path)?;

    let response = Response::from_file(file).with_header(header_content_type);
    request
        .respond(response)
}

fn serve_404(request: Request) -> io::Result<()> {
    request
        .respond(Response::from_string("404").with_status_code(StatusCode(404)))
}

fn serve_api_search(mut request:Request, model: &Model) -> io::Result<()> {
    let mut buf = Vec::new();

    if let Err(err) = request.as_reader().read_to_end(&mut buf) {
        eprintln!("ERROR: Tidak bisa membaca body request : {err}");
        return serve_404(request);
    }

    let body  = match from_utf8(&buf) {
        Ok(body) => body.to_lowercase().chars().collect::<Vec<_>>(),
        Err(err) => {
            eprintln!("ERROR: tidak bisa membaca body sebagai utf-8 strings : {err}");
            return serve_404(request);
        }
    };
    let result = model::search_query(&model, &body);

    let mut respond: Vec<&str> = Vec::new();
    for (path, _) in result.iter().take(10) {
        respond.push(path.to_str().unwrap());
    }
    let respond = match  serde_json::to_string(&respond) {
        Ok(data) => { data},
        Err(err) => {
            eprintln!("ERROR: Tidak dapat mengirim respond : {err}");
            return serve_404(request);
        }
    };
    request.respond(Response::from_string(respond))
}

fn serve_request(model: &Model, request: Request) -> io::Result<()> {
    println!(
        "INFO: mendapatkan request. method: {:?}, url: {:?}",
        request.method(),
        request.url()
    );

    match (request.method(), request.url()) {
        (Method::Post, "/api/search") => {
            serve_api_search(request, model)        }
        (Method::Get, "/") | (Method::Get, "/index.html") => {
            serve_static_file(request, "index.html", "text/html; charset=utf-8")
        }
        (Method::Get, "/index.js") => {
            serve_static_file(request, "index.js", "text/javascript; charset=utf-8")
        }
        _ => serve_404(request),
    }
}

pub fn begin_server(model: &Model) -> Result<(), ()> {
    let server = Server::http("127.0.0.1:3000").map_err(|_| {
        eprintln!("ERROR: tidak bisa menjalankan http server..");
    })?;
        
    println!("INFO: listening at http://127.0.0.1:3000");

    for request in server.incoming_requests() {
        serve_request(model, request).map_err(|_| {
            eprintln!("ERROR: tidak bisa menyediakan response");
        }).ok();
    }
    Ok(())
}
