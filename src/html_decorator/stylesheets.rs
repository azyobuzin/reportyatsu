use cssparser;
use cssparser::ToCss;
use cssparser::Token::*;
use kuchiki;

pub type SelectorDeclPair = (kuchiki::Selectors, String);

pub fn each_rule<'a, F>(s: &'a str, mut callback: F)
    where F: FnMut(Result<SelectorDeclPair, ::std::ops::Range<cssparser::SourcePosition>>)
{
    let mut parser = cssparser::Parser::new(s);
    for x in cssparser::RuleListParser::new_for_stylesheet(&mut parser, SelectorDeclPairParser) {
        callback(x);
    }
}

fn to_string_without_comments(input: &mut cssparser::Parser) -> Result<String, ()> {
    let mut s = String::new();
    let mut insert_whitespace = false;

    while let Ok(t) = input.next_including_whitespace_and_comments() {
        match t {
            WhiteSpace(_) | Comment(_) => insert_whitespace = true,
            t => {
                if insert_whitespace {
                    // スペース・コメントは半角スペース1つ扱い
                    if s.len() > 0 { s.push(' ') }
                    insert_whitespace = false;
                }
                try!(t.to_css(&mut s).map_err(|_| ()));
            }
        }
    }

    Ok(s)
}

struct SelectorDeclPairParser;

impl cssparser::QualifiedRuleParser for SelectorDeclPairParser {
    type Prelude = kuchiki::Selectors;
    type QualifiedRule = SelectorDeclPair;

    fn parse_prelude(&mut self, input: &mut cssparser::Parser) -> Result<Self::Prelude, ()> {
        let position = input.position();
        try!(input.parse_until_before(cssparser::Delimiter::CurlyBracketBlock,
            |input| Ok(while input.next_including_whitespace_and_comments().is_ok() { })));
        kuchiki::Selectors::compile(input.slice_from(position))
    }

    fn parse_block(&mut self, prelude: Self::Prelude, input: &mut cssparser::Parser) -> Result<Self::QualifiedRule, ()> {
        to_string_without_comments(input).map(|x| (prelude, x))
    }
}

impl cssparser::AtRuleParser for SelectorDeclPairParser {
    type Prelude = ();
    type AtRule = SelectorDeclPair;
    // @ には対応する気ゼロなので実装しない
}
