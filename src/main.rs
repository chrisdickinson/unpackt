#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate error_chain;

use std::collections::HashMap;
use std::io::prelude::*;

use syntect::{
    highlighting::{ ThemeSet, Theme, Style },
    parsing::SyntaxSet,
    html::highlighted_html_for_string
};
use comrak::{ markdown_to_html, ComrakOptions };
use reqwest::header::USER_AGENT;
use flate2::read::GzDecoder;
use tar::{ Archive, Entry };
use rayon::prelude::*;
use serde_json;
use reqwest;

use crate::errors::{ Result, ErrorKind };

mod errors;
mod render;

// unpack some tarballs
// - step 1: make a request
// - step 2: untar the contents
// - step 3: try syntax highlighting them
// - step 4: time it, it will tell you what to do next
//
// case 1: it's fast
// - step 5: create a repo and make travis ci build and upload it to S3
// - step 6: set up an API gateway + lambda with terraform
// - step 7: cache the results at the edge (why not)
//
// case 2: it's slow
// - step 5: ..



fn main() {
    let mut theme_bytes = std::io::Cursor::new(
        include_bytes!("./inspiredgithub.tmTheme") as &[u8]
    );

    let theme = ThemeSet::load_from_reader(&mut theme_bytes).expect("failed to read theme");
    let client = render::Client {
        client: reqwest::Client::new(),
        syntax_set: SyntaxSet::load_defaults_newlines(),
        theme: &theme
    };

    let args: Vec<String> = std::env::args().collect();
    let (package, version) = if args.len() < 3 {
        ("beefy", "1.0.0")
    } else {
        (args[1].as_str(), args[2].as_str())
    };
    let hm = render::from_registry(&client, package, version).expect("failed to fetch");
    let serialized = serde_json::to_string(&hm).expect("failed to serialize");
    println!("{}", serialized);
}
