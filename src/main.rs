#![allow(unused)]

use std::{env, error::Error};

use reqwest::header;
use scraper::{Html, Selector};

const USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64; rv:78.0) Gecko/20100101 Firefox/78.0";

#[derive(Clone, Debug, PartialEq, clap::ValueEnum)]
enum PartOfSpeech {
    Noun = 1,
    Adj,
    Verb,
    Other = 0,
}

#[derive(Default)]
struct RifmeOptions {
    syllables: Option<i8>,
    part: Option<PartOfSpeech>,
    emphasis: Option<i8>,
}

fn build_cookie(options: RifmeOptions) -> String {
    let mut cookie = String::new();
    if let Some(syllables) = options.syllables {
        cookie.push_str(&format!("slogovcookie={};", syllables));
    }
    if let Some(part) = options.part {
        cookie.push_str(&format!("chastcookie={};", part as i8));
    }
    return cookie;
}

async fn get_page(
    url: &str,
    options: RifmeOptions,
) -> Result<String, reqwest::Error> {
    let mut headers = header::HeaderMap::new();
    headers.insert(
        "COOKIE",
        header::HeaderValue::try_from(build_cookie(options)).unwrap(),
    );
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .default_headers(headers)
        .build()?;
    let body = client.get(url).send().await.unwrap().text();
    return body.await;
}

fn get_rhymes(doc: Html) -> Result<Vec<String>, Box<dyn Error>> {
    let selector = Selector::parse("li[class=riLi]").unwrap();
    let result = doc
        .select(&selector)
        .map(|li| li.value().attr("data-w").unwrap().to_string())
        .collect::<Vec<_>>();
    return Ok(result);
}

async fn get_rifme(
    word: String,
    options: RifmeOptions,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut url = format!("https://rifme.net/r/{}", word,);
    if let Some(emphasis) = options.emphasis {
        url.push_str(&format!("/{}", emphasis));
    }
    let body = get_page(&url, options).await.unwrap();
    let doc = Html::parse_document(&body);
    let rhymes = get_rhymes(doc).unwrap();
    return Ok(rhymes);
}

use clap::Parser;

#[derive(Parser, Debug)]
#[clap(
    author = "Nick Friday",
    version = "0.1",
    about = "A simple tool to generate russian rhymes with rifme.net service",
    long_about = None
)]
struct Args {
    /// Word to get rhymes for
    word: String,

    /// Number of syllables - any by default, 0 for FULL (may be slow)
    #[arg(short, long, default_value = None)]
    syllables: Option<i8>,

    /// Part of speech - all by default
    #[arg(short, long, default_value = None)]
    part: Option<PartOfSpeech>,

    /// Emphasis number - 0 for last, 1 for 2nd last, etc.
    #[arg(short, long, default_value = None)]
    emphasis: Option<i8>,
}

#[async_std::main]
async fn main() {
    let args = Args::parse();
    let rhymes: Vec<String> = if args.syllables.unwrap_or(-1) > 0 {
        get_rifme(
            args.word,
            RifmeOptions {
                syllables: args.syllables,
                part: args.part,
                emphasis: args.emphasis,
            },
        )
        .await
        .unwrap()
    } else {
        futures::future::join_all(
            (1..=8)
                .map(|syllables| {
                    let options = RifmeOptions {
                        syllables: Some(syllables),
                        part: args.part.to_owned(),
                        emphasis: args.emphasis,
                    };
                    get_rifme(args.word.to_owned(), options)
                })
                .collect::<Vec<_>>(),
        )
        .await
        .into_iter()
        .map(|result| result.unwrap())
        .flatten()
        .collect::<Vec<_>>()
    };
    println!("{}", rhymes.join("\n"));
}
