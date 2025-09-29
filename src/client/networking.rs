use crate::common::{
    Database, LikeBody, LikedState, LoginPacket, NetworkPacket, StyleBody, StyleVariant,
};
use anyhow::{Result, anyhow};
use bincode::serde::{decode_from_slice, encode_to_vec};
use ehttp::{Request, Response, fetch};
use uuid::Uuid;

/// A single “send a request” helper.
fn send<T, R>(
    host: &str,
    endpoint: &str,
    payload: Option<T>,
    on_resp: impl FnOnce(Response) -> Result<R> + 'static + Send,
    on_done: impl FnOnce(Result<R>) + 'static + Send,
) where
    T: serde::Serialize + 'static,
    R: 'static,
{
    // Build either GET or POST
    let url = format!("http://{host}/{endpoint}");
    let req = payload.map_or_else(
        || Request::get(&url),
        |body| {
            let bytes =
                encode_to_vec(&body, bincode::config::standard()).expect("serialize must not fail");
            Request::post(&url, bytes)
        },
    );

    // Fire off the request
    fetch(
        req,
        Box::new(move |res: Result<Response, String>| {
            let result: Result<R> = match res {
                Ok(resp) if resp.status == 200 => on_resp(resp),
                Ok(resp) => Err(anyhow!(
                    "Bad status {}: {}",
                    resp.status,
                    resp.text().unwrap_or_default()
                )),
                Err(e) => Err(anyhow!("Network error: {e}")),
            };
            on_done(result);
        }),
    );
}

/// POST a `NetworkPacket` and ignore the body (unit result).
fn post_unit_data<D>(
    host: &str,
    endpoint: &str,
    token: &str,
    data: D,
    on_done: impl 'static + Send + FnOnce(Result<()>),
) where
    D: serde::Serialize + 'static,
{
    let pkt = NetworkPacket {
        token: token.to_owned(),
        data,
    };
    send(host, endpoint, Some(pkt), |_| Ok(()), on_done);
}

pub fn login(
    host: &str,
    username: &str,
    password: &str,
    on_done: impl 'static + Send + FnOnce(Result<String>),
) {
    send(
        host,
        "login",
        Some(LoginPacket {
            username: username.to_string(),
            password: password.to_string(),
        }),
        |resp| {
            resp.text()
                .map(ToString::to_string)
                .ok_or_else(|| anyhow!("Failed to extract text"))
        },
        on_done,
    );
}

pub fn query_prompt(
    host: &str,
    token: &str,
    on_done: impl 'static + Send + FnOnce(Result<String>),
) {
    send(
        host,
        "queryprompt",
        Some(NetworkPacket {
            token: token.to_string(),
            data: (),
        }),
        |resp| {
            resp.text()
                .map(ToString::to_string)
                .ok_or_else(|| anyhow!("Failed to extract text"))
        },
        on_done,
    );
}

pub fn generate_wallpaper(
    host: &str,
    token: &str,
    message: &str,
    on_done: impl 'static + Send + FnOnce(Result<()>),
) {
    post_unit_data(host, "generate", token, message.to_owned(), on_done);
}

pub fn add_comment(
    host: &str,
    token: &str,
    comment: &str,
    on_done: impl 'static + Send + FnOnce(Result<()>),
) {
    post_unit_data(host, "commentadd", token, comment.to_owned(), on_done);
}

pub fn remove_comment(
    host: &str,
    token: &str,
    comment_id: &Uuid,
    on_done: impl 'static + Send + FnOnce(Result<()>),
) {
    post_unit_data(host, "commentremove", token, comment_id.to_owned(), on_done);
}

pub fn like_image(
    host: &str,
    token: &str,
    image_id: &Uuid,
    liked: LikedState,
    on_done: impl 'static + Send + FnOnce(Result<()>),
) {
    let packet = LikeBody {
        uuid: *image_id,
        liked,
    };
    post_unit_data(host, "imageliked", token, packet, on_done);
}

pub fn remove_image(
    host: &str,
    token: &str,
    image_id: &Uuid,
    on_done: impl 'static + Send + FnOnce(Result<()>),
) {
    post_unit_data(host, "imageremove", token, image_id.to_owned(), on_done);
}

pub fn recreate_image(
    host: &str,
    token: &str,
    image_id: &Uuid,
    on_done: impl 'static + Send + FnOnce(Result<()>),
) {
    post_unit_data(host, "imagerecreate", token, image_id.to_owned(), on_done);
}

pub fn edit_styles(
    host: &str,
    token: &str,
    variant: StyleVariant,
    new: &str,
    on_done: impl 'static + Send + FnOnce(Result<()>),
) {
    let packet = StyleBody {
        variant,
        string: new.to_string(),
    };
    post_unit_data(host, "styles", token, packet, on_done);
}

pub fn get_database(host: &str, on_done: impl 'static + Send + FnOnce(Result<Database>)) {
    send::<(), Database>(
        host,
        "get",
        None,
        |resp| {
            decode_from_slice::<Database, _>(&resp.bytes, bincode::config::standard())
                .map(|(db, _)| db)
                .map_err(|_| anyhow!("Failed to decode database"))
        },
        on_done,
    );
}
