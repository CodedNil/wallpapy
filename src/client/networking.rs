use crate::common::{
    Database, LikeBody, LikedState, LoginPacket, NetworkPacket, StyleBody, StyleVariant,
};
use anyhow::Result;
use bincode::serde::{decode_from_slice, encode_to_vec};
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
            encode_to_vec(
                LoginPacket {
                    username: username.to_string(),
                    password: password.to_string(),
                },
                bincode::config::standard(),
            )
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
            encode_to_vec(
                &NetworkPacket {
                    token: token.to_string(),
                    data: message.to_string(),
                },
                bincode::config::standard(),
            )
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
                        decode_from_slice(&res.bytes, bincode::config::standard())
                            .map(|(database, _)| database)
                            .map_err(|_| anyhow::anyhow!("Failed to load database"))
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
            encode_to_vec(
                &NetworkPacket {
                    token: token.to_string(),
                    data: comment.to_string(),
                },
                bincode::config::standard(),
            )
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
            encode_to_vec(
                &NetworkPacket {
                    token: token.to_string(),
                    data: *comment_id,
                },
                bincode::config::standard(),
            )
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
            encode_to_vec(
                &NetworkPacket {
                    token: token.to_string(),
                    data: LikeBody {
                        uuid: *image_id,
                        liked,
                    },
                },
                bincode::config::standard(),
            )
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
            encode_to_vec(
                &NetworkPacket {
                    token: token.to_string(),
                    data: *image_id,
                },
                bincode::config::standard(),
            )
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
            encode_to_vec(
                &NetworkPacket {
                    token: token.to_string(),
                    data: *image_id,
                },
                bincode::config::standard(),
            )
            .unwrap(),
        ),
        Box::new(move |_| {
            on_done(Ok(()));
        }),
    );
}

pub fn edit_styles(
    host: &str,
    token: &str,
    variant: StyleVariant,
    new: &str,
    on_done: impl 'static + Send + FnOnce(Result<()>),
) {
    ehttp::fetch(
        ehttp::Request::post(
            format!("http://{host}/styles"),
            encode_to_vec(
                &NetworkPacket {
                    token: token.to_string(),
                    data: StyleBody {
                        variant,
                        string: new.to_string(),
                    },
                },
                bincode::config::standard(),
            )
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
    on_done: impl 'static + Send + FnOnce(Result<String>),
) {
    ehttp::fetch(
        ehttp::Request::post(
            format!("http://{host}/queryprompt"),
            encode_to_vec(
                &NetworkPacket {
                    token: token.to_string(),
                    data: (),
                },
                bincode::config::standard(),
            )
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
