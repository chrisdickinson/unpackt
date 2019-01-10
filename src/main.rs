// #[macro_use]
// extern crate serde_derive;
// #[macro_use]
// extern crate lambda_runtime;

// use lambda_runtime::error::HandlerError;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate error_chain;

mod errors;

use std::io::prelude::*;
use std::collections::HashMap;
use crate::errors::{ Result, ErrorKind };

use syntect::{
    highlighting::{ ThemeSet, Theme, Style },
    parsing::SyntaxSet,
    html::highlighted_html_for_string
};
use comrak::{ markdown_to_html, ComrakOptions };
use reqwest::header::USER_AGENT;
use flate2::read::GzDecoder;
use tar::{ Archive, Entry };
use reqwest;
use rayon::prelude::*;
use serde_json;

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


lazy_static! {
    static ref REGISTRY_URL: String = {
        std::env::var("REGISTRY_URL").ok().unwrap_or_else(|| "https://registry.npmjs.org".to_string())
    };
}

struct Client<'a> {
    client: reqwest::Client,
    syntax_set: SyntaxSet,
    theme: &'a Theme
}

fn get_from_registry(client: &Client, name: &str, version: &str) -> Result<HashMap<String, String>> {
    let mut name_split = name.split("/");
    let first_part = name_split.next().unwrap();
    let next_part = name_split.next().unwrap_or_else(|| first_part);

    let url = format!("{}/{}/{}-{}.tgz", *REGISTRY_URL, first_part, next_part, version);

    let mut response = client.client.get(url.as_str())
        .header(USER_AGENT, "unpackt/1.0")
        .send()?;

    if response.status().as_u16() > 399 {
        return Err(ErrorKind::Request.into());
    }

    let mut buffer = vec![];
    response.read_to_end(&mut buffer)?;

    let mut archive = Archive::new(GzDecoder::new(buffer.as_slice()));

    enum FileTarget {
        Markdown,
        Highlight(String)
    };

    let tuples: Vec<(FileTarget, std::path::PathBuf, String)> = archive.entries()?.into_iter().filter_map(|mut entry| {
        let mut entry = entry.ok()?;
        let path = entry.path().ok()?.into_owned();
        let ext = path.extension()?.to_str()?;

        let mut buffer = String::new();
        entry.read_to_string(&mut buffer).ok()?;

        match ext {
            "md" | "markdown" | "MD" => Some((FileTarget::Markdown, path, buffer)),
            xs => Some((FileTarget::Highlight(xs.to_string()), path, buffer))
        }
    }).collect();

    let results: HashMap<_, _> = tuples.par_iter()
        .filter_map(|(target, path, input)| {
            match target {
                FileTarget::Markdown => {
                    let output = markdown_to_html(input.as_str(), &ComrakOptions::default());
                    Some((path.to_string_lossy().into_owned(), output))
                },

                FileTarget::Highlight(ext) => {
                    let syntax = client.syntax_set.find_syntax_by_extension(ext)?;
                    let output = highlighted_html_for_string(
                        input.as_str(),
                        &client.syntax_set,
                        &syntax,
                        &client.theme
                    );
                    Some((path.to_string_lossy().into_owned(), output))
                }
            }
        }).collect();

    Ok(results)
}

fn main() {
    let mut theme_bytes = std::io::Cursor::new(
        include_bytes!("./inspiredgithub.tmTheme") as &[u8]
    );

    let theme = ThemeSet::load_from_reader(&mut theme_bytes).expect("failed to read theme");
    let client = Client {
        client: reqwest::Client::new(),
        syntax_set: SyntaxSet::load_defaults_newlines(),
        theme: &theme
    };

    client.syntax_set.find_syntax_by_extension("js");
    let args: Vec<String> = std::env::args().collect();
    let (package, version) = if args.len() < 3 {
        ("beefy", "1.0.0")
    } else {
        (args[1].as_str(), args[2].as_str())
    };
    let hm = get_from_registry(&client, package, version).expect("failed to fetch");
    let serialized = serde_json::to_string(&hm).expect("failed to serialize");
    println!("{}", serialized);
}
