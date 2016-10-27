#[macro_use] extern crate clap;
extern crate cssparser;
extern crate html5ever;
extern crate kuchiki;
extern crate selectors;
#[macro_use] extern crate string_cache;

mod html_decorator;

use html_decorator::*;
use std::borrow::Cow;
use std::ffi::OsStr;
use std::io::prelude::*;
use std::process;
use clap::*;

fn main() {
    let matches = App::new("reportyatsu")
        .version(crate_version!())
        .arg(
            Arg::with_name("stylesheet")
                .long("stylesheet")
                .value_name("CSSFILE")
                .help("カスタム CSS")
        )
        .arg(
            Arg::with_name("highlight-style")
                .long("highlight-style")
                .takes_value(true)
                .value_name("STYLE")
                .possible_values(&["pygments", "kate", "monochrome", "espresso", "zenburn", "haddock", "tango"])
                .help("Pandoc の --highlight-style")
        )
        .arg(
            Arg::with_name("input")
                .takes_value(true)
                .multiple(true)
                .required(true)
                .value_name("INPUT")
                .help("入力する Markdown ファイル")
        )
        .get_matches();

    let stylesheet = match matches.value_of_os("stylesheet") {
        Some(x) => {
            let mut s = String::new();
            let mut file = std::fs::File::open(x).expect("Failed to open the CSS file");
            file.read_to_string(&mut s).unwrap();
            Some(Cow::Owned(s))
        },
        None if matches.is_present("stylesheet") => None,
        None => Some(Cow::Borrowed(include_str!("default_style.css")))
    };

    let highlight_style = matches.value_of_os("highlight-style");

    let options = HtmlDecoratorOptions {
        apply_style_from_style_tags: true,
        stylesheet: stylesheet,
    };

    for input in matches.values_of_os("input").into_iter().flat_map(|x| x) {
        println!("処理中: {:?}", input);
        md2html(input, &options, highlight_style);
    }
}

fn md2html(input_file: &OsStr, options: &HtmlDecoratorOptions, highlight_style: Option<&OsStr>) {
    let mut pandoc_cmd = process::Command::new("pandoc");
    pandoc_cmd.arg("-s");
    if let Some(x) = highlight_style { pandoc_cmd.arg("--highlight-style").arg(x); }

    let mut pandoc_process = pandoc_cmd
        .arg(input_file)
        .stdin(process::Stdio::null())
        .stdout(process::Stdio::piped())
        .spawn()
        .expect("Failed to execute Pandoc");

    let parse_result = HtmlDecorator::from_stream(pandoc_process.stdout.as_mut().unwrap());

    if !pandoc_process.wait().expect("Failed to execute Pandoc").success() {
        panic!("Pandoc error");
    }

    let decorator = parse_result.expect("Failed to parse the HTML");
    decorator.decorate_html(options);

    let mut output_file = std::path::PathBuf::from(input_file);
    output_file.set_extension("html");

    let mut f = std::fs::File::create(output_file).expect("Failed to create the output file");

    decorator.serialize(&mut f).expect("Failed to serialize");
}
