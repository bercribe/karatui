# Karatui

A terminal user interface for [Karakeep](https://karakeep.app/)

**Warning: this is still very WIP!**

Demo:
![Demo](./demo.svg)

I made this because I got tired of clicking around the web UI when I wanted to organize my bookmarks. Features:
- load bookmarks from a particular list
- add tags and lists to bookmarks
- remove tags and lists from bookmarks
- tag and list suggestions based on your existing bookmarks
- open links from the terminal

If you want to use this early version, you'll need to set some environment variables:
```
# set to your instance URL
export KARAKEEP_URL=https://try.karakeep.app
# generate from your server settings
export KARAKEEP_API_KEY=xxx_xxxxxxxxxxxxxxxxxxxx_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
# the list you want to load - this is shown in the URL when you select a list
export KARAKEEP_LIST_ID=xxxxxxxxxxxxxxxxxxxxxxxx
```
