extern crate xdg;
extern crate sha2;
#[macro_use]
extern crate clap;
extern crate hyper;
extern crate serde;
extern crate multipart;
extern crate serde_json;
extern crate notify_rust;
#[macro_use]
extern crate serde_derive;
extern crate hyper_native_tls;

use clap::App;
use hyper::Url;
use std::fs::File;
use std::io::prelude::*;
use std::process::Stdio;
use std::process::Command;
use hyper::method::Method;
use sha2::{Sha512, Digest};
use hyper::client::Request;
use notify_rust::Notification;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use hyper::net::HttpsConnector;
use multipart::client::Multipart;
use hyper_native_tls::NativeTlsClient;

// The different ways one is able to go from "upload the image" to "url pointing to the image"
#[derive(Serialize, Deserialize, Debug, Clone)]
enum ResponseType {
    Text,
    Redirect,
    Regex,
}

// The method used for uploading. Currently only supports HTTP POST
#[derive(Serialize, Deserialize, Debug, Clone)]
enum SharustMethod {
    Post,
}

// A structure describing a providier. A providier is the place Sharust will upload the image to (files probably work too)
#[derive(Serialize, Deserialize, Debug, Clone)]
struct SharustProvider {
    name: String,
    request_type: SharustMethod,
    request_url: String,
    file_form_name: String,
    arguments: HashMap<String, String>,
    response_type: ResponseType,
    regex_list: Vec<String>,
    url: String,
}

// Combines all the stuff above into one wonderful mess
#[derive(Serialize, Deserialize, Debug)]
struct SharustConfig {
    image_uploader: String,
    provider: Vec<SharustProvider>,
}

// The main function. duh.
fn main() {
    // Load the .yml describing the app
    let yaml = load_yaml!("clap.yml");
    // Parse said YML
    let matches = App::from_yaml(yaml).get_matches();
    // Obtain XDG compliant directories, prefixed with "sharust"
    let xdg_dirs = xdg::BaseDirectories::with_prefix("sharust").unwrap();

    // Create the config
    let config_path = xdg_dirs.place_config_file("sharust.json").expect("Cannot create config directory.");
    let config = load_config(config_path);

    if let Some(matches) = matches.subcommand_matches("upload") {
        // Probably called from maim
        let input = matches.value_of("INPUT").unwrap();
        // Get provider matching the selected one in the config
        let mut provider_iter = config.provider.clone().into_iter().filter(|x| x.name == config.image_uploader);
        let provider = provider_iter.next().unwrap();
        // Upload the file
        let response = upload_file(provider, String::from(input)).unwrap();
        std::io::stdout().write_all(response.as_bytes()).unwrap();
    } else {
        // How the user _should_ interface with sharust
        // Depending on the mode selected, it defines some arguments for maim
        let mode = matches.value_of("mode").unwrap_or("full");
        // Call maim to take the screenshot for you
        take_picture(mode);
        // Save taken image in supposedly unique file
        save_taken(xdg_dirs.clone());
        // Call itself, but now in the "uploading" mode giving it the file it first created with maim
        let out_url = upload_taken();
        // Finally open a notification with some usefull stuff
        open_notification(&out_url);
    }
}

fn load_config(path: PathBuf) -> SharustConfig {
    let mut config_file = match File::open(path.clone()) {
        // Return config file if it already exists
        Ok(conf) => conf,
        // If the config file doesn't exist, create one with default options
        Err(_) => {
            let mut c_file = File::create(path).unwrap();
            c_file.write_all(serde_json::to_string_pretty::<SharustConfig>(&SharustConfig {
                image_uploader: String::from("Uploader"),
                provider: [
                    SharustProvider {
                        name: String::from("Uploader"),
                        request_type: SharustMethod::Post,
                        request_url: String::from("https://some.url"),
                        file_form_name: String::from("file"),
                        arguments: HashMap::new(),
                        response_type: ResponseType::Text,
                        regex_list: [].to_vec(),
                        url: String::from("url")
                    },
                ].to_vec(),
            }).unwrap().as_bytes()).unwrap();
            println!("Created the config file. Please update it with valid data.");
            std::process::exit(0);
        }
    };
    // Read the config
    let mut config_contents = String::new();
    config_file.read_to_string(&mut config_contents).unwrap();
    // Parse the config using serde into a SharustConfig struct
    serde_json::from_str::<SharustConfig>(config_contents.as_str()).unwrap()
}

fn upload_file(provider: SharustProvider, path: String) -> Result<String, String> {
    // Create connection to provided url, using SSL and all the fancy stuff.
    let url: Url = provider.request_url.parse().expect("Failed to parse URL");

    let ssl = NativeTlsClient::new().unwrap();
    let connector = HttpsConnector::new(ssl);
    let request = Request::with_connector(Method::Post, url, &connector).unwrap();

    // Construct the multipart thing Sharust will send
    let mut multi = Multipart::from_request(request).expect("Failed to create Multipart");

    multi.write_file(provider.file_form_name, path).expect("FAILED TO WRITE FILE");
    for (arg_name, arg_val) in &provider.arguments {
        multi.write_text(arg_name, arg_val).unwrap();
    }

    // Actually send it
    let mut response = multi.send().expect("Failed to send multipart request");

    // Read response into a string
    let mut response_text = String::new();
    response.read_to_string(&mut response_text).unwrap();
    match response_text.len() {
        0 => Err(String::from("Something went wrong.")),
        _ => Ok(response_text)
    }
}

fn upload_taken() -> String {
    let out = Command::new("sharust")
        .args(&["upload","/tmp/mynewimage.png"])
        .output()
        .expect("Failed to execute process");
    let stdout = out.stdout.clone();
    String::from_utf8(stdout).unwrap()
}

fn take_picture(mode: &str) {
    let args: Vec<&str> = match mode {
        "full" => ["/tmp/mynewimage.png"].to_vec(),
        "area" => ["/tmp/mynewimage.png","-s","-c 1,0,1,0.1","-l"].to_vec(),
        _ => unreachable!(),
    };

    println!("{:?}",args);
    Command::new("maim")
            .args(args.into_iter().map(|x| String::from(x)).collect::<Vec<String>>())
            .output()
            .expect("Failed to execute process");
}

fn save_taken(xdg: xdg::BaseDirectories) {
    // generate name
    let mut orig_file = File::open("/tmp/mynewimage.png").unwrap();
    let mut orig_bytes: Vec<u8> = Vec::new();
    orig_file.read_to_end(&mut orig_bytes).unwrap();
    let mut hasher = Sha512::default();
    hasher.input(orig_bytes.as_slice());
    let output = hasher.result().into_iter().map(|x| format!("{:02x}",x).to_string()).collect::<String>();
    std::fs::copy(
        Path::new("/tmp/mynewimage.png"),
        xdg.place_data_file(format!("{}.png", output).as_str()).unwrap()
    ).unwrap();
}

fn open_notification(url: &str) {
    Notification::new()
    .summary("Upload successful!")
    .action("open", "Open image")
    .action("copy", "Copy to clipboard")
    .show()
    .unwrap()
    .wait_for_action({|action|
        match action {
            "open" => {
                Command::new("xdg-open")
                    .arg(url)
                    .output().
                    expect("derp");
            },
            "copy" => {
                let clip_command = Command::new("xclip")
                    .args(&["-selection","clipboard"])
                    .stdin(Stdio::piped())
                    .spawn().unwrap();
                write!(clip_command.stdin.unwrap(), "{}", url).unwrap();
            },
            _ => {
                let clip_command = Command::new("xclip")
                    .args(&["-selection","clipboard"])
                    .stdin(Stdio::piped())
                    .spawn().unwrap();
                write!(clip_command.stdin.unwrap(), "{}", url).unwrap();
            }
        }
    });
}
