# Karatui

A terminal user interface for [Karakeep](https://karakeep.app/)

**Warning: this is still WIP!**

Demo:
![Demo](./demo.svg)

I made this because I got tired of clicking around the web UI when I wanted to organize my bookmarks. Features:
- load bookmarks from a particular list
- add tags and lists to bookmarks
- remove tags and lists from bookmarks
- tag and list suggestions based on your existing bookmarks
- open links from the terminal

## Installation
You can build this from source with `cargo build`, or using nix with `nix build`.

## Config
karatui reads from ~/.config/karatui/karatui.toml. The config should appear as follows:
```
# set to your instance URL
url = "https://try.karakeep.app"
# the list you want to load - this is shown in the URL when you select a list
list_id = "xxxxxxxxxxxxxxxxxxxxxxxx"
# generate from your server settings
api_key_path = "/path/to/api_key"
```
