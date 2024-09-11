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

![image](https://github.com/user-attachments/assets/db4a1f5e-125c-4f33-aa87-ffca5e023ef5)

## Overview

Wallpapy uses AI models such as OpenAI GPT-4o to generate prompts for new wallpapers in a style you choose, aiming to give a refreshing variety, and using FLUX.1 Schnell to generate the images. It provides api calls to fetch your liked images to use in OS extensions that can serve a random wallpaper to you every x hours.

## Features

- **User Guided:** You can provide feedback on the generated wallpapers to fine tune its outputs to a style you love.
- **Cost Effective:** Very efficient usage of LLM models and Flux to cost no more than a couple pennies a day.
- **Web Application:** Access HomeFlow from any device with a web browser, ensuring a consistent and responsive experience across desktops, tablets, and smartphones.
- **Powered by `egui`:** Utilises the [`egui`](https://github.com/emilk/egui) library for a smooth and efficient graphical user interface experience.

## Getting Started

### Installation and Configuration
1. **Clone the Repository**
2. **Install the WebAssembly Target:** `rustup target add wasm32-unknown-unknown`
3. **Install Trunk [Trunk](https://github.com/trunk-rs/trunk) for building WASM applications** `cargo install --locked trunk`
4. **Install [Just](https://github.com/casey/just) for managing build commands:** `cargo install --locked just`
5. **Create Configuration File:** Copy the `.env-template` to `.env` and fill in your Home Assistant details

### Build and Run Commands
- **Run the App in Desktop Mode:** `just`
- **Compile for WebAssembly:** `just build-web` or in release mode `just build-web-release`
- **Start the Server:** `just serve` or in release mode `just serve-release`

## Contributing
Contributions are welcome! If you'd like to contribute to HomeFlow, please fork the repository and submit a pull request with your improvements or bug fixes.
