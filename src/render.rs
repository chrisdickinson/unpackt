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
use crate::render;

lazy_static! {
    static ref REGISTRY_URL: String = {
        std::env::var("REGISTRY_URL").ok().unwrap_or_else(|| "https://registry.npmjs.org".to_string())
    };
}

pub struct Client<'a> {
    pub client: reqwest::Client,
    pub syntax_set: SyntaxSet,
    pub theme: &'a Theme
}

pub fn from_registry(client: &Client, name: &str, version: &str) -> Result<HashMap<String, String>> {
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

    let tuples: Vec<(FileTarget, std::path::PathBuf, String)> = archive.entries()?.into_iter().filter_map(|entry| {
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
