use crate::{
    common::{LikedState, WallpaperData},
    routing::{
        action_comment, action_delete, action_generate, action_like, action_recreate,
        action_styles, load_gallery_data,
    },
};
use dioxus::prelude::*;
use dioxus_free_icons::{Icon, IconShape, icons::fa_solid_icons};

const NEUTRAL_COLOR: &str = "10, 10, 10";
const LOVED_COLOR: &str = "160, 100, 10";
const LIKED_COLOR: &str = "20, 80, 30";
const DISLIKED_COLOR: &str = "90, 15, 15";

const OVERLAY_OPACITY: &str = "0.7";
const OVERLAY_TEXT_OPACITY: &str = "0.9";

pub fn app() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: "/static/style.css" }
        GalleryPage {}
    }
}

#[component]
fn GalleryPage() -> Element {
    let mut generate_action = use_action(action_generate);
    let mut generate_prompt = use_signal(String::new);
    let data = use_server_future(load_gallery_data)?;

    let (items, style_val) = match data() {
        Some(Ok(d)) => (d.items, d.style_prompt),
        _ => {
            return rsx! {
                p { "Error loading gallery." }
            };
        }
    };

    rsx! {
        div {
            display: "flex",
            gap: "8px",
            padding: "8px 12px",
            background: "rgba(20, 20, 32, 0.8)",
            backdrop_filter: "blur(10px)",
            position: "sticky",
            top: "0",
            z_index: "100",
            align_items: "center",
            input {
                placeholder: "Custom prompt (optional)",
                value: generate_prompt(),
                oninput: move |e| generate_prompt.set(e.value()),
            }
            button {
                onclick: move |_| {
                    let p = generate_prompt();
                    generate_action.call(if p.trim().is_empty() { None } else { Some(p) });
                    generate_prompt.set(String::new());
                },
                "Generate"
            }
        }
        StyleBox { initial_val: style_val }
        div {
            display: "grid",
            grid_template_columns: "repeat(auto-fill, minmax(360px, 1fr))",
            gap: "10px",
            padding: "10px",
            for w in items {
                WallpaperCard { key: "{w.id}", w }
            }
        }
    }
}

#[component]
fn StyleBox(initial_val: String) -> Element {
    let mut styles_action = use_action(action_styles);
    rsx! {
        div { padding: "8px 12px", background: "rgba(15, 15, 25)",
            textarea {
                resize: "none",
                display: "block",
                width: "100%",
                min_height: "52px",
                font_size: "11px",
                padding: "8px",
                color: "white",
                background: "none",
                border: "none",
                border_radius: "6px",
                placeholder: "Style prompt...",
                oninput: move |e| styles_action.call(e.value()),
                "{initial_val}"
            }
        }
    }
}

#[component]
fn WallpaperCard(w: WallpaperData) -> Element {
    let date = w.datetime.format("%Y-%m-%d").to_string();

    let mut like_action = use_action(action_like);
    let mut recreate_action = use_action(action_recreate);
    let mut delete_action = use_action(action_delete);
    let mut comment_action = use_action(action_comment);
    let mut comment_signal = use_signal(|| w.comment.clone().unwrap_or_default());

    let mut liked_signal = use_signal(|| w.liked_state);
    let mut update_like = move |target: LikedState| {
        let new_state = if liked_signal() == target {
            LikedState::Neutral
        } else {
            target
        };
        liked_signal.set(new_state);
        like_action.call(w.id, new_state);
    };

    rsx! {
        div {
            border_radius: "26px",
            overflow: "hidden",
            background: "#1a1a24",

            a {
                display: "block",
                position: "relative",
                aspect_ratio: "16 / 9",
                overflow: "hidden",
                img {
                    width: "100%",
                    height: "100%",
                    object_fit: "cover",
                    display: "block",
                    src: "/wallpapers/{w.thumbnail_file.file_name}",
                    loading: "lazy",
                }

                div {
                    position: "absolute",
                    top: "0px",
                    left: "0px",
                    width: "100%",
                    height: "100%",
                    padding: "16px",
                    display: "flex",
                    flex_direction: "column",
                    justify_content: "space-between",
                    pointer_events: "none",

                    div {
                        height: "26px",
                        display: "flex",
                        justify_content: "space-between",
                        align_items: "start",

                        Pill { text: date }

                        div { display: "flex", gap: "4px",
                            IconButton {
                                color: if liked_signal() == LikedState::Loved { Some(LOVED_COLOR) } else { None },
                                icon: fa_solid_icons::FaHeart,
                                onclick: move |_| update_like(LikedState::Loved),
                            }
                            IconButton {
                                color: if liked_signal() == LikedState::Liked { Some(LIKED_COLOR) } else { None },
                                icon: fa_solid_icons::FaThumbsUp,
                                onclick: move |_| update_like(LikedState::Liked),
                            }
                            IconButton {
                                color: if liked_signal() == LikedState::Disliked { Some(DISLIKED_COLOR) } else { None },
                                icon: fa_solid_icons::FaThumbsDown,
                                onclick: move |_| update_like(LikedState::Disliked),
                            }
                            IconButton {
                                icon: fa_solid_icons::FaArrowRotateLeft,
                                onclick: move |_| recreate_action.call(w.id),
                            }
                            IconButton {
                                icon: fa_solid_icons::FaTrash,
                                onclick: move |_| delete_action.call(w.id),
                            }
                        }
                    }

                    div { display: "flex", justify_content: "flex-start",
                        Pill {
                            color: match liked_signal() {
                                LikedState::Loved => Some(LOVED_COLOR),
                                LikedState::Liked => Some(LIKED_COLOR),
                                LikedState::Neutral => None,
                                LikedState::Disliked => Some(DISLIKED_COLOR),
                            },
                            text: w.prompt_data.shortened_prompt,
                        }
                    }
                }
            }

            div { padding: "10px",
                div { display: "flex",
                    input {
                        r#type: "text",
                        flex: "1",
                        font_size: "11px",
                        padding: "4px 8px",
                        background: "none",
                        border: "none",
                        border_radius: "6px",
                        color: "white",
                        placeholder: "Add a note...",
                        value: comment_signal(),
                        oninput: move |evt| {
                            let val = evt.value();
                            comment_signal.set(val.clone());
                            let comment = if val.trim().is_empty() { None } else { Some(val) };
                            comment_action.call(w.id, comment);
                        },
                    }
                }
            }
        }
    }
}

#[component]
fn Pill(color: Option<&'static str>, text: String) -> Element {
    rsx! {
        span {
            padding: "6px 10px",
            border_radius: "12px",
            backdrop_filter: "blur(8px)",
            font_size: "11px",
            font_weight: "bold",
            background: format!("rgba({}, {OVERLAY_OPACITY})", color.unwrap_or(NEUTRAL_COLOR)),
            color: format!("rgba(255, 255, 255, {OVERLAY_TEXT_OPACITY})"),
            "{text}"
        }
    }
}

#[component]
fn IconButton<T: IconShape + Clone + PartialEq + 'static>(
    color: Option<&'static str>,
    icon: T,
    onclick: EventHandler<MouseEvent>,
) -> Element {
    rsx!(
        button {
            width: "26px",
            height: "26px",
            border_radius: "20px",
            backdrop_filter: "blur(8px)",
            border: "none",
            display: "flex",
            align_items: "center",
            justify_content: "center",
            background: format!("rgba({}, {OVERLAY_OPACITY})", color.unwrap_or(NEUTRAL_COLOR)),
            color: format!("rgba(255, 255, 255, {OVERLAY_TEXT_OPACITY})"),
            cursor: "pointer",
            pointer_events: "auto",
            onclick: move |evt| onclick.call(evt),
            Icon {
                width: 12,
                height: 12,
                fill: "currentColor",
                icon,
            }
        }
    )
}
