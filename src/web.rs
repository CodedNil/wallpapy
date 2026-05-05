use crate::{
    common::{LikedState, WallpaperData},
    routing::{
        action_comment, action_delete, action_generate, action_like, action_recreate,
        action_styles, load_gallery_data,
    },
};
use chrono::Utc;
use dioxus::prelude::*;
use dioxus_free_icons::{Icon, IconShape, icons::fa_solid_icons};

const NEUTRAL_COLOR: &str = "10, 10, 10";
const LOVED_COLOR: &str = "160, 100, 10";
const LIKED_COLOR: &str = "20, 80, 30";
const DISLIKED_COLOR: &str = "90, 15, 15";

const OVERLAY_OPACITY: &str = "0.7";
const OVERLAY_TEXT_COLOR: &str = "rgba(255, 255, 255, 0.9)";

pub fn app() -> Element {
    let mut fullscreen_id = use_context_provider(|| Signal::new(None::<uuid::Uuid>));

    rsx! {
        document::Title { "Wallpapy" }
        document::Meta { name: "darkreader-lock" }
        document::Link { rel: "icon", href: asset!("/assets/icon.svg") }
        document::Script {
            // Dioxus SSR renders textarea value as an HTML attribute, which browsers ignore for
            // display — only the DOM `.value` property is shown. This copies the attribute to the
            // property before Dioxus hydrates, so textareas are populated on first paint.
            r#"
                document.addEventListener('DOMContentLoaded', function() {{
                    document.querySelectorAll('textarea[value]').forEach(function(t) {{
                        if (!t.value) t.value = t.getAttribute('value');
                    }});
                }});
            "#
        }
        document::Style {
            r#"
                * {{
                    box-sizing: border-box;
                    margin: 0;
                    padding: 0;
                }}

                body {{
                    background-image: url("/smartget");
                    background-color: rgba(0, 0, 0, 0.4);
                    background-blend-mode: multiply;
                    background-size: cover;
                    color: white;
                    font-family: sans-serif;
                    font-size: 14px;
                    min-height: 100vh;
                }}
            "#
        }
        div {
            tabindex: "0",
            onkeydown: move |e| {
                if e.key() == Key::Escape {
                    fullscreen_id.set(None);
                }
            },
            GalleryPage {}
        }
    }
}

#[component]
fn GalleryPage() -> Element {
    let mut generate_action = use_action(action_generate);
    let mut styles_action = use_action(action_styles);
    let mut generate_prompt = use_signal(String::new);
    let mut btn_hovered = use_signal(|| false);
    let mut btn_pressed = use_signal(|| false);
    let mut prompt_active = use_signal(|| false);
    let data = use_server_future(load_gallery_data)?;

    let (items, style_val) = match data() {
        Some(Ok(d)) => (d.items, d.style_prompt),
        _ => {
            return rsx! {
                p { "Error loading gallery." }
            };
        }
    };

    let prompt_expanded = prompt_active() || !generate_prompt().is_empty();

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
            div { display: "flex",
                button {
                    padding: "6px 14px",
                    border_radius: "8px 0 0 8px",
                    border: "none",
                    color: "white",
                    cursor: "pointer",
                    font_size: "13px",
                    font_weight: "bolder",
                    background: format!("rgba(80, 140, 90)"),
                    filter: if btn_hovered() { "brightness(1.5)" } else { "brightness(1)" },
                    transform: if btn_pressed() { "scale(0.97)" } else { "scale(1)" },
                    transition: "filter 0.15s ease, transform 0.1s ease",
                    onmouseenter: move |_| btn_hovered.set(true),
                    onmouseleave: move |_| {
                        btn_hovered.set(false);
                        btn_pressed.set(false);
                    },
                    onmousedown: move |_| btn_pressed.set(true),
                    onmouseup: move |_| btn_pressed.set(false),
                    onclick: move |_| {
                        let p = generate_prompt();
                        generate_action.call(if p.trim().is_empty() { None } else { Some(p) });
                        generate_prompt.set(String::new());
                    },
                    "Generate"
                }
                input {
                    style: format!(
                        "width: {}; border-radius: 0 8px 8px 0; background: rgba(100, 160, 110); border: none; outline: none; color: white; font-size: 13px; padding: 6px {}; transition: width 0.25s ease, padding 0.25s ease; text-align: left;",
                        if prompt_expanded { "160px" } else { "30px" },
                        if prompt_expanded { "10px" } else { "6px" },
                    ),
                    placeholder: if prompt_expanded { "Custom prompt..." } else { "✨" },
                    value: generate_prompt(),
                    oninput: move |e| generate_prompt.set(e.value()),
                    onmouseenter: move |_| prompt_active.set(true),
                    onmouseleave: move |_| prompt_active.set(false),
                    onfocus: move |_| prompt_active.set(true),
                    onblur: move |_| prompt_active.set(false),
                }
            }
        }
        GhostInput {
            value: style_val,
            placeholder: "Style prompt...",
            oninput: move |val| styles_action.call(val),
        }
        div {
            display: "grid",
            grid_template_columns: "repeat(auto-fill, minmax(360px, 1fr))",
            gap: "20px",
            padding: "20px",
            for w in items {
                WallpaperCard { key: "{w.id}", w }
            }
        }
    }
}

#[component]
fn WallpaperCard(w: WallpaperData) -> Element {
    let diff = Utc::now().signed_duration_since(w.datetime);
    let date = if diff.num_weeks() >= 1 || diff.num_milliseconds() < 0 {
        w.datetime.format("%d/%m/%Y %I%P").to_string()
    } else if diff.num_days() >= 1 {
        let n = diff.num_days();
        format!("{n} day{} ago", if n == 1 { "" } else { "s" })
    } else if diff.num_hours() >= 1 {
        let n = diff.num_hours();
        format!("{n} hour{} ago", if n == 1 { "" } else { "s" })
    } else if diff.num_minutes() >= 1 {
        let n = diff.num_minutes();
        format!("{n} minute{} ago", if n == 1 { "" } else { "s" })
    } else {
        "just now".to_string()
    };

    let mut like_action = use_action(action_like);
    let mut recreate_action = use_action(action_recreate);
    let mut delete_action = use_action(action_delete);
    let mut comment_action = use_action(action_comment);

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

    let mut comment_signal = use_signal(|| w.comment.clone().unwrap_or_default());
    let mut hovered = use_signal(|| false);

    let mut fullscreen_id = use_context::<Signal<Option<uuid::Uuid>>>();
    let is_fullscreen = fullscreen_id() == Some(w.id);

    rsx! {
        div {
            id: "{w.id}",
            border_radius: "26px",
            overflow: "hidden",
            grid_column: if is_fullscreen { "1 / -1" } else { "auto" },
            min_width: "100%",
            width: "100%",
            transition: "box-shadow 0.4s cubic-bezier(0.33, 1, 0.68, 1)",
            box_shadow: if hovered() { "0 0 20px 4px rgba(20, 20, 20, 0.6)" } else { "0 0 12px 4px rgba(20, 20, 20, 0.4)" },

            div {
                display: "block",
                position: "relative",
                aspect_ratio: "16 / 9",
                overflow: "hidden",
                cursor: "pointer",
                onclick: move |_| {
                    if is_fullscreen {
                        fullscreen_id.set(None);
                    } else {
                        fullscreen_id.set(Some(w.id));
                    }
                },
                onmouseenter: move |_| hovered.set(true),
                onmouseleave: move |_| hovered.set(false),

                img {
                    width: "100%",
                    height: "100%",
                    object_fit: "cover",
                    display: "block",
                    src: "/wallpapers/{w.image_file.file_name}",
                    loading: "lazy",
                    transition: "transform 0.6s cubic-bezier(0.33, 1, 0.68, 1), filter 0.6s cubic-bezier(0.33, 1, 0.68, 1)",
                    transform: if hovered() { "scale(1.1)" } else { "scale(1)" },
                    filter: if hovered() { "brightness(1.1)" } else { "brightness(1)" },
                }

                div {
                    position: "absolute",
                    top: "0",
                    left: "0",
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

                        div {
                            display: "flex",
                            gap: "4px",
                            pointer_events: "auto",
                            IconButton {
                                color: (liked_signal() == LikedState::Loved).then_some(LOVED_COLOR),
                                icon: fa_solid_icons::FaHeart,
                                onclick: move |e: MouseEvent| {
                                    e.stop_propagation();
                                    update_like(LikedState::Loved);
                                },
                            }
                            IconButton {
                                color: (liked_signal() == LikedState::Liked).then_some(LIKED_COLOR),
                                icon: fa_solid_icons::FaThumbsUp,
                                onclick: move |e: MouseEvent| {
                                    e.stop_propagation();
                                    update_like(LikedState::Liked);
                                },
                            }
                            IconButton {
                                color: (liked_signal() == LikedState::Disliked).then_some(DISLIKED_COLOR),
                                icon: fa_solid_icons::FaThumbsDown,
                                onclick: move |e: MouseEvent| {
                                    e.stop_propagation();
                                    update_like(LikedState::Disliked);
                                },
                            }
                            IconButton {
                                icon: fa_solid_icons::FaArrowRotateLeft,
                                onclick: move |e: MouseEvent| {
                                    e.stop_propagation();
                                    recreate_action.call(w.id);
                                },
                            }
                            IconButton {
                                icon: fa_solid_icons::FaTrash,
                                onclick: move |e: MouseEvent| {
                                    e.stop_propagation();
                                    delete_action.call(w.id);
                                },
                            }
                        }
                    }

                    div { display: "flex", justify_content: "flex-start",
                        Pill {
                            color: match liked_signal() {
                                LikedState::Loved => Some(LOVED_COLOR),
                                LikedState::Liked => Some(LIKED_COLOR),
                                LikedState::Disliked => Some(DISLIKED_COLOR),
                                LikedState::Neutral => None,
                            },
                            text: w.prompt_data.shortened_prompt,
                        }
                    }
                }
            }

            GhostInput {
                value: comment_signal(),
                placeholder: "Add a note...",
                single_line: true,
                maxlength: 54,
                oninput: move |val: String| {
                    let comment = (!val.trim().is_empty()).then(|| val.clone());
                    comment_signal.set(val);
                    comment_action.call(w.id, comment);
                },
            }
        }
    }
}

#[component]
fn GhostInput(
    value: String,
    placeholder: &'static str,
    #[props(default = false)] single_line: bool,
    #[props(default = 1000)] maxlength: usize,
    oninput: EventHandler<String>,
) -> Element {
    let mut focused = use_signal(|| false);
    rsx! {
        textarea {
            maxlength,
            resize: "none",
            display: "block",
            width: "100%",
            rows: if single_line { "1" } else { "3" },
            font_size: "11px",
            font_weight: "bold",
            padding: "8px 20px",
            color: "black",
            background: format!("rgba(240, 240, 240, {})", if focused() { "0.7" } else { "0.3" }),
            backdrop_filter: "blur(20px)",
            border: "none",
            outline: "none",
            placeholder,
            onfocus: move |_| focused.set(true),
            onblur: move |_| focused.set(false),
            oninput: move |e| {
                let val = if single_line { e.value().replace('\n', "") } else { e.value() };
                oninput.call(val);
            },
            value,
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
            color: OVERLAY_TEXT_COLOR,
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
    let mut hovered = use_signal(|| false);
    let mut pressed = use_signal(|| false);

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
            color: OVERLAY_TEXT_COLOR,
            cursor: "pointer",
            pointer_events: "auto",
            transform: if pressed() { "scale(0.9)" } else if hovered() { "scale(1.2)" } else { "scale(1)" },
            filter: if hovered() { "brightness(1.4)" } else { "brightness(1)" },
            transition: "transform 0.15s ease, filter 0.15s ease",
            onmouseenter: move |_| hovered.set(true),
            onmouseleave: move |_| {
                hovered.set(false);
                pressed.set(false);
            },
            onmousedown: move |_| pressed.set(true),
            onmouseup: move |_| pressed.set(false),
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
