// Copyright 2018 Fredrik Portström <https://portstrom.com>
// This is free software distributed under the terms specified in
// the file LICENSE at the top-level directory of this distribution.

#![forbid(unsafe_code)]

extern crate hyper;
extern crate hyper_tls;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use hyper::rt::{Future, Stream};
use std::borrow::Cow;

#[derive(Debug, Deserialize)]
struct General<'a> {
    #[serde(rename = "linktrail")]
    link_trail: Cow<'a, str>,
}

#[derive(Debug, Deserialize)]
struct MagicWord<'a> {
    aliases: Vec<Cow<'a, str>>,
    name: Cow<'a, str>,
}

#[derive(Debug, Deserialize)]
struct Namespace<'a> {
    #[serde(rename = "*")]
    alias: Cow<'a, str>,
    canonical: Option<Cow<'a, str>>,
    id: i32,
}

#[derive(Debug, Deserialize)]
struct NamespaceAlias<'a> {
    id: i32,
    #[serde(rename = "*")]
    alias: Cow<'a, str>,
}

#[derive(Debug, Deserialize)]
struct Query<'a> {
    #[serde(rename = "extensiontags")]
    extension_tags: Vec<Cow<'a, str>>,
    general: General<'a>,
    #[serde(rename = "magicwords")]
    magic_words: Vec<MagicWord<'a>>,
    #[serde(rename = "namespacealiases")]
    namespace_aliases: Vec<NamespaceAlias<'a>>,
    namespaces: std::collections::HashMap<Cow<'a, str>, Namespace<'a>>,
    protocols: Vec<Cow<'a, str>>,
}

#[derive(Deserialize)]
struct Response<'a> {
    query: Query<'a>,
}

fn main() {
    let mut arguments = std::env::args();
    if arguments.len() != 2 {
        eprintln!("Invalid use.");
        std::process::exit(1);
    }
    let host_name = arguments.nth(1).unwrap();
    let url = match format!("https://{}/w/api.php?action=query&format=json&meta=siteinfo&siprop=extensiontags%7Cgeneral%7Cmagicwords%7Cnamespaces%7Cnamespacealiases%7Cprotocols", host_name).parse() {
        Err(error) => {
            eprintln!("Invalid URL: {}", error);
            std::process::exit(1);
        }
        Ok(url) => url
    };
    let client = ::hyper::Client::builder()
        .build::<_, ::hyper::Body>(::hyper_tls::HttpsConnector::new(4).unwrap());
    hyper::rt::run(client.get(url).then(|result| match result {
        Err(error) => {
            eprintln!("Request failed: {}", error);
            std::process::exit(1);
        }
        Ok(response) => {
            if response.status() != hyper::StatusCode::OK {
                eprintln!(
                    "The status of the response is not as expected. Response: {:#?}",
                    response
                );
                std::process::exit(1);
            }
            if match response.headers().get(::hyper::header::CONTENT_TYPE) {
                None => false,
                Some(value) => value.as_bytes() != b"application/json; charset=utf-8",
            } {
                eprintln!("The value of the 'Content-Type' header of the response is not as expected. Response: {:#?}", response);
                ::std::process::exit(1);
            }
            response
                .into_body()
                .concat2()
                .then(|result| -> Result<(), ()> {
                    match result {
                        Err(error) => {
                            eprintln!("Request failed: {}", error);
                            std::process::exit(1);
                        }
                        Ok(response_body) => {
                            match serde_json::from_slice::<Response>(&response_body) {
                                Err(error) => {
                                    eprintln!("Failed to parse response: {}", error);
                                    std::process::exit(1);
                                }
                                Ok(response_data) => {
                                    create_configuration(response_data.query);
                                    std::process::exit(0);
                                }
                            }
                        }
                    }
                })
        }
    }));
}

macro_rules! check {
    ($message:tt $value:expr) => {
        if !$value {
            fail!($message);
        }
    };
}

macro_rules! fail {
    ($message:tt) => {{
        eprintln!($message);
        std::process::exit(1);
    }};
}

fn create_configuration(query: Query) {
    let mut extension_tags = vec![];
    for tag in query.extension_tags {
        check!("Extension tag not recognized." tag.starts_with('<') && tag.ends_with('>') && tag.as_bytes()[1..tag.len() - 1].iter().all(u8::is_ascii_lowercase));
        check!("Duplicate extension_tag." !extension_tags.iter().any(|item| item == &tag));
        extension_tags.push(tag[1..tag.len() - 1].to_string());
    }
    let link_trail = match if !query.general.link_trail.starts_with("/^([") {
        None
    } else if query.general.link_trail.ends_with("]+)(.*)$/sD") {
        Some(&query.general.link_trail[4..query.general.link_trail.len() - 11])
    } else if query.general.link_trail.ends_with("]+)(.*)$/sDu") {
        Some(&query.general.link_trail[4..query.general.link_trail.len() - 12])
    } else {
        None
    }.and_then(|link_trail| {
        let link_trail = link_trail.replace(
            "a-z",
            "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz",
        );
        if link_trail.as_bytes().contains(&b'-') {
            None
        } else {
            Some(link_trail)
        }
    }) {
        None => fail!("Link trail not recognized"),
        Some(link_trail) => link_trail,
    };
    let mut magic_words = vec![];
    let mut redirect_magic_words = None;
    for magic_word in &query.magic_words {
        if magic_word.name == "redirect" {
            check!("Duplicate magic word" redirect_magic_words.is_none());
            let mut aliases = vec![];
            for alias in &magic_word.aliases {
                check!("Redirect magic word alias not recognized." alias.starts_with('#'));
                let alias = &alias[1..];
                check!("Duplicate redirect magic word alias." !aliases.contains(&alias));
                aliases.push(alias);
            }
            redirect_magic_words = Some(aliases);
        } else {
            for alias in &magic_word.aliases {
                if alias.starts_with("__") && alias.ends_with("__") {
                    let alias = &alias[2..alias.len() - 2];
                    check!("Magic word alias not recognized." !alias.is_empty());
                    check!("Duplicate magic word." !magic_words.iter().any(|item| item == alias));
                    magic_words.push(alias.to_string());
                }
            }
        }
    }
    let mut redirect_magic_words = match redirect_magic_words {
        None => fail!("Redirect magic word missing."),
        Some(redirect_magic_words) => redirect_magic_words,
    };
    let mut category_namespaces = vec![];
    let mut file_namespaces = vec![];
    for item in query.namespace_aliases {
        match item.id {
            6 => {
                let alias = item.alias.to_lowercase();
                check!("Duplicate namespace alias." !file_namespaces.contains(&alias));
                file_namespaces.push(alias);
            }
            14 => {
                let alias = item.alias.to_lowercase();
                check!("Duplicate namespace alias." !category_namespaces.contains(&alias));
                category_namespaces.push(alias);
            }
            _ => {}
        }
    }
    add_namespace(&mut file_namespaces, query.namespaces.get("6"), 6);
    add_namespace(&mut category_namespaces, query.namespaces.get("14"), 14);
    let mut protocols = query.protocols;
    protocols.sort();
    for protocols in protocols.windows(2) {
        if protocols[0] == protocols[1] {
            fail!("Duplicate protocol.");
        }
    }
    category_namespaces.sort();
    extension_tags.sort();
    file_namespaces.sort();
    let mut link_trail: Vec<char> = link_trail.chars().collect();
    link_trail.sort();
    let link_trail: String = link_trail.iter().collect();
    magic_words.sort();
    redirect_magic_words.sort();
    println!(concat!(
        "pub fn create_configuration() -> ::parse_wiki_text::Configuration {{\n",
        "    ::parse_wiki_text::create_configuration(&::parse_wiki_text::ConfigurationSource {{\n",
        "        category_namespaces: &["
    ));
    print_items(&category_namespaces);
    println!("        ],\n        extension_tags: &[");
    print_items(&extension_tags);
    println!("        ],\n        file_namespaces: &[");
    print_items(&file_namespaces);
    println!(
        "        ],\n        link_trail: {:?},\n        magic_words: &[",
        link_trail
    );
    print_items(&magic_words);
    println!("        ],\n        protocols: &[");
    print_items(&protocols);
    println!("        ],\n        redirect_magic_words: &[");
    print_items(&redirect_magic_words);
    println!("        ]\n    }})\n}}");
}

fn add_namespace(namespaces: &mut Vec<String>, namespace: Option<&Namespace>, id: i32) {
    match namespace {
        None => fail!("Namespace missing."),
        Some(namespace) => {
            check!("Namespace ID does not match." namespace.id == id);
            let alias = namespace.alias.to_lowercase();
            check!("Duplicate namespace alias." !namespaces.contains(&alias));
            let canonical = match &namespace.canonical {
                None => fail!("Namespace canonical name missing."),
                Some(canonical) => canonical.to_lowercase(),
            };
            if canonical != alias {
                check!("Duplicate namespace alias." !namespaces.contains(&canonical));
                namespaces.push(canonical);
            }
            namespaces.push(alias);
        }
    }
}

fn print_items(items: &[impl std::fmt::Debug]) {
    for item in items {
        println!("            {:?},", item);
    }
}
