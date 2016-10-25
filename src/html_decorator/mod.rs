mod stylesheets;

use std::borrow::Cow;
use std::io;
use std::io::prelude::*;
use html5ever::tendril;
use kuchiki;
use kuchiki::traits::*;

pub struct HtmlDecoratorOptions<'a> {
    pub apply_style_from_style_tags: bool,
    pub stylesheet: Option<Cow<'a, str>>,
}

pub struct HtmlDecorator {
    root: kuchiki::NodeRef,
}

impl HtmlDecorator {
    pub fn from_tendril<T>(t: T) -> HtmlDecorator
        where T: Into<tendril::Tendril<tendril::fmt::UTF8>>
    {
        HtmlDecorator {
            root: kuchiki::parse_html().one(t),
        }
    }

    pub fn from_stream<R: Read>(reader: &mut R) -> io::Result<HtmlDecorator> {
        let mut s = String::new();
        try!(reader.read_to_string(&mut s));
        Ok(Self::from_tendril(s))
    }

    pub fn serialize<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        self.root.descendants()
            .filter(|x| x.as_element().map(|y| &y.name.local) == Some(&atom!("body")))
            .nth(0).unwrap()
            .serialize(writer)
    }

    pub fn decorate_html(&self, options: &HtmlDecoratorOptions) {
        let core = HtmlDecoratorCore {
            root: &self.root,
            options: options,
        };

        core.apply_style();
    }
}

struct HtmlDecoratorCore<'a> {
    root: &'a kuchiki::NodeRef,
    options: &'a HtmlDecoratorOptions<'a>,
}

impl<'a> HtmlDecoratorCore<'a> {
    /// タグ名またはclassをトリガーにスタイルを適用
    fn apply_style(&self) {
        fn apply(root: &kuchiki::NodeRef, stylesheet: &str) {
            stylesheets::each_rule(stylesheet, |r| match r {
                Ok((selector, decls)) =>
                    for elm in selector.filter(kuchiki::iter::Elements(root.descendants())) {
                        let mut attrs = elm.attributes.borrow_mut();
                        if match attrs.get_mut(atom!("style")) {
                            Some(v) => {
                                v.push_str(&decls);
                                false
                            },
                            None => true
                        } {
                            attrs.insert(atom!("style"), decls.clone());
                        }
                    },
                Err(x) => panic!("Invalid stylesheet: {:?}", x)
            })
        }

        // style タグ
        if self.options.apply_style_from_style_tags {
            let style_tags = self.root.descendants()
                .filter(|node_ref| node_ref.as_element().map(|elm| &elm.name.local) == Some(&atom!("style")));

            for node_ref in style_tags {
                let mut children = node_ref.children();
                let child = children.next();

                // style タグ内に 2 要素以上あったらキレる
                if children.next().is_some() { panic!("Invalid style tag: {:?}", node_ref); }

                if let Some(node) = child {
                    match *node.data() {
                        kuchiki::NodeData::Text(ref s) | kuchiki::NodeData::Comment(ref s)
                            => apply(self.root, &s.borrow()),
                        _ => panic!("Invalid style tag: {:?}", node)
                    }
                }
            }
        }

        // ユーザー指定スタイル
        if let Some(ref s) = self.options.stylesheet {
            apply(self.root, &s)
        }
    }
}
