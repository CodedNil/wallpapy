use crate::common::{LoginPacket, TokenPacket, WallpaperData};
use anyhow::Result;

pub fn login(
    host: &str,
    username: &str,
    password: &str,
    on_done: impl 'static + Send + FnOnce(Result<String>),
) {
    ehttp::fetch(
        ehttp::Request::post(
            &format!("http://{host}/login"),
            bincode::serialize(&LoginPacket {
                username: username.to_string(),
                password: password.to_string(),
            })
            .unwrap(),
        ),
        Box::new(move |res: std::result::Result<ehttp::Response, String>| {
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
    on_done: impl 'static + Send + FnOnce(Result<()>),
) {
    ehttp::fetch(
        ehttp::Request::post(
            &format!("http://{host}/generate"),
            bincode::serialize(&TokenPacket {
                token: token.to_string(),
            })
            .unwrap(),
        ),
        Box::new(move |_| {
            on_done(Ok(()));
        }),
    );
}

pub fn get_gallery(host: &str, on_done: impl 'static + Send + FnOnce(Result<Vec<WallpaperData>>)) {
    ehttp::fetch(
        ehttp::Request::get(&format!("http://{host}/get")),
        Box::new(move |res: std::result::Result<ehttp::Response, String>| {
            on_done(match res {
                Ok(res) => {
                    if res.status == 200 {
                        bincode::deserialize(&res.bytes)
                            .map_or_else(|_| Err(anyhow::anyhow!("Failed to load gallery")), Ok)
                    } else {
                        Err(anyhow::anyhow!(
                            "Failed to load gallery, status code: {}",
                            res.status
                        ))
                    }
                }
                Err(e) => Err(anyhow::anyhow!("Network error loading gallery: {}", e)),
            });
        }),
    );
}
