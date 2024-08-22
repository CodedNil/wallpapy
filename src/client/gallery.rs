use egui::{Id, Image, ScrollArea, Ui, Vec2};
use egui_infinite_scroll::InfiniteScroll;
use egui_pull_to_refresh::PullToRefresh;
use egui_thumbhash::ThumbhashImage;

use crate::common::WallpaperData;

pub struct Gallery {
    items: InfiniteScroll<WallpaperData, usize>,
}

impl Gallery {
    pub fn new(gallery_items: Vec<WallpaperData>) -> Self {
        let items = InfiniteScroll::new().end_loader(move |cursor, callback| {
            let cursor = cursor.unwrap_or(0);
            let items: Vec<_> = gallery_items
                .iter()
                .skip(cursor)
                .take(10)
                .cloned()
                .collect();
            callback(Ok((items, Some(cursor + 10))));
        });
        Self { items }
    }
}

impl Gallery {
    pub fn show(&mut self, ui: &mut Ui, host: &str) {
        let height = 300.0;

        let refresh_response = PullToRefresh::new(false).scroll_area_ui(ui, |ui| {
            ScrollArea::vertical()
                .max_height(ui.available_height() * 0.9 - 32.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = Vec2::splat(16.0);
                    let item_spacing = ui.spacing_mut().item_spacing.x;

                    self.items.ui_custom_layout(ui, 10, |ui, start_idx, item| {
                        let total_width = ui.available_width();

                        let mut count = 1;
                        let mut combined_width =
                            item.first().map(|item| item.width).unwrap_or(0) as f32;

                        while combined_width < total_width - item_spacing * (count - 1) as f32
                            && count < item.len()
                        {
                            count += 1;
                            let item = &item[count - 1];
                            let item_aspect_ratio = item.width as f32 / item.height as f32;
                            let item_width = height * item_aspect_ratio;
                            combined_width += item_width;
                        }

                        let scale =
                            (total_width - item_spacing * (count - 1) as f32) / combined_width;

                        let height = height * scale;

                        ui.horizontal(|ui| {
                            for (idx, item) in item.iter().enumerate().take(count) {
                                let size = Vec2::new(item.width as f32 * scale, height);
                                let response = ui.add_sized(
                                    size,
                                    ThumbhashImage::new(
                                        Image::new(&format!(
                                            "http://{}/wallpapers/{}",
                                            host, item.file_name
                                        )),
                                        &item.thumbhash,
                                    )
                                    .id(Id::new("gallery_item").with(start_idx + idx))
                                    .rounding(8.0),
                                );
                            }
                        });

                        count
                    });
                })
        });

        if refresh_response.should_refresh() {
            self.items.reset();
            ui.ctx().forget_all_images();
            ui.ctx().clear_animations();
        }
    }
}
