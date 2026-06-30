use crate::common::{GenerationEvent, GenerationStage, LikedState, WallpaperData};
use crate::server_functions::{
    action_comment, action_generate, action_like, load_gallery_data, stream_generation_events,
};
use chrono::{DateTime, Utc};
use dioxus::prelude::*;
use std::collections::HashSet;
use uuid::Uuid;

const NEUTRAL_COLOR: &str = "10, 10, 10";
const LOVED_COLOR: &str = "160, 100, 10";
const LIKED_COLOR: &str = "20, 80, 30";
const DISLIKED_COLOR: &str = "90, 15, 15";
const FILTER_ACTIVE_COLOR: &str = "160, 100, 10";
const OVERLAY_OPACITY: &str = "0.7";
const OVERLAY_TEXT_COLOR: &str = "rgba(255, 255, 255, 0.9)";

#[derive(Clone, Copy, PartialEq)]
enum TimeOfDay {
    Night,
    Sunrise,
    Day,
}

impl TimeOfDay {
    const fn brightness_range(self) -> (f32, f32) {
        match self {
            Self::Night => (0.0, 0.5),
            Self::Sunrise => (0.5, 0.65),
            Self::Day => (0.65, 1.0),
        }
    }
}

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
        dt.format("%d/%m/%Y %-I%P").to_string()
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
        document::Link {
            rel: "stylesheet",
            href: "https://fonts.googleapis.com/css2?family=Inter:wght@300..700&display=swap",
        }
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
                    font-family: 'Inter', sans-serif;
                    font-size: 15px;
                    font-weight: 700;
                    min-height: 100vh;
                    user-select: none;
                }}

                *, input, textarea, button {{
                    font-family: inherit;
                    font-size: inherit;
                    font-weight: inherit;
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
            will_change: "transform",
        }
        div { GalleryPage {} }
    }
}

macro_rules! icon_svg {
    ($name:literal) => {
        include_str!(concat!("../assets/icons/", $name, ".svg"))
    };
}

#[component]
fn GalleryPage() -> Element {
    let mut data = use_server_future(load_gallery_data)?;
    let mut expanded_id: Signal<Option<Uuid>> = use_signal(|| None);
    let like_filter: Signal<Option<LikedState>> = use_signal(|| None);
    let time_filter: Signal<Option<TimeOfDay>> = use_signal(|| None);

    let Some(Ok(gallery)) = data() else {
        return rsx! {
            p { "Loading..." }
        };
    };

    let items = gallery.into_iter().filter(|w| {
        let like_ok = like_filter().is_none_or(|f| w.liked_state == f);
        let time_ok = time_filter().is_none_or(|f| {
            let (lo, hi) = f.brightness_range();
            w.image_brightness >= lo && w.image_brightness <= hi
        });
        like_ok && time_ok
    });

    rsx! {
        div {
            onkeydown: move |e| {
                if e.key() == Key::Escape {
                    expanded_id.set(None);
                }
            },

            if expanded_id().is_some() {
                div {
                    position: "fixed",
                    top: "0",
                    left: "0",
                    right: "0",
                    bottom: "0",
                    z_index: "498",
                    background: "rgba(0, 0, 0, 0.6)",
                    backdrop_filter: "blur(6px)",
                    tabindex: "-1",
                    onmounted: move |e| async move {
                        let _ = e.set_focus(true).await;
                    },
                    onclick: move |_| expanded_id.set(None),
                }
            }

            div {
                display: "grid",
                grid_template_columns: "repeat(auto-fill, minmax(360px, 1fr))",
                gap: "20px",
                padding: "20px",
                for w in items {
                    WallpaperCard { key: "{w.id}", w, expanded_id }
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

                FilterBar { like_filter, time_filter }

                GenerateButton {}
            }
        }
    }
}

#[component]
fn FilterBar(
    mut like_filter: Signal<Option<LikedState>>,
    mut time_filter: Signal<Option<TimeOfDay>>,
) -> Element {
    rsx! {
        div {
            display: "flex",
            gap: "6px",
            padding: "8px 12px",
            background: "rgba(255, 255, 255, 0.2)",
            backdrop_filter: "blur(20px)",
            border_radius: "12px",
            border: "1px solid rgba(255, 255, 255, 0.3)",
            box_shadow: "0 8px 32px rgba(0, 0, 0, 0.3)",
            align_items: "center",

            IconButton {
                color: (like_filter() == Some(LikedState::Loved)).then_some(FILTER_ACTIVE_COLOR),
                svg: icon_svg!("sentiment_excited"),
                onclick: move |_| {
                    like_filter
                        .set(
                            (like_filter() != Some(LikedState::Loved)).then_some(LikedState::Loved),
                        );
                },
            }
            IconButton {
                color: (like_filter() == Some(LikedState::Liked)).then_some(FILTER_ACTIVE_COLOR),
                svg: icon_svg!("sentiment_satisfied"),
                onclick: move |_| {
                    like_filter
                        .set(
                            (like_filter() != Some(LikedState::Liked)).then_some(LikedState::Liked),
                        );
                },
            }
            IconButton {
                color: (like_filter() == Some(LikedState::Neutral)).then_some(FILTER_ACTIVE_COLOR),
                svg: icon_svg!("sentiment_neutral"),
                onclick: move |_| {
                    like_filter
                        .set(
                            (like_filter() != Some(LikedState::Neutral))
                                .then_some(LikedState::Neutral),
                        );
                },
            }
            IconButton {
                color: (like_filter() == Some(LikedState::Disliked)).then_some(FILTER_ACTIVE_COLOR),
                svg: icon_svg!("sentiment_dissatisfied"),
                onclick: move |_| {
                    like_filter
                        .set(
                            (like_filter() != Some(LikedState::Disliked))
                                .then_some(LikedState::Disliked),
                        );
                },
            }

            div {
                width: "1px",
                align_self: "stretch",
                margin: "2px 2px",
                background: "rgba(255, 255, 255, 0.3)",
            }

            IconButton {
                color: (time_filter() == Some(TimeOfDay::Night)).then_some(FILTER_ACTIVE_COLOR),
                svg: icon_svg!("bedtime"),
                onclick: move |_| {
                    time_filter
                        .set((time_filter() != Some(TimeOfDay::Night)).then_some(TimeOfDay::Night));
                },
            }
            IconButton {
                color: (time_filter() == Some(TimeOfDay::Sunrise)).then_some(FILTER_ACTIVE_COLOR),
                svg: icon_svg!("wb_twilight"),
                onclick: move |_| {
                    time_filter
                        .set(
                            (time_filter() != Some(TimeOfDay::Sunrise)).then_some(TimeOfDay::Sunrise),
                        );
                },
            }
            IconButton {
                color: (time_filter() == Some(TimeOfDay::Day)).then_some(FILTER_ACTIVE_COLOR),
                svg: icon_svg!("sunny"),
                onclick: move |_| {
                    time_filter
                        .set((time_filter() != Some(TimeOfDay::Day)).then_some(TimeOfDay::Day));
                },
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
            button {
                padding: "10px 14px",
                flex_grow: "1",
                border_radius: "8px 8px 0 0",
                border: "none",
                color: "white",
                cursor: "pointer",
                font_size: "14px",
                font_weight: "900",
                background: "rgba(80, 140, 90)",
                onmousedown: move |_| pressed.set(true),
                onmouseup: move |_| pressed.set(false),
                onclick: move |_| {
                    let p = prompt();
                    action.call(if p.trim().is_empty() { None } else { Some(p) });
                    prompt.set(String::new());
                },
                "Generate"
            }
            input {
                style: "width: 100%; border-radius: 0 0 8px 8px; background: rgba(100, 160, 110); border: none; outline: none; color: white; padding: 8px 10px; text-align: center;",
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
            color: "white",
            max_width: "40vw",
            overflow_wrap: "break-word",
            box_shadow: "0 4px 12px rgba(0, 0, 0, 0.1)",
            "{text}"
        }
    }
}

#[component]
fn WallpaperCard(w: WallpaperData, mut expanded_id: Signal<Option<Uuid>>) -> Element {
    let mut like_action = use_action(action_like);
    let mut comment_action = use_action(action_comment);

    let mut deleted = use_signal(|| false);
    let mut liked = use_signal(|| w.liked_state);
    let mut comment = use_signal(|| w.comment.unwrap_or_default());
    let mut hovered = use_signal(|| false);
    let mut open_anim = use_signal(|| false);
    let mut start_rect: Signal<Option<(f64, f64, f64, f64)>> = use_signal(|| None);
    let mut end_rect: Signal<Option<(f64, f64, f64, f64)>> = use_signal(|| None);
    let mut is_closing = use_signal(|| false);
    let mut card_element = use_signal(|| None);

    let is_expanded = expanded_id() == Some(w.id);
    let is_active = is_expanded || is_closing();

    use_effect(move || {
        let currently_expanded = expanded_id() == Some(w.id);
        let rect_set = start_rect().is_some();
        if currently_expanded && !open_anim() && !is_closing() {
            spawn(async move {
                gloo_timers::future::TimeoutFuture::new(50).await;
                open_anim.set(true);
            });
        } else if !currently_expanded && rect_set && !is_closing() && open_anim() {
            is_closing.set(true);
            open_anim.set(false);
            spawn(async move {
                gloo_timers::future::TimeoutFuture::new(450).await;
                start_rect.set(None);
                end_rect.set(None);
                is_closing.set(false);
            });
        }
    });

    let mut update_like = move |target: LikedState| {
        let new_state = if liked() == target {
            LikedState::Neutral
        } else {
            target
        };
        liked.set(new_state);
        like_action.call(w.id, new_state);
    };

    let (pos_top, pos_left, pos_width) = match (is_active, open_anim()) {
        (true, false) => {
            let (t, l, w, _) = start_rect().unwrap_or((0.0, 0.0, 0.0, 0.0));
            (format!("{t}px"), format!("{l}px"), format!("{w}px"))
        }
        (true, true) => {
            let (t, l, w, _) = end_rect().unwrap_or((0.0, 0.0, 0.0, 0.0));
            (format!("{t}px"), format!("{l}px"), format!("{w}px"))
        }
        _ => ("auto".to_string(), "auto".to_string(), "100%".to_string()),
    };

    if deleted() {
        return rsx! {};
    }

    rsx! {
        if is_active {
            div {
                width: "100%",
                height: start_rect().map_or_else(|| "0px".to_string(), |r| format!("{}px", r.3)),
            }
        }

        div {
            id: "{w.id}",
            onmounted: move |element| card_element.set(Some(element.data())),
            border_radius: "26px",
            overflow: "hidden",
            position: if is_active { "fixed" } else { "relative" },
            top: pos_top,
            left: pos_left,
            width: pos_width,
            z_index: if is_active { "499" } else { "auto" },
            transition: if is_active && (open_anim() || is_closing()) { "top 0.4s cubic-bezier(0.33, 1, 0.68, 1), left 0.4s cubic-bezier(0.33, 1, 0.68, 1), width 0.4s cubic-bezier(0.33, 1, 0.68, 1), box-shadow 0.4s ease" } else { "box-shadow 0.4s cubic-bezier(0.33, 1, 0.68, 1)" },
            will_change: if is_active && (open_anim() || is_closing()) { "top, left, width" } else { "auto" },
            box_shadow: if is_active { "0 24px 80px rgba(0, 0, 0, 0.8)" } else if hovered() { "0 0 20px 4px rgba(20, 20, 20, 0.6)" } else { "0 0 12px 4px rgba(20, 20, 20, 0.4)" },

            div {
                id: "img-{w.id}",
                display: "block",
                position: "relative",
                aspect_ratio: "16 / 9",
                overflow: "hidden",
                cursor: if is_active { "default" } else { "pointer" },
                onmouseenter: move |_| hovered.set(true),
                onmouseleave: move |_| hovered.set(false),
                onclick: move |_| async move {
                    if !is_active {
                        if let Some(element) = card_element.cloned()
                            && let Ok(r) = element.get_client_rect().await
                            && let Ok((vw, vh)) = document::eval(
                                "return [window.innerWidth, window.innerHeight]",
                            )
                            .join::<(f64, f64)>()
                            .await
                        {
                            let sw = r.size.width;
                            let sh = r.size.height;
                            let ew = f64::min(
                                vw * 0.92,
                                vh.mul_add(0.92, -(sh - sw * 9.0 / 16.0)) * 16.0 / 9.0,
                            );
                            let eh = ew * 9.0 / 16.0 + (sh - sw * 9.0 / 16.0);
                            start_rect.set(Some((r.origin.y, r.origin.x, sw, sh)));
                            end_rect.set(Some(((vh - eh) / 2.0, (vw - ew) / 2.0, ew, eh)));
                        }
                        expanded_id.set(Some(w.id));
                    }
                },

                img {
                    width: "100%",
                    height: "100%",
                    object_fit: "cover",
                    display: "block",
                    loading: "lazy",
                    draggable: "false",
                    src: "/wallpapers/{w.image_file}",
                    transition: "transform 0.6s cubic-bezier(0.33, 1, 0.68, 1), filter 0.6s cubic-bezier(0.33, 1, 0.68, 1)",
                    transform: if hovered() && !is_active { "scale(1.1)" } else { "scale(1.01)" },
                    filter: if hovered() && !is_active { "brightness(1.1)" } else { "brightness(1)" },
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
                                svg: icon_svg!("sentiment_excited"),
                                onclick: move |e: MouseEvent| {
                                    e.stop_propagation();
                                    update_like(LikedState::Loved);
                                },
                            }
                            IconButton {
                                color: like_color(liked()).filter(|_| liked() == LikedState::Liked),
                                svg: icon_svg!("sentiment_satisfied"),
                                onclick: move |e: MouseEvent| {
                                    e.stop_propagation();
                                    update_like(LikedState::Liked);
                                },
                            }
                            IconButton {
                                color: like_color(liked()).filter(|_| liked() == LikedState::Disliked),
                                svg: icon_svg!("sentiment_dissatisfied"),
                                onclick: move |e: MouseEvent| {
                                    e.stop_propagation();
                                    deleted.set(true);
                                    update_like(LikedState::Disliked);
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
                            text: if is_active { "{w.prompt}" } else { "{w.shortened_prompt}" },
                            onclick: move |_| {
                                let text = if is_active {
                                    w.prompt.clone()
                                } else {
                                    w.shortened_prompt.clone()
                                };
                                async move {
                                    let eval = document::eval(
                                        "navigator.clipboard.writeText(await dioxus.recv())",
                                    );
                                    let _ = eval.send(text);
                                    let _ = eval.await;
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
                maxlength: 200,
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
            font_size: "0.8em",
            padding: "8px 20px",
            color: "black",
            background: format!("rgba(240, 240, 240, {})", if focused() { "0.7" } else { "0.3" }),
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
            font_size: "0.8em",
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
fn IconButton(
    color: Option<&'static str>,
    svg: &'static str,
    onclick: EventHandler<MouseEvent>,
) -> Element {
    let mut hovered = use_signal(|| false);
    let mut pressed = use_signal(|| false);

    rsx! {
        button {
            width: "26px",
            height: "26px",
            border_radius: "20px",
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
            span {
                style: "width: 14px; height: 14px; display: block;",
                aria_hidden: "true",
                dangerous_inner_html: "{svg}",
            }
        }
    }
}
