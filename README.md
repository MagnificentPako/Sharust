# Sharust
> Share + Rust

Sharust is a WIP file-uploader (mainly with images in mind though) which aims to be somewhat sharex/sharenix-esque, but not quite. The configuration file looks a bit different, and not all the features are implemented yet. So don't use this if you expect a working drop-in replacement for Sharenix/ShareX on Linux. 
> Wait, Linux you say?

Yes, this only runs on Linux. And it will likely never run on Windows, unless someone want's to open a PR.

## Dependencies
Other than the stuff inside the _Cargo.toml_ this project also depends on _maim, xclip, xdg-open, libnotify_ and soon _xdotool_ in order to provide you with somewhat native features.

## Installation
Clone the repo and run `cargo install`. That should install sharust into your cargo bin folder.

## Usage
This was somewhat written with modularity in mind. It contains some upload "helpers" and a command for you to actually upload the file. When running the program with the `--help` flag or the `help` subcommand you might have noticed that it's rather empty. That's because Sharust mainly shoulders the uploading of your beloved files, everything else is up to you. I provide you with two (as the time of writing) standard features for screenshots though which you can use by calling `sharust -m <option>`. The available options are _full_ and _area_, which both take a screenshot, one is fullscreen, the other one an area you select with _slop_. Note that notifications are only enabled/available for these two, while uploading through `sharust upload <file>` directly won't open a notification. That's because `upload` will write the extracted URL to stdout so other programs can easily grab and reuse them.