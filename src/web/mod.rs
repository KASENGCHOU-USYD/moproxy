use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use ::rustful::{Server, Context, Response, TreeRouter};
use ::rustful::StatusCode::{NotFound, InternalServerError};
use ::rustful::header::ContentType;
use ::serde_json;
use ::monitor::{ServerList, ServerInfo};
use ::proxy::ProxyServer;

#[derive(Debug, Serialize)]
struct ServersJson<'a> {
    servers: &'a [ProxyServer],
    infos: &'a [ServerInfo],
}

fn index(_context: Context, response: Response) {
    response.send(include_str!("index.html"));
}

fn not_found(_context: Context, mut response: Response) {
    response.set_status(NotFound);
    response.send("page not found");
}

fn version(_context: Context, response: Response) {
    response.send(env!("CARGO_PKG_VERSION"));
}

fn servers_json(context: Context, mut response: Response) {
    let json_type = ContentType(content_type!(Text / Json; Charset = Utf8));
    response.headers_mut().set(json_type);
    let list: &Arc<ServerList> = context.global.get()
        .expect("not servers found in global");
    let servers = ServersJson {
        servers: &list.servers,
        infos: &list.get_infos(),
    };
    let resp = match serde_json::to_string(&servers) {
        Ok(json) => json,
        Err(e) => {
            response.set_status(InternalServerError);
            error!("fail to serialize servers to json: {}", e);
            format!("internal error: {}", e)
        },
    };
    response.send(resp);
}

pub fn run_server(bind: SocketAddr, servers: Arc<ServerList>) {
    let router = insert_routes! {
        TreeRouter::new() => {
            "/" => Get: index as fn(Context, Response),
            "/servers" => Get: servers_json,
            "/version" => Get: version,
            "/*" => Get: not_found,
        }
    };
    let server_result = Server {
        handlers: router,
        host: bind.into(),
        global: Box::new(servers).into(),
        threads: Some(1),
        ..Server::default()
    }.run();
    match server_result {
        Ok(_) => (),
        Err(e) => error!("could not start server: {}", e.description())
    };
}
