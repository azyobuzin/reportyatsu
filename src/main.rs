extern crate html5ever;
#[macro_use] extern crate string_cache;

use html_decorator::*;
use std::process;

fn main() {
    for arg in std::env::args().skip(1) {
        println!("{}", arg);
        md2html(arg);
    }
}

fn md2html(input_file: String) {
    let mut pandoc_process = process::Command::new("pandoc")
        .arg(&input_file)
        .stdin(process::Stdio::null())
        .stdout(process::Stdio::piped())
        .spawn()
        .expect("Failed to execute Pandoc");

    let parse_result = parse_html(pandoc_process.stdout.as_mut().unwrap());

    if !pandoc_process.wait().expect("Failed to execute Pandoc").success() {
        panic!("Pandoc error");
    }

    let mut dom = parse_result.expect("Failed to parse the HTML");
    decorate_html(&mut dom);

    let mut output_file = std::path::PathBuf::from(&input_file);
    output_file.set_extension("html");

    let mut f = std::fs::File::create(output_file).expect("Failed to create the output file");

    serialize_html(&mut f, &dom).expect("Failed to serialize");
}

mod html_decorator {
    use std::collections::HashMap;
    use std::io::{self, Read, Write};
    use html5ever::{self, Attribute, tendril};
    use html5ever::rcdom::*;
    use html5ever::tree_builder::interface::NodeOrText;
    use html5ever::tree_builder::TreeSink;
    use string_cache::{Atom, QualName};

    pub fn parse_html<R: Read>(r: &mut R) -> Result<RcDom, io::Error> {
        use html5ever::tendril::TendrilSink;

        html5ever::parse_document(RcDom::default(), Default::default())
            .from_utf8()
            .read_from(r)
    }

    pub fn serialize_html<Wr: Write>(writer: &mut Wr, dom: &RcDom) -> io::Result<()>
    {
        use html5ever::serialize::*;

        fn find_body(handle: &Handle) -> Option<Handle> {
            let node = handle.borrow();
            match node.node {
                Element(QualName { local: atom!("body"), .. }, _, _) => Some(handle.clone()),
                _ => node.children.iter().filter_map(find_body).nth(0)
            }
        }

        let node = &dom.document;
        serialize(writer, find_body(node).as_ref().unwrap_or(node), SerializeOpts { scripting_enabled: true, traversal_scope: TraversalScope::ChildrenOnly})
    }

    pub fn decorate_html(dom: &mut RcDom) {
        let doc = dom.get_document();
        apply_style(&doc);
        table_caption(dom, doc.clone());
        source_code_caption(dom, doc.clone());
        decorate_code_with_linenums(&doc);
        remove_class_and_id(&doc);
    }

    // StrTendril が NonAtomic でキレそう
    thread_local! {
        static TAG_STYLE: HashMap<Atom, tendril::StrTendril> = {
            let mut map = HashMap::new();
            {
                let mut add = |tag_name, style| assert_eq!(map.insert(tag_name, tendril::Tendril::from_slice(style)), None);
                add(atom!("h1"), "border-color: #478384; border-style: solid; border-width: 0 0 2px 8px; padding-left: 3px");
                add(atom!("h2"), "border-color: #478384; border-style: solid; border-width: 0 0 0 5px; padding-left: 3px");
                add(atom!("img"), "max-width: 100%");
                add(atom!("code"), "white-space: pre");
                add(atom!("pre"), "background-color: #f8f8f8; padding: 8px; overflow-x: auto");
                add(atom!("table"), "border: 1pt solid #ddd; border-spacing: 0; border-collapse: collapse; margin-left: auto; margin-right: auto");
                add(atom!("th"), "border: 1pt solid #ddd; padding: 8px; border-bottom-width: 2px");
                add(atom!("td"), "border: 1pt solid #ddd; padding: 8px");
                add(atom!("blockquote"), "border-left: 3px solid #bbb; padding-left: 0.5em");
            }
            map
        };

        static CLASS_STYLE: HashMap<&'static str, tendril::StrTendril> = {
            let mut map = HashMap::new();
            {
                let mut add = |class, style| assert_eq!(map.insert(class, tendril::Tendril::from_slice(style)), None);
                add("figure", "text-align: center; margin-top: 1em");
                // --highlight-style=tango
                add("kw", "color: #204a87");
                add("dt", "color: #204a87");
                add("dv", "color: #0000cf");
                add("bn", "color: #0000cf");
                add("fl", "color: #0000cf");
                add("ch", "color: #4e9a06");
                add("st", "color: #4e9a06");
                add("co", "color: #8f5902");
                add("ot", "color: #8f5902");
                add("al", "color: #ef2929");
                add("fu", "color: #000000");
                add("er", "color: #a40000; font-weight: bold");
                add("wa", "color: #8f5902; font-weight: bold; font-style: italic");
                add("cn", "color: #000000");
                add("sc", "color: #000000");
                add("vs", "color: #4e9a06");
                add("ss", "color: #4e9a06");
                add("va", "color: #000000");
                add("cf", "color: #204a87; font-weight: bold");
                add("op", "color: #ce5c00; font-weight: bold");
                add("pp", "color: #8f5902; font-style: italic");
                add("at", "color: #c4a000");
                add("do", "color: #8f5902; font-weight: bold; font-style: italic");
                add("an", "color: #8f5902; font-weight: bold; font-style: italic");
                add("cv", "color: #8f5902; font-weight: bold; font-style: italic");
                add("in", "color: #8f5902; font-weight: bold; font-style: italic");
            }
            map
        };

        static TABLE_SOURCECODE_STYLE: tendril::StrTendril = "width: 100%; line-height: 100%; background-color: #f8f8f8; margin: 0; padding: 0; vertical-align: baseline; border: none".into();
        static TR_TD_SOURCECODE_STYLE: tendril::StrTendril = "margin: 0; padding: 0; vertical-align: baseline; border: none".into();
        static LINE_NUMBERS_STYLE: tendril::StrTendril = "margin: 0; padding: 0 4px; vertical-align: baseline; border: none; text-align: right; color: #aaaaaa; border-right: 1px solid #aaaaaa; width: 2em".into();
    }

    fn update_style_attr(attrs: &mut Vec<Attribute>, value: tendril::StrTendril) {
        let style_name = qualname!("", "style");

        if let Some(attr_ptr) = attrs.iter_mut().find(|x| x.name == style_name) {
            let v = &mut attr_ptr.value;
            v.push_char(';');
            v.push_tendril(&value);
            return;
        }

        attrs.push(Attribute { name: style_name, value: value });
    }

    fn replace_style_attr(attrs: &mut Vec<Attribute>, value: tendril::StrTendril) {
        let style_name = qualname!("", "style");

        if let Some(attr_ptr) = attrs.iter_mut().find(|x| x.name == style_name) {
            attr_ptr.value = value;
            return;
        }

        attrs.push(Attribute { name: style_name, value: value });
    }

    macro_rules! get_classes {
        ($attrs:expr) => (
            $attrs.iter()
                .filter(|x| x.name == qualname!("", "class"))
                .flat_map(|x| x.value.split_whitespace())
        )
    }

    /// タグ名またはclassをトリガーにスタイルを適用
    fn apply_style(handle: &Handle) {
        if let Element(QualName { local: ref tag_name, .. }, _, ref mut attrs) = handle.borrow_mut().node {
            // タグ名
            if let Some(style) = TAG_STYLE.with(|m| m.get(tag_name).map(|x| x.clone())) {
                update_style_attr(attrs, style);
            }

            // class
            let class_styles: Vec<_> = get_classes!(attrs)
                .filter_map(|x| CLASS_STYLE.with(|m| m.get(x).map(|y| y.clone())))
                .collect();
            for style in class_styles {
                update_style_attr(attrs, style);
            }
        }

        for child in handle.borrow().children.iter() {
            apply_style(child);
        }
    }

    /// caption タグを p タグにする（デザインの統一）
    fn table_caption(dom: &mut RcDom, handle: Handle) {
        struct Target { table: Handle, caption: Handle, p: Handle }

        fn core(dom: &mut RcDom, handle: Handle, targets: &mut Vec<Target>) {
            if let Element(QualName { local: atom!("table"), .. }, _, _) = handle.borrow().node {
                targets.extend(
                    handle.borrow().children.iter().filter_map(|child|
                        match child.borrow().node {
                            Element(QualName { ref ns, local: atom!("caption") }, _, _)  =>
                                Some(Target{
                                    table: handle.clone(),
                                    caption: child.clone(),
                                    p: dom.create_element(
                                        QualName { ns: ns.clone(), local: atom!("p") },
                                        vec![Attribute { name: qualname!("", "style"), value: "text-align: center".into() }]
                                    )
                                }),
                            _ => None
                        }
                    )
                );
            } else {
                // テーブルはネストしないと信じてる
                for child in handle.borrow().children.iter() {
                    core(dom, child.clone(), targets);
                }
            }
        }

        let mut targets = Vec::new();
        core(dom, handle, &mut targets);

        for Target { table, caption, p } in targets {
            dom.remove_from_parent(caption.clone());
            dom.reparent_children(caption, p.clone());
            assert!(dom.append_before_sibling(table, NodeOrText::AppendNode(p)).is_ok());
        }
    }

    /// ソースコードの title 属性を p タグにする
    fn source_code_caption(dom: &mut RcDom, handle: Handle) {
        let t = match *handle.borrow_mut() {
            Node { node: Element(QualName { ref ns, local: atom!("div") }, _, ref mut attrs), ref children, .. } => {
                if get_classes!(attrs).any(|x| x == "sourceCode") {
                    match attrs.iter().position(|x| x.name == qualname!("", "title")) {
                        Some(pos) => {
                            let title_attr = attrs.drain(pos..pos+1).next().unwrap();

                            let p_node = dom.create_element(
                                QualName { ns: ns.clone(), local: atom!("p") },
                                vec![Attribute { name: qualname!("", "style"), value: "text-align: center".into() }]
                            );

                            dom.append(p_node.clone(), NodeOrText::AppendText(title_attr.value));

                            Some((children[0].clone(), p_node))
                        },
                        None => None
                    }
                } else { None }
            },
            _ => None
        };

        if let Some((sibling, p_node)) = t {
            assert!(dom.append_before_sibling(sibling, NodeOrText::AppendNode(p_node)).is_ok());
        } else {
            for child in handle.borrow().children.iter() {
                source_code_caption(dom, child.clone());
            }
        }
    }

    /// 行数つきソースコード
    fn decorate_code_with_linenums(handle: &Handle) {
        if let Element(QualName { local: ref tag_name, .. }, _, ref mut attrs) = handle.borrow_mut().node {
            let new_style = get_classes!(attrs)
                .filter_map(|class|
                    match class {
                        "sourceCode" => match tag_name {
                            &atom!("table") => Some(TABLE_SOURCECODE_STYLE.with(|x| x.clone())),
                            &atom!("td") | &atom!("tr") => Some(TR_TD_SOURCECODE_STYLE.with(|x| x.clone())),
                            _ => None
                        },
                        "lineNumbers" => Some(LINE_NUMBERS_STYLE.with(|x| x.clone())),
                        _ => None
                    }
                )
                .nth(0);

            if let Some(style) = new_style {
                replace_style_attr(attrs, style);
            }
        }

        for child in handle.borrow().children.iter() {
            decorate_code_with_linenums(child);
        }
    }

    /// class と id 消し去るマン
    fn remove_class_and_id(handle: &Handle) {
        if let Element(_, _, ref mut attrs) = handle.borrow_mut().node {
            attrs.retain(|x| match x.name.local {
                atom!("class") | atom!("id") => false,
                _ => true
            })
        }

        for child in handle.borrow().children.iter() {
            remove_class_and_id(child);
        }
    }
}
