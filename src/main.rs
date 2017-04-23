extern crate xdg;
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
use hyper::client::Request;
use notify_rust::Notification;
use std::collections::HashMap;
use hyper::net::HttpsConnector;
use multipart::client::Multipart;
use hyper_native_tls::NativeTlsClient;

#[derive(Serialize, Deserialize, Debug, Clone)]
enum ResponseType {
    Text,
    Redirect,
    Regex,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
enum SharustMethod {
    Post,
}

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

#[derive(Serialize, Deserialize, Debug)]
struct SharustConfig {
    image_uploader: String,
    provider: Vec<SharustProvider>,
}

fn main() {
    let yaml = load_yaml!("clap.yml");
    let matches = App::from_yaml(yaml).get_matches();
    let xdg_dirs = xdg::BaseDirectories::with_prefix("sharust").unwrap();

    let mut config_path = xdg_dirs.place_config_file("sharust.json").expect("Cannot create config directory.");
    let mut config_file = match File::open(config_path.clone()) {
        Ok(conf) => conf,
        Err(_) => {
            let mut c_file = File::create(config_path).unwrap();
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
            }).unwrap().as_bytes());
            println!("Created the config file. Please update it with valid data.");
            std::process::exit(0);
        }
    };
    let mut config_contents = String::new();
    config_file.read_to_string(&mut config_contents).unwrap();
    let config: SharustConfig = serde_json::from_str(config_contents.as_str()).unwrap();

    if let Some(matches) = matches.subcommand_matches("upload") {
        // Probably called from maim
        let input = matches.value_of("INPUT").unwrap();
        let mut provider_iter = config.provider.clone().into_iter().filter(|x| x.name == config.image_uploader);
        let provider = provider_iter.next().unwrap();

        let url: Url = provider.request_url.parse().expect("Failed to parse URL");

        let ssl = NativeTlsClient::new().unwrap();
        let connector = HttpsConnector::new(ssl);
        let request = Request::with_connector(Method::Post, url, &connector).unwrap();

        let mut multi = Multipart::from_request(request).expect("Failed to create Multipart");

        multi.write_file(provider.file_form_name, input).expect("FAILED TO WRITE FILE");
        for (arg_name, arg_val) in &provider.arguments {
            multi.write_text(arg_name, arg_val).unwrap();
        }

        let mut response = multi.send().expect("Failed to send multipart request");

        let mut response_text = String::new();
        response.read_to_string(&mut response_text).unwrap();

        std::io::stdout().write_all(response_text.as_bytes()).unwrap();
    } else {
        // How the user _should_ interface with sharust
        let mode = matches.value_of("mode").unwrap_or("full");
        let args = match mode {
            "full" => ["/tmp/mynewimage.png","","",""],
            "area" => ["/tmp/mynewimage.png","-s","-c 1,0,1,0.1","-l"],
            _ => unreachable!(),
        };

        Command::new("maim")
    .args(&args)
    .output()
    .expect("Failed to execute process");
    let out = Command::new("sharust")
            .args(&["upload","/tmp/mynewimage.png"])
            .output()
            .expect("Failed to execute process");
    let out_url = out.stdout.clone();
    let out_url = String::from_utf8(out_url).unwrap();
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
                        .arg(out_url.clone())
                        .output().
                        expect("derp");
                },
                "copy" => {
                    let clip_command = Command::new("xclip")
                        .args(&["-selection","clipboard"])
                        .stdin(Stdio::piped())
                        .spawn().unwrap();
                    write!(clip_command.stdin.unwrap(), "{}", out_url).unwrap();
                },
                _ => ()
            }
        });
    }

}