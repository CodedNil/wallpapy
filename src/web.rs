use crate::common::{GenerationEvent, GenerationStage, LikedState, WallpaperData};
use crate::server_functions::{
    action_comment, action_delete, action_generate, action_like, action_styles, load_gallery_data,
    stream_generation_events,
};
use chrono::{DateTime, Utc};
use dioxus::prelude::*;
use dioxus_free_icons::{Icon, IconShape, icons::fa_solid_icons};
use std::collections::HashSet;

const NEUTRAL_COLOR: &str = "10, 10, 10";
const LOVED_COLOR: &str = "160, 100, 10";
const LIKED_COLOR: &str = "20, 80, 30";
const DISLIKED_COLOR: &str = "90, 15, 15";
const OVERLAY_OPACITY: &str = "0.7";
const OVERLAY_TEXT_COLOR: &str = "rgba(255, 255, 255, 0.9)";

const fn like_color(state: LikedState) -> Option<&'static str> {
    match state {
        LikedState::Loved => Some(LOVED_COLOR),
        LikedState::Liked => Some(LIKED_COLOR),
        LikedState::Disliked => Some(DISLIKED_COLOR),
        LikedState::Neutral => None,
    }
}

fn format_age(dt: DateTime<Utc>) -> String {
    let diff = Utc::now().signed_duration_since(dt);
    let plural = |n: i64, unit: &str| format!("{n} {unit}{} ago", if n == 1 { "" } else { "s" });
    if diff.num_weeks() >= 1 || diff.num_milliseconds() < 0 {
        dt.format("%d/%m/%Y %I%P").to_string()
    } else if diff.num_days() >= 1 {
        plural(diff.num_days(), "day")
    } else if diff.num_hours() >= 1 {
        plural(diff.num_hours(), "hour")
    } else if diff.num_minutes() >= 1 {
        plural(diff.num_minutes(), "minute")
    } else {
        "just now".to_string()
    }
}

pub fn app() -> Element {
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
                    color: white;
                    font-family: sans-serif;
                    font-size: 14px;
                    min-height: 100vh;
                    user-select: none;
                }}
            "#
        }
        img {
            src: "/smartget",
            position: "fixed",
            top: "0",
            left: "0",
            width: "100%",
            height: "100%",
            object_fit: "cover",
            z_index: "-1",
            filter: "blur(40px) brightness(0.8)",
            transform: "scale(1.02)",
        }
        div { GalleryPage {} }
    }
}

#[component]
fn GalleryPage() -> Element {
    let mut styles_action = use_action(action_styles);
    let mut data = use_server_future(load_gallery_data)?;

    let Some(Ok(gallery)) = data() else {
        return rsx! {
            p { "Loading..." }
        };
    };

    rsx! {
        div {
            display: "grid",
            grid_template_columns: "repeat(auto-fill, minmax(360px, 1fr))",
            gap: "20px",
            padding: "20px",
            for w in gallery.items {
                WallpaperCard { key: "{w.id}", w }
            }
        }

        div {
            position: "fixed",
            bottom: "20px",
            right: "20px",
            z_index: "100",
            display: "flex",
            flex_direction: "column",
            align_items: "flex-end",
            gap: "10px",

            EventsPanel { on_image_received: move || data.restart() }

            div {
                display: "flex",
                gap: "16px",
                padding: "12px",
                background: "rgba(255, 255, 255, 0.2)",
                backdrop_filter: "blur(20px)",
                border_radius: "16px",
                border: "1px solid rgba(255, 255, 255, 0.3)",
                align_items: "stretch",
                box_shadow: "0 8px 32px rgba(0, 0, 0, 0.3)",

                div {
                    display: "flex",
                    width: "400px",
                    border_radius: "16px",
                    overflow: "hidden",
                    GhostInput {
                        value: gallery.style_prompt,
                        placeholder: "Style prompt...",
                        oninput: move |val| styles_action.call(val),
                    }
                }
                GenerateButton {}
            }
        }
    }
}

#[component]
fn GenerateButton() -> Element {
    let mut action = use_action(action_generate);
    let mut prompt = use_signal(String::new);
    let mut hovered = use_signal(|| false);
    let mut pressed = use_signal(|| false);

    rsx! {
        div {
            display: "flex",
            flex_direction: "column",
            width: "200px",
            filter: if hovered() { "brightness(1.1)" } else { "brightness(1)" },
            transform: if pressed() { "scale(0.98)" } else { "scale(1)" },
            transition: "filter 0.15s ease, transform 0.1s ease",
            onmouseenter: move |_| hovered.set(true),
            onmouseleave: move |_| {
                hovered.set(false);
                pressed.set(false);
            },
            onmousedown: move |_| pressed.set(true),
            onmouseup: move |_| pressed.set(false),
            onclick: move |_| {
                let p = prompt();
                action.call(if p.trim().is_empty() { None } else { Some(p) });
                prompt.set(String::new());
            },
            button {
                padding: "10px 14px",
                flex_grow: "1",
                border_radius: "8px 8px 0 0",
                border: "none",
                color: "white",
                cursor: "pointer",
                font_size: "14px",
                font_weight: "bolder",
                background: "rgba(80, 140, 90)",
                "Generate"
            }
            input {
                style: "width: 100%; border-radius: 0 0 8px 8px; background: rgba(100, 160, 110); border: none; outline: none; color: white; font-size: 13px; padding: 8px 10px;",
                placeholder: "Custom prompt...",
                value: prompt(),
                oninput: move |e| prompt.set(e.value()),
            }
        }
    }
}

#[component]
fn EventsPanel(on_image_received: EventHandler<()>) -> Element {
    let mut cached_events: Signal<Vec<GenerationEvent>> = use_signal(Vec::new);

    use_future(move || async move {
        let Ok(mut stream) = stream_generation_events().await else {
            return;
        };
        let mut handled: HashSet<uuid::Uuid> = HashSet::new();
        while let Some(Ok(snapshot)) = stream.recv().await {
            for event in &snapshot {
                if event.stage == GenerationStage::ReceivedImage && handled.insert(event.id) {
                    on_image_received.call(());
                }
            }
            handled.retain(|id| snapshot.iter().any(|e| e.id == *id));
            cached_events.set(snapshot);
        }
    });

    rsx! {
        div {
            display: "flex",
            flex_direction: "column",
            align_items: "flex-end",
            gap: "6px",
            for event in cached_events() {
                GenerationEventView { key: "{event.id}", event }
            }
        }
    }
}

#[component]
fn GenerationEventView(event: GenerationEvent) -> Element {
    let mut tick = use_signal(|| 0u32);
    use_future(move || async move {
        loop {
            gloo_timers::future::TimeoutFuture::new(1000).await;
            *tick.write() += 1;
        }
    });

    let _ = tick();
    let elapsed = Utc::now()
        .signed_duration_since(event.start_time)
        .num_seconds();
    let text = match &event.stage {
        GenerationStage::WaitingForPrompt => {
            format!("Generating image started {elapsed}s ago, creating prompt...")
        }
        GenerationStage::ReceivedPrompt { prompt } => {
            format!("Generating image started {elapsed}s ago, prompt: \"{prompt}\"")
        }
        GenerationStage::ReceivedImage => "Image received! Refreshing gallery...".to_string(),
        GenerationStage::Failed { reason } => {
            format!("Failed: {reason}")
        }
    };

    rsx! {
        div {
            padding: "6px 12px",
            background: "rgba(10, 10, 10, 0.4)",
            backdrop_filter: "blur(10px)",
            border_radius: "8px",
            font_size: "12px",
            font_weight: "bold",
            color: "white",
            white_space: "nowrap",
            box_shadow: "0 4px 12px rgba(0, 0, 0, 0.1)",
            "{text}"
        }
    }
}

#[component]
fn WallpaperCard(w: WallpaperData) -> Element {
    let mut like_action = use_action(action_like);
    let mut delete_action = use_action(action_delete);
    let mut comment_action = use_action(action_comment);

    let mut deleted = use_signal(|| false);
    let mut liked = use_signal(|| w.liked_state);
    let mut comment = use_signal(|| w.comment.clone().unwrap_or_default());
    let mut hovered = use_signal(|| false);

    let mut update_like = move |target: LikedState| {
        let new_state = if liked() == target {
            LikedState::Neutral
        } else {
            target
        };
        liked.set(new_state);
        like_action.call(w.id, new_state);
    };

    if deleted() {
        return rsx! {};
    }

    rsx! {
        div {
            id: "{w.id}",
            border_radius: "26px",
            overflow: "hidden",
            width: "100%",
            transition: "box-shadow 0.4s cubic-bezier(0.33, 1, 0.68, 1)",
            box_shadow: if hovered() { "0 0 20px 4px rgba(20, 20, 20, 0.6)" } else { "0 0 12px 4px rgba(20, 20, 20, 0.4)" },

            div {
                id: "img-{w.id}",
                display: "block",
                position: "relative",
                aspect_ratio: "16 / 9",
                overflow: "hidden",
                cursor: "pointer",
                onmouseenter: move |_| hovered.set(true),
                onmouseleave: move |_| hovered.set(false),

                img {
                    width: "100%",
                    height: "100%",
                    object_fit: "cover",
                    display: "block",
                    loading: "lazy",
                    draggable: "false",
                    src: "/wallpapers/{w.image_file.file_name}",
                    transition: "transform 0.6s cubic-bezier(0.33, 1, 0.68, 1), filter 0.6s cubic-bezier(0.33, 1, 0.68, 1)",
                    transform: if hovered() { "scale(1.1)" } else { "scale(1.01)" },
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

                        Pill { text: format_age(w.datetime) }

                        div {
                            display: "flex",
                            gap: "4px",
                            pointer_events: "auto",
                            IconButton {
                                color: like_color(liked()).filter(|_| liked() == LikedState::Loved),
                                icon: fa_solid_icons::FaHeart,
                                onclick: move |e: MouseEvent| {
                                    e.stop_propagation();
                                    update_like(LikedState::Loved);
                                },
                            }
                            IconButton {
                                color: like_color(liked()).filter(|_| liked() == LikedState::Liked),
                                icon: fa_solid_icons::FaThumbsUp,
                                onclick: move |e: MouseEvent| {
                                    e.stop_propagation();
                                    update_like(LikedState::Liked);
                                },
                            }
                            IconButton {
                                color: like_color(liked()).filter(|_| liked() == LikedState::Disliked),
                                icon: fa_solid_icons::FaThumbsDown,
                                onclick: move |e: MouseEvent| {
                                    e.stop_propagation();
                                    update_like(LikedState::Disliked);
                                },
                            }
                            IconButton {
                                icon: fa_solid_icons::FaTrash,
                                onclick: move |e: MouseEvent| {
                                    e.stop_propagation();
                                    deleted.set(true);
                                    delete_action.call(w.id);
                                },
                            }
                        }
                    }

                    div {
                        display: "flex",
                        flex_direction: "column",
                        gap: "4px",
                        pointer_events: "auto",

                        Pill {
                            color: like_color(liked()),
                            text: w.prompt_data.shortened_prompt.clone(),
                            onclick: {
                                let prompt = w.prompt_data.shortened_prompt;
                                move |_| {
                                    if let Some(window) = web_sys::window() {
                                        let _ = window.navigator().clipboard().write_text(&prompt);
                                    }
                                }
                            },
                        }
                    }
                }
            }

            GhostInput {
                value: comment(),
                placeholder: "Add a note...",
                single_line: true,
                maxlength: 54,
                oninput: move |val: String| {
                    let saved = (!val.trim().is_empty()).then(|| val.clone());
                    comment.set(val);
                    comment_action.call(w.id, saved);
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
                oninput.call(if single_line { e.value().replace('\n', "") } else { e.value() });
            },
            value,
        }
    }
}

#[component]
fn Pill(
    color: Option<&'static str>,
    text: String,
    onclick: Option<EventHandler<MouseEvent>>,
) -> Element {
    rsx! {
        span {
            padding: "6px 10px",
            border_radius: "12px",
            backdrop_filter: "blur(8px)",
            font_size: "11px",
            font_weight: "bold",
            background: format!("rgba({}, {OVERLAY_OPACITY})", color.unwrap_or(NEUTRAL_COLOR)),
            color: OVERLAY_TEXT_COLOR,
            onclick: move |e| {
                if let Some(handler) = onclick {
                    e.stop_propagation();
                    handler.call(e);
                }
            },
            cursor: if onclick.is_some() { "pointer" } else { "default" },
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
    rsx! {
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
    }
}
