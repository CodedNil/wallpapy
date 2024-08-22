use super::super::PORT;
use super::networking::{generate_wallpaper, login};
use crate::{client::networking::get_gallery, common::WallpaperData};
use anyhow::Result;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use egui::{
    vec2, Align2, CentralPanel, Color32, Context, FontId, Frame, ScrollArea, Shape, TextEdit, Vec2,
    Window,
};
use egui_notify::Toasts;
use egui_pull_to_refresh::PullToRefresh;
use egui_thumbhash::ThumbhashImage;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};

nestify::nest! {
    pub struct Wallpapy {
        host: String,
        toasts: Arc<Mutex<Toasts>>,

        gallery: Option<(Vec<WallpaperData>, DateTime<Utc>)>,

        #>[derive(Deserialize, Serialize, Default)]
        #>[serde(default)]
        stored: pub struct StoredData {
            auth_token: String,
        },

        login_form: struct LoginForm {
            username: String,
            password: String,
        },

        #>[derive(Default)]*
        network_data: Arc<Mutex<struct DownloadData {
            login: enum LoginState {
                #[default]
                None,
                InProgress,
                Done(Result<String>),
            },
            get_gallery: enum GetGalleryState {
                #[default]
                None,
                InProgress,
                Done(Result<Vec<WallpaperData>>),
            },
        }>>,
    }
}

impl Wallpapy {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let stored = cc.storage.map_or_else(StoredData::default, |storage| {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        });

        Self {
            host: format!("localhost:{PORT}"),
            toasts: Arc::new(Mutex::new(Toasts::default())),
            gallery: None,
            stored,
            login_form: LoginForm {
                username: String::new(),
                password: String::new(),
            },
            network_data: Arc::new(Mutex::new(DownloadData::default())),
        }
    }
}

impl eframe::App for Wallpapy {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.stored);
    }

    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        #[cfg(target_arch = "wasm32")]
        {
            let web_info = &_frame.info().web_info;
            self.host = web_info.location.host.clone();
        }

        ctx.style_mut(|style| {
            style.visuals.window_shadow = egui::epaint::Shadow::NONE;
        });

        if self.gallery.is_none() {
            self.get_gallery();
        }
        if self.stored.auth_token.is_empty() {
            self.show_login_panel(ctx);
        } else {
            self.show_main_panel(ctx);
        }

        self.toasts.lock().show(ctx);
    }
}

impl Wallpapy {
    fn show_main_panel(&mut self, ctx: &Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Welcome to Wallpapy!");

                if ui.button("Generate Wallpaper").clicked() {
                    self.toasts
                        .lock()
                        .info("Generating Wallpaper")
                        .set_duration(Some(Duration::from_secs(2)));
                    generate_wallpaper(&self.host, &self.stored.auth_token, move |_| {});
                }

                if ui.button("Logout").clicked() {
                    self.stored.auth_token.clear();
                }
            });
        });

        egui_extras::install_image_loaders(ctx);
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some((wallpapers, _)) = &self.gallery {
                let available_width = ui.available_width();
                let image_width = 400.0;
                let columns = (available_width / image_width).floor().max(1.0) as usize;
                let image_width = available_width / columns as f32;

                let refresh_response = PullToRefresh::new(false).scroll_area_ui(ui, |ui| {
                    ScrollArea::vertical().show(ui, |ui| {
                        for chunk in wallpapers.chunks(columns) {
                            let mut chunk_height: f32 = 0.0;
                            for wallpaper in chunk {
                                let scale = image_width / wallpaper.width as f32;
                                let height = wallpaper.height as f32 * scale;
                                chunk_height = chunk_height.max(height);
                            }
                            ui.horizontal(|ui| {
                                for wallpaper in chunk {
                                    self.draw_wallpaper_box(
                                        ui,
                                        wallpaper,
                                        image_width,
                                        chunk_height,
                                    );
                                }
                            });
                        }
                    })
                });
                if refresh_response.should_refresh() {
                    let network_store = self.network_data.clone();
                    network_store.lock().get_gallery = GetGalleryState::InProgress;
                    get_gallery(&self.host, move |res| {
                        network_store.lock().get_gallery = GetGalleryState::Done(res);
                    });
                }
            }
        });
    }

    fn draw_wallpaper_box(
        &self,
        ui: &mut egui::Ui,
        wallpaper: &WallpaperData,
        width: f32,
        height: f32,
    ) {
        let image_rect = ui
            .add_sized(
                Vec2::new(width, height),
                ThumbhashImage::new(
                    egui::Image::new(&format!(
                        "http://{}/wallpapers/{}",
                        self.host, wallpaper.file_name
                    )),
                    &wallpaper.thumbhash,
                )
                .id(format!("gallery_item_{}", wallpaper.id).into())
                .rounding(16.0),
            )
            .rect;

        // Add delete button in top-right corner
        let delete_button_size = vec2(20.0, 20.0);
        let delete_button_rect = egui::Rect::from_min_size(
            image_rect.right_top() - vec2(delete_button_size.x + 5.0, -5.0),
            delete_button_size,
        );
        let delete_button = ui.allocate_rect(delete_button_rect, egui::Sense::click());
        if delete_button.clicked() {
            println!("Delete wallpaper: {:?}", wallpaper.id);
        }

        // Start painting
        let painter = ui.painter();

        // Draw date in top-left corner
        let datetime_text = NaiveDateTime::parse_from_str(&wallpaper.datetime, "%Y-%m-%d_%H-%M-%S")
            .ok()
            .map(|naive_dt| {
                Utc.from_utc_datetime(&naive_dt)
                    .format("%d/%m/%Y %H:%M")
                    .to_string()
            })
            .unwrap_or_default();
        let datetime_scale = 12.0;

        let datetime_galley = painter.layout_no_wrap(
            datetime_text,
            FontId::proportional(datetime_scale),
            Color32::WHITE.gamma_multiply(0.8),
        );
        let datetime_rect = egui::Align2::LEFT_TOP.anchor_size(
            image_rect.left_top() + vec2(20.0, 20.0),
            datetime_galley.size(),
        );
        painter.add(Shape::rect_filled(
            datetime_rect.expand(datetime_scale * 0.5),
            datetime_scale,
            Color32::from_black_alpha(200),
        ));
        painter.galley(datetime_rect.min, datetime_galley, Color32::WHITE);

        // Paint delete button
        painter.text(
            delete_button_rect.center(),
            egui::Align2::CENTER_CENTER,
            "X",
            FontId::proportional(14.0),
            Color32::WHITE,
        );
    }

    fn get_gallery(&mut self) {
        let network_store = self.network_data.clone();
        let mut network_data_guard = network_store.lock();
        match &network_data_guard.get_gallery {
            GetGalleryState::None => {
                network_data_guard.get_gallery = GetGalleryState::InProgress;
                drop(network_data_guard);

                get_gallery(&self.host, move |res| {
                    network_store.lock().get_gallery = GetGalleryState::Done(res);
                });
            }
            GetGalleryState::InProgress => {}
            GetGalleryState::Done(ref response) => {
                match response {
                    Ok(gallery) => {
                        let datetime = Utc::now();
                        self.gallery = Some((gallery.clone(), datetime));
                    }
                    Err(e) => {
                        log::error!("Failed to fetch galleries: {:?}", e);
                    }
                }
                network_data_guard.get_gallery = GetGalleryState::None;
            }
        }
    }

    fn show_login_panel(&mut self, ctx: &Context) {
        CentralPanel::default()
            .frame(Frame {
                fill: Color32::from_rgb(25, 25, 35),
                ..Default::default()
            })
            .show(ctx, |_| {
                Window::new("Login Form".to_string())
                    .fixed_pos(ctx.screen_rect().center())
                    .fixed_size([300.0, 0.0])
                    .pivot(Align2::CENTER_CENTER)
                    .title_bar(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.vertical_centered(|ui| {
                            self.draw_login_form(ui);
                        });
                    });
            });
    }

    fn draw_login_form(&mut self, ui: &mut egui::Ui) {
        let network_store = self.network_data.clone();
        let mut network_data_guard = network_store.lock();
        match &network_data_guard.login {
            LoginState::None => {
                ui.horizontal(|ui| {
                    ui.label("Username:");
                    TextEdit::singleline(&mut self.login_form.username).show(ui);
                });
                ui.horizontal(|ui| {
                    ui.label("Password:");
                    TextEdit::singleline(&mut self.login_form.password)
                        .password(true)
                        .show(ui);
                });
                if ui.button("Login").clicked() {
                    network_data_guard.login = LoginState::InProgress;
                    drop(network_data_guard);
                    login(
                        &self.host,
                        &self.login_form.username,
                        &self.login_form.password,
                        move |res| {
                            network_store.lock().login = LoginState::Done(res);
                        },
                    );
                }
            }
            LoginState::InProgress => {
                ui.label("Logging in...");
                ui.add(egui::Spinner::new());
            }
            LoginState::Done(ref response) => {
                match response {
                    Ok(response) => {
                        if response.contains('|') {
                            // Split token on | to get message and token separately
                            let split: Vec<&str> = response.split('|').collect();
                            let message = split[0];
                            let token = split[1];

                            self.toasts
                                .lock()
                                .info(message)
                                .set_duration(Some(Duration::from_secs(3)));

                            self.stored.auth_token = token.to_string();
                        } else {
                            // If no | is found, treat the entire response as the token
                            self.stored.auth_token.clone_from(response);
                        }
                    }
                    Err(e) => {
                        self.toasts
                            .lock()
                            .error(e.to_string())
                            .set_duration(Some(Duration::from_secs(3)));
                    }
                }
                network_data_guard.login = LoginState::None;
            }
        }
    }
}
