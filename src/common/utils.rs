use std::fmt::Display;

pub fn vec_str<T: Display>(items: &[T]) -> String {
    items
        .iter()
        .map(|item| format!("{item}"))
        .collect::<Vec<String>>()
        .join(", ")
}
