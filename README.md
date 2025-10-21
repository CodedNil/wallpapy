<h1 align="center">
    Wallpapy
    <br>
    <a href="https://github.com/CodedNil/wallpapy/blob/master/LICENSE"><img src="https://img.shields.io/github/license/CodedNil/wallpapy"/></a>
    <a href="https://deps.rs/repo/github/CodedNil/wallpapy"><img src="https://deps.rs/repo/github/CodedNil/wallpapy/status.svg"/></a>
    <img src="https://img.shields.io/github/commit-activity/w/CodedNil/wallpapy"/>
    <img src="https://img.shields.io/github/last-commit/CodedNil/wallpapy"/>
    <img src="https://img.shields.io/github/actions/workflow/status/CodedNil/wallpapy/rust.yml"/>
    <br>
    <img src="https://img.shields.io/github/repo-size/CodedNil/wallpapy"/>
    <img src="https://img.shields.io/github/languages/code-size/CodedNil/wallpapy"/>
</h1>

Your personalised daily wallpaper generator

<img width="1872" height="2058" alt="image" src="https://github.com/user-attachments/assets/1189ff43-9c26-4a33-ae6d-6c155bde10e5" />


## Overview

Wallpapy uses AI models such as Gemini 2.5 Flash to generate prompts for new wallpapers in a style you choose, aiming to give a refreshing variety, and using Seedream 4.0 to generate the images. It provides api calls to fetch your liked images to use in OS extensions that can serve a random wallpaper to you every x hours.

## Features

- **User Guided:** You can provide feedback on the generated wallpapers to fine tune its outputs to a style you love.
- **Cost Effective:** Very efficient usage of ai models to cost no more than a couple pennies a day.
- **Web Application:** Access Wallpapy from any device with a web browser, ensuring a consistent and responsive experience across desktops, tablets, and smartphones.
- **Powered by `egui`:** Utilises the [`egui`](https://github.com/emilk/egui) library for a smooth and efficient graphical user interface experience.

## Getting Started

### Installation and Configuration
1. **Clone the Repository**
2. **Install the WebAssembly Target:** `rustup target add wasm32-unknown-unknown`
3. **Install wasm-bindgen and wasm-opt for building WASM applications**
4. **Install [Just](https://github.com/casey/just) for managing build commands:** `cargo install --locked just`
5. **Create Configuration File:** Copy the `.env-template` to `.env` and fill in your Gemini and Replicate details

### Build and Run Commands
- **Run the App in Desktop Mode:** `just`
- **Compile for WebAssembly:** `just build-web` or in release mode `just build-web-release`
- **Start the Server:** `just serve` or in release mode `just serve-release`

## Contributing
Contributions are welcome! If you'd like to contribute to Wallpapy, please fork the repository and submit a pull request with your improvements or bug fixes.
