use crate::{
    PORT,
    client::networking::{
        add_comment, edit_styles, generate_wallpaper, get_database, like_image, login,
        query_prompt, recreate_image, remove_comment, remove_image,
    },
    common::{CommentData, Database, LikedState, StyleVariant, WallpaperData},
};
use anyhow::Result;
use bitflags::bitflags;
use chrono::Local;
use egui::{
    Align2, CentralPanel, Color32, Context, CursorIcon, FontId, Frame, Image, Key, PointerButton,
    Rect, RichText, ScrollArea, Sense, Shape, TextEdit, Vec2, Widget, Window, vec2,
};
use egui_notify::Toasts;
use egui_pull_to_refresh::PullToRefresh;
use egui_thumbhash::ThumbhashImage;
use log::{error, info};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

pub struct Wallpapy {
    host: String,
    toasts: Arc<Mutex<Toasts>>,

    database: Option<Database>,
    fullscreen_image: Option<Uuid>,
    state_filter: StateFilter,

    stored: StoredData,
    login_form: LoginForm,
    comment_submission: String,

    network_data: Arc<Mutex<DownloadData>>,
}

#[derive(Deserialize, Serialize, Default)]
pub struct StoredData {
    auth_token: String,
}

struct LoginForm {
    username: String,
    password: String,
}

#[derive(Default)]
struct DownloadData {
    login: LoginState,
    get_database: GetDatabaseState,
}

#[derive(Default)]
enum LoginState {
    #[default]
    None,
    InProgress,
    Done(Result<String>),
}

#[derive(Default)]
enum GetDatabaseState {
    None,
    #[default]
    Wanted,
    InProgress,
    Done(Result<Database>),
}

bitflags! {
    #[derive(Clone, Copy)]
    pub struct StateFilter: u32 {
        const LIKED    = 0b00001;
        const LOVED    = 0b00010;
        const COMMENT  = 0b00100;
        const NEUTRAL  = 0b01000;
        const DISLIKED = 0b10000;
    }
}

impl Wallpapy {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let stored = cc.storage.map_or_else(StoredData::default, |storage| {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        });

        egui_extras::install_image_loaders(&cc.egui_ctx);
        egui_thumbhash::register(&cc.egui_ctx);

        cc.egui_ctx.style_mut(|style| {
            style.visuals.window_shadow = egui::epaint::Shadow::NONE;
            style.spacing.item_spacing = Vec2::new(8.0, 8.0);
        });

        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        cc.egui_ctx.set_fonts(fonts);

        Self {
            host: format!("localhost:{PORT}"),
            toasts: Arc::new(Mutex::new(Toasts::default())),
            database: None,
            fullscreen_image: None,
            state_filter: StateFilter::all(),
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

        self.get_database(ctx);
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
                    let ctx = ctx.clone();
                    generate_wallpaper(
                        &self.host,
                        &self.stored.auth_token,
                        self.comment_submission.trim(),
                        move |result| {
                            ctx.request_repaint();
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
                    let ctx = ctx.clone();
                    add_comment(
                        &self.host,
                        &self.stored.auth_token,
                        self.comment_submission.trim(),
                        move |result| {
                            ctx.request_repaint();
                            button_pressed_result(result, &network_store, &toasts_store, "");
                        },
                    );
                    self.comment_submission = String::new();
                }

                // Debug button that prints the prompt to console
                if ui.button("Query Prompt").clicked() {
                    query_prompt(&self.host, &self.stored.auth_token, move |result| {
                        if let Ok(prompt) = result {
                            info!("{prompt}");
                        }
                    });
                }

                if ui.button("Logout").clicked() {
                    self.stored.auth_token.clear();
                }

                // Filter buttons
                render_statefilter_button(
                    ui,
                    &mut self.state_filter,
                    StateFilter::LOVED,
                    egui_phosphor::regular::HEART,
                );
                render_statefilter_button(
                    ui,
                    &mut self.state_filter,
                    StateFilter::LIKED,
                    egui_phosphor::regular::THUMBS_UP,
                );
                render_statefilter_button(
                    ui,
                    &mut self.state_filter,
                    StateFilter::NEUTRAL,
                    egui_phosphor::regular::ALIGN_CENTER_HORIZONTAL_SIMPLE,
                );
                render_statefilter_button(
                    ui,
                    &mut self.state_filter,
                    StateFilter::DISLIKED,
                    egui_phosphor::regular::THUMBS_DOWN,
                );
                render_statefilter_button(
                    ui,
                    &mut self.state_filter,
                    StateFilter::COMMENT,
                    egui_phosphor::regular::CHAT_TEXT,
                );
            });
            if let Some(database) = &mut self.database {
                ui.horizontal(|ui| {
                    if TextEdit::multiline(&mut database.style.style)
                        .desired_width(f32::INFINITY)
                        .hint_text("What styles of wallpapers should it aim for (painted, realistic, etc.)?")
                        .ui(ui)
                        .changed()
                    {
                        let toasts_store = self.toasts.clone();
                        edit_styles(
                            &self.host,
                            &self.stored.auth_token,
                            StyleVariant::Style,
                            database.style.style.trim(),
                            move |result| match result {
                                Ok(()) => {}
                                Err(e) => {
                                    toasts_store
                                        .lock()
                                        .error(format!("Failed to update style: {e}"));
                                }
                            },
                        );
                    }
                });
                ui.horizontal(|ui| {
                    if TextEdit::multiline(&mut database.style.contents)
                        .desired_width(f32::INFINITY)
                        .hint_text("What contents of wallpapers should it aim for (epic fantasy, surreal, abstract, etc.)?")
                        .ui(ui)
                        .changed()
                    {
                        let toasts_store = self.toasts.clone();
                        edit_styles(
                            &self.host,
                            &self.stored.auth_token,
                            StyleVariant::Contents,
                            database.style.contents.trim(),
                            move |result| match result {
                                Ok(()) => {}
                                Err(e) => {
                                    toasts_store
                                        .lock()
                                        .error(format!("Failed to update contents: {e}"));
                                }
                            },
                        );
                    }
                });
                ui.horizontal(|ui| {
                    if TextEdit::multiline(&mut database.style.negative_contents)
                        .desired_width(f32::INFINITY)
                        .hint_text("What should never be included in wallpapers?")
                        .ui(ui)
                        .changed()
                    {
                        let toasts_store = self.toasts.clone();
                        edit_styles(
                            &self.host,
                            &self.stored.auth_token,
                            StyleVariant::NegativeContents,
                            database.style.negative_contents.trim(),
                            move |result| match result {
                                Ok(()) => {}
                                Err(e) => {
                                    toasts_store
                                        .lock()
                                        .error(format!("Failed to update negative contents: {e}"));
                                }
                            },
                        );
                    }
                });
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let mut new_fullscreen = None;
            // If escape pressed, close the fullscreen image
            if ui.input(|i| i.key_pressed(Key::Escape)) {
                self.fullscreen_image = None;
            }

            let refresh_response = PullToRefresh::new(false).scroll_area_ui(ui, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    // Display the fullscreen image if it exists
                    let wallpaper = self.fullscreen_image.as_ref().and_then(|id| {
                        self.database.as_ref().and_then(|db| {
                            db.wallpapers
                                .iter()
                                .find(|(wid, _)| *wid == id)
                                .map(|(_, w)| w)
                        })
                    });
                    if let Some(wallpaper) = &wallpaper {
                        ui.vertical(|ui| {
                            Image::new(format!(
                                "http://{}/wallpapers/{}",
                                self.host, wallpaper.image_file.file_name
                            ))
                            .show_loading_spinner(false)
                            .corner_radius(16.0)
                            .ui(ui);

                            let font_id = FontId::proportional(20.0);
                            if ui
                                .button(
                                    RichText::new(wallpaper.prompt_data.shortened_prompt.clone())
                                        .font(font_id.clone()),
                                )
                                .clicked()
                            {
                                ui.ctx()
                                    .copy_text(wallpaper.prompt_data.shortened_prompt.clone());
                                self.toasts.lock().info("Text copied to clipboard");
                            }
                            if ui
                                .button(
                                    RichText::new(wallpaper.prompt_data.prompt.clone())
                                        .font(font_id.clone()),
                                )
                                .clicked()
                            {
                                ui.ctx().copy_text(wallpaper.prompt_data.prompt.clone());
                                self.toasts.lock().info("Prompt copied to clipboard");
                            }
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!(
                                        "Saturation {}%  Lightness {}%  Chroma {}%",
                                        (wallpaper.color_data.saturation * 100.0) as i32,
                                        (wallpaper.color_data.lightness * 100.0) as i32,
                                        (wallpaper.color_data.chroma * 100.0) as i32
                                    ))
                                    .font(font_id.clone())
                                    .background_color(Color32::from_rgb(
                                        (wallpaper.color_data.average_color.0 * 255.0) as u8,
                                        (wallpaper.color_data.average_color.1 * 255.0) as u8,
                                        (wallpaper.color_data.average_color.2 * 255.0) as u8,
                                    ))
                                    .color(Color32::WHITE)
                                    .strong(),
                                );
                                ui.label(
                                    RichText::new(format!(
                                        "Top20 {}%  Bot20 {}%  Contrast {:.1}",
                                        (wallpaper.color_data.top_20_percent_brightness * 100.0)
                                            as i32,
                                        (wallpaper.color_data.bottom_20_percent_brightness * 100.0)
                                            as i32,
                                        wallpaper.color_data.contrast_ratio
                                    ))
                                    .font(font_id.clone())
                                    .background_color(Color32::DARK_GRAY)
                                    .color(Color32::WHITE)
                                    .strong(),
                                );
                            });
                        });

                        // Handle left and right arrow key press
                        let left_pressed =
                            ui.input(|i| i.key_pressed(Key::ArrowLeft) || i.key_pressed(Key::A));
                        let right_pressed =
                            ui.input(|i| i.key_pressed(Key::ArrowRight) || i.key_pressed(Key::D));
                        if (left_pressed || right_pressed) && self.database.is_some() {
                            let mut target_datetime = None;
                            let mut target_wallpaper = None;

                            let comparison = if left_pressed {
                                |dt1, dt2| dt1 > dt2
                            } else {
                                |dt1, dt2| dt1 < dt2
                            };

                            for paper in self.database.as_ref().unwrap().wallpapers.values() {
                                if comparison(paper.datetime, wallpaper.datetime)
                                    && (target_datetime.is_none()
                                        || comparison(target_datetime.unwrap(), paper.datetime))
                                {
                                    target_datetime = Some(paper.datetime);
                                    target_wallpaper = Some(paper.clone());
                                }
                            }

                            if let Some(target_wallpaper) = target_wallpaper {
                                new_fullscreen = Some(target_wallpaper.id);
                            }
                        }
                    } else if let Some(database) = self.database.clone() {
                        // Collect the wallpapers and comments into a single list, sorted by
                        // datetime
                        let mut combined_list = database
                            .wallpapers
                            .values()
                            .filter(|wallpaper| match wallpaper.liked_state {
                                LikedState::Liked => self.state_filter.contains(StateFilter::LIKED),
                                LikedState::Loved => self.state_filter.contains(StateFilter::LOVED),
                                LikedState::Disliked => {
                                    self.state_filter.contains(StateFilter::DISLIKED)
                                }
                                LikedState::Neutral => {
                                    self.state_filter.contains(StateFilter::NEUTRAL)
                                }
                            })
                            .map(|wallpaper| (wallpaper.datetime, Some(wallpaper), None))
                            .chain(
                                database
                                    .comments
                                    .values()
                                    .filter(|_| self.state_filter.contains(StateFilter::COMMENT))
                                    .map(|comment| (comment.datetime, None, Some(comment))),
                            )
                            .collect::<Vec<_>>();
                        combined_list.sort_by_key(|(datetime, ..)| *datetime);

                        let available_width = ui.available_width();
                        let spacing = ui.spacing().item_spacing;
                        let cell_width = 400.0;
                        let columns = (available_width / (cell_width + spacing.x))
                            .floor()
                            .max(1.0) as usize;
                        let cell_width = (columns as f32 - 1.0)
                            .mul_add(-spacing.x, available_width / columns as f32);
                        let cell_height = cell_width * 0.5625;

                        ui.horizontal_wrapped(|ui| {
                            for (_, wallpaper, comment) in combined_list.iter().rev() {
                                if let Some(wallpaper) = wallpaper {
                                    self.draw_wallpaper_box(ui, wallpaper, cell_width, cell_height);
                                }
                                if let Some(comment) = comment {
                                    self.draw_comment_box(ui, comment, cell_width, cell_height);
                                }
                            }
                        });
                    }
                })
            });
            if refresh_response.should_refresh() {
                self.network_data.lock().get_database = GetDatabaseState::Wanted;
                ui.ctx().forget_all_images();
                ui.ctx().clear_animations();
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
        let datetime_text = wallpaper
            .datetime
            .with_timezone(&Local)
            .format("%d/%m/%Y %H:%M")
            .to_string();
        let datetime_galley = painter.layout_no_wrap(
            datetime_text,
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
                let ctx = ui.ctx().clone();
                remove_image(
                    &self.host,
                    &self.stored.auth_token,
                    &wallpaper.id,
                    move |result| {
                        ctx.request_repaint();
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
                let ctx = ui.ctx().clone();
                like_image(
                    &self.host,
                    &self.stored.auth_token,
                    &wallpaper.id,
                    LikedState::Disliked,
                    move |result| {
                        ctx.request_repaint();
                        button_pressed_result(result, &network_store, &toasts_store, "");
                    },
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
                let ctx = ui.ctx().clone();
                like_image(
                    &self.host,
                    &self.stored.auth_token,
                    &wallpaper.id,
                    LikedState::Liked,
                    move |result| {
                        ctx.request_repaint();
                        button_pressed_result(result, &network_store, &toasts_store, "");
                    },
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
                let ctx = ui.ctx().clone();
                like_image(
                    &self.host,
                    &self.stored.auth_token,
                    &wallpaper.id,
                    LikedState::Loved,
                    move |result| {
                        ctx.request_repaint();
                        button_pressed_result(result, &network_store, &toasts_store, "");
                    },
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
                let ctx = ui.ctx().clone();
                recreate_image(
                    &self.host,
                    &self.stored.auth_token,
                    &wallpaper.id,
                    move |result| {
                        ctx.request_repaint();
                        button_pressed_result(result, &network_store, &toasts_store, "");
                    },
                );
            }
        }

        // Draw shortened prompt in bottom center, click to copy to clipboard
        let prompt_galley = painter.layout(
            wallpaper.prompt_data.shortened_prompt.clone(),
            FontId::proportional(ui_scale),
            Color32::WHITE.gamma_multiply(0.8),
            width - 40.0,
        );
        let prompt_rect = egui::Align2::CENTER_BOTTOM.anchor_size(
            image_rect.center_bottom() + vec2(0.0, -20.0),
            prompt_galley.size(),
        );
        let is_hovering = ui.rect_contains_pointer(prompt_rect);
        painter.add(Shape::rect_filled(
            prompt_rect.expand(ui_scale * 0.5625),
            ui_scale,
            match wallpaper.liked_state {
                LikedState::Loved => Color32::from_rgb(170, 120, 10),
                LikedState::Liked => Color32::from_rgb(40, 70, 40),
                LikedState::Disliked => Color32::from_rgb(100, 20, 20),
                LikedState::Neutral => Color32::BLACK,
            }
            .gamma_multiply(if is_hovering { 1.0 } else { 0.9 }),
        ));
        painter.galley(prompt_rect.min, prompt_galley, Color32::WHITE);
        if is_hovering {
            sub_button_hovered = true;
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            if ui.input(|i| i.pointer.button_clicked(PointerButton::Primary)) {
                ui.ctx()
                    .copy_text(wallpaper.prompt_data.shortened_prompt.clone());
                self.toasts.lock().info("Text copied to clipboard");
            }
        }

        // Check if image is clicked
        let is_hovering = ui.rect_contains_pointer(image_rect);
        if is_hovering
            && !sub_button_hovered
            && ui.input(|i| i.pointer.button_clicked(PointerButton::Primary))
        {
            self.fullscreen_image = Some(wallpaper.id);
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
        let datetime_text = comment
            .datetime
            .with_timezone(&Local)
            .format("%d/%m/%Y %H:%M")
            .to_string();
        let datetime_galley = painter.layout_no_wrap(
            datetime_text,
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
                let ctx = ui.ctx().clone();
                remove_comment(
                    &self.host,
                    &self.stored.auth_token,
                    &comment.id,
                    move |result| {
                        ctx.request_repaint();
                        button_pressed_result(result, &network_store, &toasts_store, "");
                    },
                );
            }
        }

        // Draw comments text in bottom center, click to copy to clipboard
        let text_galley = painter.layout(
            comment.comment.clone(),
            FontId::proportional(ui_scale),
            Color32::WHITE.gamma_multiply(0.8),
            width - 40.0,
        );
        let text_rect = egui::Align2::CENTER_BOTTOM
            .anchor_size(rect.center_bottom() + vec2(0.0, -20.0), text_galley.size());
        let is_hovering = ui.rect_contains_pointer(text_rect);
        painter.add(Shape::rect_filled(
            text_rect.expand(ui_scale * 0.5),
            ui_scale,
            Color32::BLACK.gamma_multiply(0.8),
        ));
        painter.galley(text_rect.min, text_galley, Color32::WHITE);
        if is_hovering {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            if ui.input(|i| i.pointer.button_clicked(PointerButton::Primary)) {
                ui.ctx().copy_text(comment.comment.clone());
                self.toasts.lock().info("Comment copied to clipboard");
            }
        }
    }

    fn get_database(&mut self, ctx: &Context) {
        let network_store = self.network_data.clone();
        let mut network_data_guard = network_store.lock();
        match &network_data_guard.get_database {
            GetDatabaseState::InProgress | GetDatabaseState::None => {}
            GetDatabaseState::Wanted => {
                network_data_guard.get_database = GetDatabaseState::InProgress;
                drop(network_data_guard);

                let ctx = ctx.clone();
                get_database(&self.host, move |res| {
                    network_store.lock().get_database = GetDatabaseState::Done(res);
                    ctx.request_repaint();
                });
            }
            GetDatabaseState::Done(response) => {
                match response {
                    Ok(database) => {
                        self.database = Some(database.clone());
                    }
                    Err(e) => {
                        error!("Failed to fetch galleries: {e:?}");
                    }
                }
                network_data_guard.get_database = GetDatabaseState::None;
                drop(network_data_guard);
                ctx.request_repaint();
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
                Window::new("Login Form")
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
            LoginState::Done(response) => {
                match response {
                    Ok(response) => {
                        if let Some((message, token)) = response.split_once('|') {
                            self.toasts.lock().info(message);
                            self.stored.auth_token = token.to_string();
                        } else {
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
    if let Err(e) = result {
        toasts_store
            .lock()
            .error(format!("Failed to submit request: {e}"));
        return;
    }

    if !success_str.is_empty() {
        toasts_store.lock().success(success_str);
    }
    network_store.lock().get_database = GetDatabaseState::Wanted;
}

fn render_statefilter_button(
    ui: &mut egui::Ui,
    state: &mut StateFilter,
    flag: StateFilter,
    label: &str,
) {
    let is_active = state.contains(flag);

    let button = egui::Button::new(label).fill(if is_active {
        egui::Color32::DARK_BLUE
    } else {
        egui::Color32::DARK_GRAY
    });

    if ui.add(button).clicked() {
        state.toggle(flag);
    }
}
