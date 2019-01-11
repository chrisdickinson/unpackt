#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate lambda_runtime;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate error_chain;

use lambda_runtime::error::HandlerError;
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

mod errors;
mod render;

#[derive(Deserialize, Clone)]
struct Input {
    field: u32
}

#[derive(Serialize, Clone)]
struct Output {
    field: u32
}

fn main() {
    lambda!(handler);
}

fn handler(ev: Input, ctx: lambda_runtime::Context) -> Result<Output, HandlerError> {
    Ok(Output { field: 42 })
}

