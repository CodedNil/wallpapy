default:
    dx serve --port 4560

build:
    dx build --release

# Fetch a rounded Material Symbol SVG from Google's repo into assets/icons/materialsymbolsrounded/<name>.svg
fetch-icon name:
    curl -L --fail "https://chromium.googlesource.com/external/github.com/google/material-design-icons/+/master/symbols/web/{{ name }}/materialsymbolsrounded/{{ name }}_wght700fill1_48px.svg?format=TEXT" | base64 -d > "assets/icons/{{ name }}.svg"
