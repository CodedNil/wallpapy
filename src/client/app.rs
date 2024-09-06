use crate::{
    client::networking::{
        add_comment, generate_wallpaper, get_gallery, like_image, login, recreate_image,
        remove_comment, remove_image,
    },
    common::{CommentData, DatabaseObjectType, GetWallpapersResponse, LikedState, WallpaperData},
    PORT,
};
use anyhow::Result;
use egui::{
    vec2, Align2, CentralPanel, Color32, Context, CursorIcon, FontId, Frame, Image, Key,
    PointerButton, Rect, RichText, ScrollArea, Sense, Shape, TextEdit, Vec2, Widget, Window,
};
use egui_notify::Toasts;
use egui_pull_to_refresh::PullToRefresh;
use egui_thumbhash::ThumbhashImage;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

nestify::nest! {
    pub struct Wallpapy {
        host: String,
        toasts: Arc<Mutex<Toasts>>,

        wallpapers: Option<Vec<WallpaperData>>,
        comments: Option<Vec<CommentData>>,
        fullscreen_image: Option<WallpaperData>,

        #>[derive(Deserialize, Serialize, Default)]
        #>[serde(default)]
        stored: pub struct StoredData {
            auth_token: String,
        },

        login_form: struct LoginForm {
            username: String,
            password: String,
        },
        comment_submission: String,

        #>[derive(Default)]*
        network_data: Arc<Mutex<struct DownloadData {
            login: enum LoginState {
                #[default]
                None,
                InProgress,
                Done(Result<String>),
            },
            get_gallery: enum GetGalleryState {
                None,
                #[default]
                Wanted,
                InProgress,
                Done(Result<GetWallpapersResponse>),
            },
        }>>,
    }
}

impl Wallpapy {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let stored = cc.storage.map_or_else(StoredData::default, |storage| {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        });

        egui_extras::install_image_loaders(&cc.egui_ctx);
        egui_thumbhash::register(&cc.egui_ctx);

        Self {
            host: format!("localhost:{PORT}"),
            toasts: Arc::new(Mutex::new(Toasts::default())),
            wallpapers: None,
            comments: None,
            fullscreen_image: None,
            stored,
            login_form: LoginForm {
                username: String::new(),
                password: String::new(),
            },
            comment_submission: String::new(),
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
            style.spacing.item_spacing = Vec2::new(8.0, 8.0);
        });

        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        ctx.set_fonts(fonts);

        ctx.request_repaint();

        self.get_gallery(ctx);
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
                if ui.button("Generate Wallpaper").clicked() {
                    let toasts_store = self.toasts.clone();
                    let network_store = self.network_data.clone();
                    toasts_store.lock().info("Generating Wallpaper");
                    generate_wallpaper(
                        &self.host,
                        &self.stored.auth_token,
                        self.comment_submission.trim(),
                        move |result| {
                            button_pressed_result(
                                result,
                                &network_store,
                                &toasts_store,
                                "Generated wallpaper",
                            );
                        },
                    );
                    self.comment_submission = String::new();
                }

                // Text input for submitting a comment
                ui.text_edit_singleline(&mut self.comment_submission);
                if ui.button("Submit Comment").clicked() {
                    let toasts_store = self.toasts.clone();
                    let network_store = self.network_data.clone();
                    add_comment(
                        &self.host,
                        &self.stored.auth_token,
                        self.comment_submission.trim(),
                        move |result| {
                            button_pressed_result(result, &network_store, &toasts_store, "");
                        },
                    );
                    self.comment_submission = String::new();
                }

                if ui.button("Logout").clicked() {
                    self.stored.auth_token.clear();
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let mut new_fullscreen = None;
            // If escape pressed, close the fullscreen image
            if ui.input(|i| i.key_pressed(Key::Escape)) {
                self.fullscreen_image = None;
            }
            // Display the fullscreen image if it exists
            if let Some(wallpaper) = &self.fullscreen_image {
                let file = wallpaper
                    .upscaled_file
                    .as_ref()
                    .map_or(&wallpaper.original_file, |upscaled_file| upscaled_file);
                ui.vertical(|ui| {
                    Image::new(format!(
                        "http://{}/wallpapers/{}",
                        self.host, file.file_name
                    ))
                    .show_loading_spinner(false)
                    .rounding(16.0)
                    .ui(ui);
                    ui.label(
                        RichText::new(wallpaper.prompt.clone()).font(FontId::proportional(20.0)),
                    )
                });

                // Handle left and right arrow key press
                let left_pressed =
                    ui.input(|i| i.key_pressed(Key::ArrowLeft) || i.key_pressed(Key::A));
                let right_pressed =
                    ui.input(|i| i.key_pressed(Key::ArrowRight) || i.key_pressed(Key::D));
                if (left_pressed || right_pressed) && self.wallpapers.is_some() {
                    let mut target_datetime = None;
                    let mut target_wallpaper = None;

                    let comparison = if left_pressed {
                        |dt1, dt2| dt1 > dt2
                    } else {
                        |dt1, dt2| dt1 < dt2
                    };

                    for paper in self.wallpapers.as_ref().unwrap() {
                        if comparison(paper.datetime, wallpaper.datetime)
                            && (target_datetime.is_none()
                                || comparison(target_datetime.unwrap(), paper.datetime))
                        {
                            target_datetime = Some(paper.datetime);
                            target_wallpaper = Some(paper.clone());
                        }
                    }

                    if let Some(target_wallpaper) = target_wallpaper {
                        new_fullscreen = Some(target_wallpaper);
                    }
                }
            } else if let (Some(wallpapers), Some(comments)) = (&self.wallpapers, &self.comments) {
                // Collect the wallpapers and comments into a single list, sorted by datetime
                let mut combined_list = wallpapers
                    .iter()
                    .map(|wallpaper| {
                        (
                            wallpaper.datetime,
                            DatabaseObjectType::Wallpaper(wallpaper.clone()),
                        )
                    })
                    .chain(comments.iter().map(|comment| {
                        (
                            comment.datetime,
                            DatabaseObjectType::Comment(comment.clone()),
                        )
                    }))
                    .collect::<Vec<_>>();
                combined_list.sort_by_key(|(datetime, _)| *datetime);
                let combined_list = combined_list;

                let available_width = ui.available_width();
                let spacing = ui.spacing().item_spacing;
                let cell_width = 400.0;
                let columns = (available_width / (cell_width + spacing.x))
                    .floor()
                    .max(1.0) as usize;
                let cell_width =
                    (columns as f32 - 1.0).mul_add(-spacing.x, available_width / columns as f32);
                let cell_height = cell_width * 0.5625;

                let refresh_response = PullToRefresh::new(false).scroll_area_ui(ui, |ui| {
                    ScrollArea::vertical().show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            for (_, data) in combined_list.iter().rev() {
                                match data {
                                    DatabaseObjectType::Wallpaper(image) => {
                                        self.draw_wallpaper_box(ui, image, cell_width, cell_height);
                                    }
                                    DatabaseObjectType::Comment(comment) => {
                                        self.draw_comment_box(
                                            ui,
                                            comment,
                                            cell_width * 0.5,
                                            cell_height,
                                        );
                                    }
                                }
                            }
                        })
                    })
                });
                if refresh_response.should_refresh() {
                    self.network_data.lock().get_gallery = GetGalleryState::Wanted;
                    ui.ctx().forget_all_images();
                    ui.ctx().clear_animations();
                }
            }
            if new_fullscreen.is_some() {
                self.fullscreen_image = new_fullscreen;
            }
        });
    }

    fn draw_wallpaper_box(
        &mut self,
        ui: &mut egui::Ui,
        wallpaper: &WallpaperData,
        width: f32,
        height: f32,
    ) {
        // Only render images if they are visible (this is basically lazy loading)
        let image_size = Vec2::new(width, height);
        let image_rect =
            if ui.is_rect_visible(Rect::from_min_size(ui.next_widget_position(), image_size)) {
                let image = egui::Image::new(format!(
                    "http://{}/wallpapers/{}",
                    self.host, wallpaper.thumbnail_file.file_name
                ))
                .show_loading_spinner(false);
                ui.add_sized(
                    image_size,
                    ThumbhashImage::new(image, &wallpaper.thumbhash).rounding(16.0),
                )
                .rect
            } else {
                let (rect, _) = ui.allocate_exact_size(image_size, Sense::hover());
                rect
            };

        // Start painting
        let ui_scale = 12.0;
        let painter = ui.painter();
        let mut sub_button_hovered = false;

        // Draw date in top-left corner
        let datetime_galley = painter.layout_no_wrap(
            wallpaper.datetime_text.clone(),
            FontId::proportional(ui_scale),
            Color32::WHITE.gamma_multiply(0.8),
        );
        let datetime_rect = egui::Align2::LEFT_TOP.anchor_size(
            image_rect.left_top() + vec2(20.0, 20.0),
            datetime_galley.size(),
        );
        painter.add(Shape::rect_filled(
            datetime_rect.expand(ui_scale * 0.5),
            ui_scale,
            Color32::BLACK.gamma_multiply(0.8),
        ));
        painter.galley(datetime_rect.min, datetime_galley, Color32::WHITE);

        // Draw average brightness and color below date
        let colorinfo_galley = painter.layout_no_wrap(
            format!("{:.0}%", f32::from(wallpaper.brightness) / 255.0 * 100.0,),
            FontId::proportional(ui_scale),
            Color32::WHITE,
        );
        let colorinfo_rect = egui::Align2::LEFT_TOP.anchor_size(
            datetime_rect.left_bottom() + vec2(0.0, 15.0),
            colorinfo_galley.size(),
        );
        painter.add(Shape::rect_filled(
            colorinfo_rect.expand(ui_scale * 0.5),
            ui_scale,
            Color32::from_rgb(
                wallpaper.average_color.0,
                wallpaper.average_color.1,
                wallpaper.average_color.2,
            ),
        ));
        painter.galley(colorinfo_rect.min, colorinfo_galley, Color32::WHITE);

        // Add delete button in top-right corner
        let delete_button_size = vec2(ui_scale.mul_add(2.0, 2.0), ui_scale.mul_add(2.0, 2.0));
        let delete_button_rect = egui::Align2::RIGHT_TOP.anchor_size(
            image_rect.right_top() + vec2(-20.0, 20.0),
            delete_button_size,
        );
        let is_hovering = ui.rect_contains_pointer(delete_button_rect);
        painter.add(Shape::rect_filled(
            delete_button_rect,
            ui_scale,
            Color32::BLACK.gamma_multiply(if is_hovering { 1.0 } else { 0.8 }),
        ));
        painter.text(
            delete_button_rect.center(),
            egui::Align2::CENTER_CENTER,
            egui_phosphor::regular::X,
            FontId::proportional(ui_scale),
            Color32::WHITE,
        );
        if is_hovering {
            sub_button_hovered = true;
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            if ui.input(|i| i.pointer.button_clicked(PointerButton::Primary)) {
                let toasts_store = self.toasts.clone();
                let network_store = self.network_data.clone();
                remove_image(
                    &self.host,
                    &self.stored.auth_token,
                    &wallpaper.id,
                    move |result| {
                        button_pressed_result(result, &network_store, &toasts_store, "");
                    },
                );
            }
        }

        // Add thumbs down button
        let thumbs_down_button_rect = egui::Align2::RIGHT_TOP.anchor_size(
            delete_button_rect.left_top() + vec2(-10.0, 0.0),
            delete_button_size,
        );
        let is_hovering = ui.rect_contains_pointer(thumbs_down_button_rect);
        painter.add(Shape::rect_filled(
            thumbs_down_button_rect,
            ui_scale,
            if wallpaper.liked_state == LikedState::Disliked {
                Color32::DARK_RED
            } else {
                Color32::BLACK
            }
            .gamma_multiply(if is_hovering { 1.0 } else { 0.8 }),
        ));
        painter.text(
            thumbs_down_button_rect.center(),
            egui::Align2::CENTER_CENTER,
            egui_phosphor::regular::THUMBS_DOWN,
            FontId::proportional(ui_scale),
            Color32::WHITE,
        );
        if is_hovering {
            sub_button_hovered = true;
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            if ui.input(|i| i.pointer.button_clicked(PointerButton::Primary)) {
                let toasts_store = self.toasts.clone();
                let network_store = self.network_data.clone();
                like_image(
                    &self.host,
                    &self.stored.auth_token,
                    &wallpaper.id,
                    LikedState::Disliked,
                    move |result| button_pressed_result(result, &network_store, &toasts_store, ""),
                );
            }
        }

        // Add thumbs up button
        let thumbs_up_button_rect = egui::Align2::RIGHT_TOP.anchor_size(
            thumbs_down_button_rect.left_top() + vec2(-10.0, 0.0),
            delete_button_size,
        );
        let is_hovering = ui.rect_contains_pointer(thumbs_up_button_rect);
        painter.add(Shape::rect_filled(
            thumbs_up_button_rect,
            ui_scale,
            if wallpaper.liked_state == LikedState::Liked {
                Color32::DARK_GREEN
            } else {
                Color32::BLACK
            }
            .gamma_multiply(if is_hovering { 1.0 } else { 0.8 }),
        ));
        painter.text(
            thumbs_up_button_rect.center(),
            egui::Align2::CENTER_CENTER,
            egui_phosphor::regular::THUMBS_UP,
            FontId::proportional(ui_scale),
            Color32::WHITE,
        );
        if is_hovering {
            sub_button_hovered = true;
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            if ui.input(|i| i.pointer.button_clicked(PointerButton::Primary)) {
                let toasts_store = self.toasts.clone();
                let network_store = self.network_data.clone();
                like_image(
                    &self.host,
                    &self.stored.auth_token,
                    &wallpaper.id,
                    LikedState::Liked,
                    move |result| button_pressed_result(result, &network_store, &toasts_store, ""),
                );
            }
        }

        // Add loved button
        let loved_button_rect = egui::Align2::RIGHT_TOP.anchor_size(
            thumbs_up_button_rect.left_top() + vec2(-10.0, 0.0),
            delete_button_size,
        );
        let is_hovering = ui.rect_contains_pointer(loved_button_rect);
        painter.add(Shape::rect_filled(
            loved_button_rect,
            ui_scale,
            if wallpaper.liked_state == LikedState::Loved {
                Color32::from_rgb(140, 90, 0)
            } else {
                Color32::BLACK
            }
            .gamma_multiply(if is_hovering { 1.0 } else { 0.8 }),
        ));
        painter.text(
            loved_button_rect.center(),
            egui::Align2::CENTER_CENTER,
            egui_phosphor::regular::HEART,
            FontId::proportional(ui_scale),
            Color32::WHITE,
        );
        if is_hovering {
            sub_button_hovered = true;
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            if ui.input(|i| i.pointer.button_clicked(PointerButton::Primary)) {
                let toasts_store = self.toasts.clone();
                let network_store = self.network_data.clone();
                like_image(
                    &self.host,
                    &self.stored.auth_token,
                    &wallpaper.id,
                    LikedState::Loved,
                    move |result| button_pressed_result(result, &network_store, &toasts_store, ""),
                );
            }
        }

        // Add recreate button
        let recreate_button_rect = egui::Align2::RIGHT_TOP.anchor_size(
            loved_button_rect.left_top() + vec2(-10.0, 0.0),
            delete_button_size,
        );
        let is_hovering = ui.rect_contains_pointer(recreate_button_rect);
        painter.add(Shape::rect_filled(
            recreate_button_rect,
            ui_scale,
            Color32::BLACK.gamma_multiply(if is_hovering { 1.0 } else { 0.8 }),
        ));
        painter.text(
            recreate_button_rect.center(),
            egui::Align2::CENTER_CENTER,
            egui_phosphor::regular::REPEAT,
            FontId::proportional(ui_scale),
            Color32::WHITE,
        );
        if is_hovering {
            sub_button_hovered = true;
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            if ui.input(|i| i.pointer.button_clicked(PointerButton::Primary)) {
                let toasts_store = self.toasts.clone();
                let network_store = self.network_data.clone();
                recreate_image(
                    &self.host,
                    &self.stored.auth_token,
                    &wallpaper.id,
                    move |result| button_pressed_result(result, &network_store, &toasts_store, ""),
                );
            }
        }

        // Draw shortened prompt in bottom center
        let prompt_galley = painter.layout(
            wallpaper.prompt_short.clone(),
            FontId::proportional(ui_scale),
            Color32::WHITE.gamma_multiply(0.8),
            width - 40.0,
        );
        let prompt_rect = egui::Align2::CENTER_BOTTOM.anchor_size(
            image_rect.center_bottom() + vec2(0.0, -20.0),
            prompt_galley.size(),
        );
        painter.add(Shape::rect_filled(
            prompt_rect.expand(ui_scale * 0.5625),
            ui_scale,
            match wallpaper.liked_state {
                LikedState::Loved => Color32::from_rgb(170, 120, 10),
                LikedState::Liked => Color32::from_rgb(40, 70, 40),
                LikedState::Disliked => Color32::from_rgb(100, 20, 20),
                LikedState::None => Color32::BLACK,
            }
            .gamma_multiply(0.9),
        ));
        painter.galley(prompt_rect.min, prompt_galley, Color32::WHITE);

        // Check if image is clicked
        let is_hovering = ui.rect_contains_pointer(image_rect);
        if is_hovering
            && !sub_button_hovered
            && ui.input(|i| i.pointer.button_clicked(PointerButton::Primary))
        {
            self.fullscreen_image = Some(wallpaper.clone());
        }
    }

    fn draw_comment_box(&self, ui: &mut egui::Ui, comment: &CommentData, width: f32, height: f32) {
        let (response, painter) = ui.allocate_painter(Vec2::new(width, height), Sense::click());
        let rect = response.rect;

        // Start painting
        let ui_scale = 12.0;

        // Draw rounded rectangle filling the rect
        painter.add(Shape::rect_filled(
            rect,
            ui_scale,
            Color32::from_rgb(60, 60, 80).gamma_multiply(0.8),
        ));

        // Draw date in top-left corner
        let datetime_galley = painter.layout_no_wrap(
            comment.datetime_text.clone(),
            FontId::proportional(ui_scale),
            Color32::WHITE.gamma_multiply(0.8),
        );
        let datetime_rect = egui::Align2::LEFT_TOP
            .anchor_size(rect.left_top() + vec2(20.0, 20.0), datetime_galley.size());
        painter.add(Shape::rect_filled(
            datetime_rect.expand(ui_scale * 0.5),
            ui_scale,
            Color32::BLACK.gamma_multiply(0.8),
        ));
        painter.galley(datetime_rect.min, datetime_galley, Color32::WHITE);

        // Add delete button in top-right corner
        let delete_button_size = vec2(ui_scale.mul_add(2.0, 2.0), ui_scale.mul_add(2.0, 2.0));
        let delete_button_rect = egui::Align2::RIGHT_TOP
            .anchor_size(rect.right_top() + vec2(-20.0, 20.0), delete_button_size);
        let is_hovering = ui.rect_contains_pointer(delete_button_rect);
        painter.add(Shape::rect_filled(
            delete_button_rect,
            ui_scale,
            Color32::BLACK.gamma_multiply(if is_hovering { 1.0 } else { 0.8 }),
        ));
        painter.text(
            delete_button_rect.center(),
            egui::Align2::CENTER_CENTER,
            egui_phosphor::regular::X,
            FontId::proportional(ui_scale),
            Color32::WHITE,
        );
        if is_hovering {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            if ui.input(|i| i.pointer.button_clicked(PointerButton::Primary)) {
                let toasts_store = self.toasts.clone();
                let network_store = self.network_data.clone();
                remove_comment(
                    &self.host,
                    &self.stored.auth_token,
                    &comment.id,
                    move |result| button_pressed_result(result, &network_store, &toasts_store, ""),
                );
            }
        }

        // Draw comments text in bottom center
        let text_galley = painter.layout(
            comment.comment.clone(),
            FontId::proportional(ui_scale),
            Color32::WHITE.gamma_multiply(0.8),
            width - 40.0,
        );
        let text_rect = egui::Align2::CENTER_BOTTOM
            .anchor_size(rect.center_bottom() + vec2(0.0, -20.0), text_galley.size());
        painter.add(Shape::rect_filled(
            text_rect.expand(ui_scale * 0.5),
            ui_scale,
            Color32::BLACK.gamma_multiply(0.8),
        ));
        painter.galley(text_rect.min, text_galley, Color32::WHITE);
    }

    fn get_gallery(&mut self, ctx: &Context) {
        let network_store = self.network_data.clone();
        let mut network_data_guard = network_store.lock();
        match &network_data_guard.get_gallery {
            GetGalleryState::InProgress | GetGalleryState::None => {}
            GetGalleryState::Wanted => {
                log::info!("Fetching gallery");
                ctx.request_repaint();
                network_data_guard.get_gallery = GetGalleryState::InProgress;
                drop(network_data_guard);

                get_gallery(&self.host, move |res| {
                    network_store.lock().get_gallery = GetGalleryState::Done(res);
                });
            }
            GetGalleryState::Done(ref response) => {
                match response {
                    Ok(wallpapers) => {
                        self.wallpapers = Some(wallpapers.images.clone());
                        self.comments = Some(wallpapers.comments.clone());
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

                            self.toasts.lock().info(message);

                            self.stored.auth_token = token.to_string();
                        } else {
                            // If no | is found, treat the entire response as the token
                            self.stored.auth_token.clone_from(response);
                        }
                    }
                    Err(e) => {
                        self.toasts.lock().error(e.to_string());
                    }
                }
                network_data_guard.login = LoginState::None;
            }
        }
    }
}

fn button_pressed_result(
    result: Result<()>,
    network_store: &Arc<Mutex<DownloadData>>,
    toasts_store: &Arc<Mutex<Toasts>>,
    success_str: &str,
) {
    match result {
        Ok(()) => {
            if !success_str.is_empty() {
                toasts_store.lock().success(success_str);
            }
            network_store.lock().get_gallery = GetGalleryState::Wanted;
        }
        Err(e) => {
            toasts_store
                .lock()
                .error(format!("Failed to submit request: {e}"));
        }
    }
}
