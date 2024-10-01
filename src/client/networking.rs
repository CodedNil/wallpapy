use crate::common::{
    Database, LikedState, LoginPacket, TokenStringPacket, TokenUuidLikedPacket, TokenUuidPacket,
};
use anyhow::Result;
use uuid::Uuid;

pub fn login(
    host: &str,
    username: &str,
    password: &str,
    on_done: impl 'static + Send + FnOnce(Result<String>),
) {
    ehttp::fetch(
        ehttp::Request::post(
            format!("http://{host}/login"),
            bincode::serialize(&LoginPacket {
                username: username.to_string(),
                password: password.to_string(),
            })
            .unwrap(),
        ),
        Box::new(move |res: Result<ehttp::Response, String>| {
            on_done(match res {
                Ok(res) => {
                    if res.status == 200 {
                        res.text()
                            .map(std::string::ToString::to_string)
                            .ok_or_else(|| anyhow::anyhow!("Failed to extract text from response"))
                    } else {
                        Err(anyhow::anyhow!(
                            "Login failed: {}",
                            res.text().unwrap_or_default()
                        ))
                    }
                }
                Err(e) => Err(anyhow::anyhow!("Failed to login: {}", e)),
            });
        }),
    );
}

pub fn generate_wallpaper(
    host: &str,
    token: &str,
    message: &str,
    on_done: impl 'static + Send + FnOnce(Result<()>),
) {
    ehttp::fetch(
        ehttp::Request::post(
            format!("http://{host}/generate"),
            bincode::serialize(&TokenStringPacket {
                token: token.to_string(),
                string: message.to_string(),
            })
            .unwrap(),
        ),
        Box::new(move |_| {
            on_done(Ok(()));
        }),
    );
}

pub fn get_database(host: &str, on_done: impl 'static + Send + FnOnce(Result<Database>)) {
    ehttp::fetch(
        ehttp::Request::get(format!("http://{host}/get")),
        Box::new(move |res: Result<ehttp::Response, String>| {
            on_done(match res {
                Ok(res) => {
                    if res.status == 200 {
                        bincode::deserialize(&res.bytes)
                            .map_or_else(|_| Err(anyhow::anyhow!("Failed to load database")), Ok)
                    } else {
                        Err(anyhow::anyhow!(
                            "Failed to load database, status code: {}",
                            res.status
                        ))
                    }
                }
                Err(e) => Err(anyhow::anyhow!("Network error loading database: {}", e)),
            });
        }),
    );
}

pub fn add_comment(
    host: &str,
    token: &str,
    comment: &str,
    on_done: impl 'static + Send + FnOnce(Result<()>),
) {
    ehttp::fetch(
        ehttp::Request::post(
            format!("http://{host}/commentadd"),
            bincode::serialize(&TokenStringPacket {
                token: token.to_string(),
                string: comment.to_string(),
            })
            .unwrap(),
        ),
        Box::new(move |_| {
            on_done(Ok(()));
        }),
    );
}

pub fn remove_comment(
    host: &str,
    token: &str,
    comment_id: &Uuid,
    on_done: impl 'static + Send + FnOnce(Result<()>),
) {
    ehttp::fetch(
        ehttp::Request::post(
            format!("http://{host}/commentremove"),
            bincode::serialize(&TokenUuidPacket {
                token: token.to_string(),
                uuid: *comment_id,
            })
            .unwrap(),
        ),
        Box::new(move |_| {
            on_done(Ok(()));
        }),
    );
}

pub fn like_image(
    host: &str,
    token: &str,
    image_id: &Uuid,
    liked: LikedState,
    on_done: impl 'static + Send + FnOnce(Result<()>),
) {
    ehttp::fetch(
        ehttp::Request::post(
            format!("http://{host}/imageliked"),
            bincode::serialize(&TokenUuidLikedPacket {
                token: token.to_string(),
                uuid: *image_id,
                liked,
            })
            .unwrap(),
        ),
        Box::new(move |_| {
            on_done(Ok(()));
        }),
    );
}

pub fn remove_image(
    host: &str,
    token: &str,
    image_id: &Uuid,
    on_done: impl 'static + Send + FnOnce(Result<()>),
) {
    ehttp::fetch(
        ehttp::Request::post(
            format!("http://{host}/imageremove"),
            bincode::serialize(&TokenUuidPacket {
                token: token.to_string(),
                uuid: *image_id,
            })
            .unwrap(),
        ),
        Box::new(move |_| {
            on_done(Ok(()));
        }),
    );
}

pub fn recreate_image(
    host: &str,
    token: &str,
    image_id: &Uuid,
    on_done: impl 'static + Send + FnOnce(Result<()>),
) {
    ehttp::fetch(
        ehttp::Request::post(
            format!("http://{host}/imagerecreate"),
            bincode::serialize(&TokenUuidPacket {
                token: token.to_string(),
                uuid: *image_id,
            })
            .unwrap(),
        ),
        Box::new(move |_| {
            on_done(Ok(()));
        }),
    );
}

pub fn edit_key_style(
    host: &str,
    token: &str,
    new: &str,
    on_done: impl 'static + Send + FnOnce(Result<()>),
) {
    ehttp::fetch(
        ehttp::Request::post(
            format!("http://{host}/keystyle"),
            bincode::serialize(&TokenStringPacket {
                token: token.to_string(),
                string: new.to_string(),
            })
            .unwrap(),
        ),
        Box::new(move |_| {
            on_done(Ok(()));
        }),
    );
}

pub fn query_prompt(
    host: &str,
    token: &str,
    message: &str,
    on_done: impl 'static + Send + FnOnce(Result<String>),
) {
    ehttp::fetch(
        ehttp::Request::post(
            format!("http://{host}/queryprompt"),
            bincode::serialize(&TokenStringPacket {
                token: token.to_string(),
                string: message.to_string(),
            })
            .unwrap(),
        ),
        Box::new(move |res: Result<ehttp::Response, String>| {
            on_done(match res {
                Ok(res) => {
                    if res.status == 200 {
                        res.text()
                            .map(std::string::ToString::to_string)
                            .ok_or_else(|| anyhow::anyhow!("Failed to extract text from response"))
                    } else {
                        Err(anyhow::anyhow!(
                            "Querying prompt failed {}",
                            res.text().unwrap_or_default()
                        ))
                    }
                }
                Err(e) => Err(anyhow::anyhow!("Querying prompt failed {}", e)),
            });
        }),
    );
}
