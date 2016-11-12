mod stylesheets;

use std::borrow::Cow;
use std::io;
use std::io::prelude::*;
use html5ever::serialize;
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
        serialize::serialize(
            writer,
            &self.root.descendants()
                .filter(|x| x.as_element().map(|y| &y.name.local) == Some(&local_name!("body")))
                .nth(0).unwrap(),
            serialize::SerializeOpts {
                scripting_enabled: false,
                traversal_scope: serialize::TraversalScope::ChildrenOnly,
            }
        )
    }

    pub fn decorate_html(&self, options: &HtmlDecoratorOptions) {
        let core = HtmlDecoratorCore {
            root: &self.root,
            options: options,
        };

        // core.line_highlight();
        // moodle に pre 内の div の style 消されて泣いた
        core.apply_stylesheets();
        core.table_caption();
        core.source_code_caption();
        core.remove_class_and_id();
    }
}

struct HtmlDecoratorCore<'a> {
    root: &'a kuchiki::NodeRef,
    options: &'a HtmlDecoratorOptions<'a>,
}

impl<'a> HtmlDecoratorCore<'a> {
    /// 指定された行数にスタイル適用（はつらかったので行数表示に）
    #[allow(dead_code)]
    fn line_highlight(&self) {
        let targets = self.root.descendants()
            .filter_map(|node_ref|
                node_ref.as_element()
                    .and_then(|elm| elm.attributes.borrow_mut().remove("data-highlight"))
                    .map(|s| (node_ref, s))
            );

        for (node_ref, s) in targets {
            let mut lines = ::std::collections::BTreeSet::new();
            for x in s.split(',').map(|x| x.trim()).filter(|x| !x.is_empty()) {
                match x.find('-') {
                    Some(i) => {
                        let s: usize = x[..i].trim().parse().expect("invalid range");
                        let e: usize = x[i + 1..].trim().parse().expect("invalid range");
                        for y in s..e+1 { lines.insert(y); }
                    }
                    None => { lines.insert(x.parse().expect("invalid range")); }
                }
            }

            for pre in node_ref.select(".lineNumbers pre").unwrap() {
                let pre = pre.as_node();
                let line_count = pre.text_contents().lines().count();

                // 元のやつは全部削除
                for child in pre.children() {
                    child.detach();
                }

                for i in 1..line_count+1 {
                    let txt = kuchiki::NodeRef::new_text(format!("{}\n", i));
                    if lines.contains(&i) {
                        let elm = kuchiki::NodeRef::new_element(
                            qualname!("", "div"),
                            Some((qualname!("", "class"), "lineHighlight".to_owned()))
                        );
                        elm.append(txt);
                        pre.append(elm);
                    } else {
                        pre.append(txt);
                    }
                }
            }
        }
    }

    /// style タグとユーザー指定スタイルシートから style 属性を作成
    fn apply_stylesheets(&self) {
        fn apply(root: &kuchiki::NodeRef, stylesheet: &str) {
            stylesheets::each_rule(stylesheet, |r| match r {
                Ok((selector, decls)) =>
                    for elm in selector.filter(kuchiki::iter::Elements(root.descendants())) {
                        let mut attrs = elm.attributes.borrow_mut();
                        if match attrs.get_mut(local_name!("style")) {
                            Some(v) => {
                                v.push_str(&decls);
                                false
                            },
                            None => true
                        } {
                            attrs.insert(local_name!("style"), decls.clone());
                        }
                    },
                Err(x) => panic!("Invalid stylesheet: {:?}", x)
            })
        }

        // style タグ
        if self.options.apply_style_from_style_tags {
            let style_tags = self.root.descendants()
                .filter(|node_ref| node_ref.as_element().map(|elm| &elm.name.local) == Some(&local_name!("style")));

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

    /// caption タグを p タグにする（デザインの統一）
    fn table_caption(&self) {
        for caption in self.root.select("table > caption").unwrap() {
            let caption = caption.as_node();
            if let Some(table) = caption.parent() {
                let p = kuchiki::NodeRef::new_element(
                    qualname!("", "p"),
                    Some((qualname!("", "style"), "text-align: center;".to_owned()))
                );
                for x in caption.children() {
                    p.append(x);
                }
                caption.detach();
                table.insert_before(p);
            }
        }
    }

    /// ソースコードの title 属性を p タグにする
    fn source_code_caption(&self) {
        for source_code_box in self.root.select("div.sourceCode, pre").unwrap() {
            let title = source_code_box.attributes.borrow_mut().remove(local_name!("title"));
            if let Some(title) = title {
                let p = kuchiki::NodeRef::new_element(
                    qualname!("", "p"),
                    Some((qualname!("", "style"), "text-align: center;".to_owned()))
                );
                p.append(kuchiki::NodeRef::new_text(title));
                source_code_box.as_node().insert_before(p);
            }
        }
    }

    /// class と id 消し去るマン
    fn remove_class_and_id(&self) {
        for elm in kuchiki::iter::Elements(self.root.descendants()) {
            let mut attrs = elm.attributes.borrow_mut();
            attrs.remove(local_name!("class"));
            attrs.remove(local_name!("id"));
        }
    }
}
