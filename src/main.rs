extern crate cssparser;
extern crate html5ever;
extern crate kuchiki;
extern crate selectors;
#[macro_use] extern crate string_cache;

mod html_decorator;

use html_decorator::*;
use std::process;

fn main() {
    let options = HtmlDecoratorOptions {
        apply_style_from_style_tags: true,
        stylesheet: Some(std::borrow::Cow::Borrowed(include_str!("default_style.css"))),
    };

    for arg in std::env::args().skip(1) {
        println!("{}", arg);
        md2html(arg, &options);
    }
}

fn md2html(input_file: String, options: &HtmlDecoratorOptions) {
    let mut pandoc_process = process::Command::new("pandoc")
        .arg("-s")
        .arg(&input_file)
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

    let mut output_file = std::path::PathBuf::from(&input_file);
    output_file.set_extension("html");

    let mut f = std::fs::File::create(output_file).expect("Failed to create the output file");

    decorator.serialize(&mut f).expect("Failed to serialize");
}
