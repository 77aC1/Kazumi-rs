// XPath 1.0 DOM 树遍历引擎
// 将 XPath 表达式解析为步骤序列，在 HTML DOM 树中遍历

use std::collections::HashMap;
use scraper::{Html, Element, Selector, Node as ScraperNode};

#[derive(Debug, Clone, serde::Serialize)]
pub struct XPathNode {
    pub text: String,
    pub attributes: HashMap<String, String>,
    pub html: String,
    pub outer_html: String,
    pub name: String,
}

#[derive(Debug, Clone)]
struct XPathStep {
    axis: Axis,
    node_test: NodeTest,
    predicates: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
enum Axis {
    Child,
    Descendant,
    Parent,
    Self_,
    DescendantOrSelf,
    FollowingSibling,
    PrecedingSibling,
}

#[derive(Debug, Clone)]
enum NodeTest {
    NameTest(String),
    Wildcard,
    TextNode,
    CommentNode,
}

#[derive(Debug, Clone)]
pub enum XPathValue {
    Nodes(Vec<XPathNode>),
    String(String),
    Bool(bool),
    Number(f64),
}

fn parse_xpath_steps(xpath: &str) -> Vec<XPathStep> {
    // 解析 XPath 为步骤列表
    // 先判断是否以 // 开头
    let xpath = xpath.trim();
    let mut steps = Vec::new();
    
    // 处理 / 和 // 分隔
    let mut remaining = xpath;
    
    // 如果以 // 开头，添加 DescendantOrSelf::Wildcard 步骤
    if remaining.starts_with("//") {
        steps.push(XPathStep {
            axis: Axis::DescendantOrSelf,
            node_test: NodeTest::Wildcard,
            predicates: Vec::new(),
        });
        remaining = &remaining[2..];
    } else if remaining.starts_with('/') {
        remaining = &remaining[1..];
    }
    
    // 剩余的按 / 或 // 分割
    while !remaining.is_empty() {
        // 跳过开头的 /
        if remaining.starts_with('/') {
            remaining = &remaining[1..];
            continue;
        }
        
        // 检查是否是 //
        if remaining.starts_with("//") {
            steps.push(XPathStep {
                axis: Axis::DescendantOrSelf,
                node_test: NodeTest::Wildcard,
                predicates: Vec::new(),
            });
            remaining = &remaining[2..];
            continue;
        }
        
        // 解析单个步骤
        let (step, consumed) = parse_single_step(remaining);
        steps.push(step);
        remaining = &remaining[consumed..];
    }
    
    steps
}

fn parse_single_step(s: &str) -> (XPathStep, usize) {
    let mut consumed = 0;
    let chars: Vec<char> = s.chars().collect();
    
    // 确定轴
    let axis = Axis::Child;
    
    // 解析节点测试
    let (node_test, nc) = parse_node_test(&s[consumed..]);
    consumed += nc;
    
    // 解析谓词
    let mut predicates = Vec::new();
    while consumed < s.len() {
        let ch = chars[consumed];
        if ch == '[' {
            let (pred, pc) = parse_predicate(&s[consumed..]);
            predicates.push(pred);
            consumed += pc;
        } else {
            break;
        }
    }
    
    (XPathStep { axis, node_test, predicates }, consumed)
}

fn parse_node_test(s: &str) -> (NodeTest, usize) {
    let s = s.trim_start();
    let mut consumed = 0;
    
    if s.starts_with("comment()") {
        return (NodeTest::CommentNode, 9);
    }
    if s.starts_with("text()") {
        return (NodeTest::TextNode, 6);
    }
    if s.starts_with('*') {
        return (NodeTest::Wildcard, 1);
    }
    
    // 解析标签名
    let mut name = String::new();
    for ch in s.chars() {
        if ch.is_alphanumeric() || ch == '_' || ch == '-' {
            name.push(ch);
            consumed += 1;
        } else {
            break;
        }
    }
    
    if name.is_empty() {
        (NodeTest::Wildcard, 0)
    } else {
        (NodeTest::NameTest(name), consumed)
    }
}

fn parse_predicate(s: &str) -> (String, usize) {
    let mut depth = 0;
    let mut pred = String::new();
    let mut consumed = 0;
    
    for (i, ch) in s.char_indices() {
        if i == 0 && ch == '[' {
            depth = 1;
            consumed = 1;
            continue;
        }
        if ch == '[' { depth += 1; }
        if ch == ']' {
            depth -= 1;
            if depth == 0 {
                consumed = i + 1;
                break;
            }
        }
        pred.push(ch);
    }
    
    (pred.trim().to_string(), consumed)
}

pub fn evaluate(html: &str, xpath: &str) -> Result<XPathValue, String> {
    if xpath.is_empty() {
        return Ok(XPathValue::Nodes(Vec::new()));
    }
    
    // 处理 | 联合运算符
    if let Some(pipe_idx) = xpath.find('|') {
        let left = evaluate(html, xpath[..pipe_idx].trim())?;
        let right = evaluate(html, xpath[pipe_idx+1..].trim())?;
        return merge_values(left, right);
    }
    
    let document = Html::parse_document(html);
    let root = document.root_element();
    let steps = parse_xpath_steps(xpath);
    
    evaluate_inner(&root, &steps, 0)
}

fn evaluate_inner(element: &Element, steps: &[XPathStep], step_idx: usize) -> Result<XPathValue, String> {
    if step_idx >= steps.len() {
        return Ok(XPathValue::Nodes(vec![node_to_xpath(element)]));
    }
    
    let step = &steps[step_idx];
    let mut results = Vec::new();
    
    match step.axis {
        Axis::Child | Axis::Descendant => {
            let children = element.children();
            for child in &children {
                if let ScraperNode::Element(el) = child.value() {
                    if matches_node_test(el, &step.node_test) {
                        if evaluate_predicates(child, &step.predicates)? {
                            let next = evaluate_inner(child, steps, step_idx + 1)?;
                            merge_into(&mut results, next);
                        }
                    }
                }
                if step.axis == Axis::Descendant {
                    let next = evaluate_inner(child, steps, step_idx)?;
                    merge_into(&mut results, next);
                }
            }
        }
        Axis::DescendantOrSelf => {
            if matches_node_test(element, &step.node_test) {
                if evaluate_predicates(element, &step.predicates)? {
                    let next = evaluate_inner(element, steps, step_idx + 1)?;
                    merge_into(&mut results, next);
                }
            }
            for child in element.children() {
                let next = evaluate_inner(&child, steps, step_idx)?;
                merge_into(&mut results, next);
            }
        }
        _ => {}
    }
    
    Ok(XPathValue::Nodes(results))
}

fn matches_node_test(el: &Element, test: &NodeTest) -> bool {
    match test {
        NodeTest::NameTest(name) => el.value().name() == name.as_str(),
        NodeTest::Wildcard => true,
        NodeTest::TextNode => false,
        NodeTest::CommentNode => false,
    }
}

fn evaluate_predicates(element: &Element, predicates: &[String]) -> Result<bool, String> {
    for pred in predicates {
        if !evaluate_single_predicate(element, pred)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn evaluate_single_predicate(element: &Element, pred: &str) -> Result<bool, String> {
    let pred = pred.trim();
    if pred.is_empty() { return Ok(true); }
    
    // 数字谓词
    if let Ok(n) = pred.parse::<usize>() {
        let idx = element.parent().map(|p| {
            p.children().filter(|c| {
                if let ScraperNode::Element(el) = c.value() {
                    el.value().name() == element.value().name()
                } else { false }
            }).position(|c| c.id() == element.id())
        }).flatten().unwrap_or(0);
        return Ok(idx + 1 == n);
    }
    
    // @attr='value'
    if let Some(val) = pred.strip_prefix('@') {
        if let Some(eq_idx) = val.find('=') {
            let attr_name = val[..eq_idx].trim();
            let attr_val = val[eq_idx+1..].trim().trim_matches('\'').trim_matches('"');
            return Ok(element.value().attr(attr_name) == Some(attr_val));
        }
        return Ok(element.value().attr(pred.trim_start_matches('@')).is_some());
    }
    
    // contains(@attr, 'value')
    if pred.starts_with("contains(") && pred.ends_with(')') {
        let inner = &pred[9..pred.len()-1];
        if let Some(comma) = inner.find(',') {
            let func_arg = inner[..comma].trim();
            let search_val = inner[comma+1..].trim().trim_matches('\'').trim_matches('"');
            if let Some(attr) = func_arg.strip_prefix('@') {
                let attr = attr.trim();
                return Ok(element.value().attr(attr)
                    .map(|v| v.contains(search_val))
                    .unwrap_or(false));
            }
        }
    }
    
    // not(condition)
    if pred.starts_with("not(") && pred.ends_with(')') {
        let inner = &pred[4..pred.len()-1];
        return Ok(!evaluate_single_predicate(element, inner)?);
    }
    
    // position() = N
    if pred.contains("position()") {
        if let Some(eq_idx) = pred.find('=') {
            let num = pred[eq_idx+1..].trim().parse::<usize>().unwrap_or(0);
            let idx = element.parent().map(|p| {
                p.children().filter(|c| {
                    if let ScraperNode::Element(el) = c.value() {
                        el.value().name() == element.value().name()
                    } else { false }
                }).position(|c| c.id() == element.id())
            }).flatten().unwrap_or(0);
            return Ok(idx + 1 == num);
        }
    }
    
    // last()
    if pred == "last()" {
        let total = element.parent().map(|p| {
            p.children().filter(|c| {
                if let ScraperNode::Element(el) = c.value() {
                    el.value().name() == element.value().name()
                } else { false }
            }).count()
        }).unwrap_or(0);
        let idx = element.parent().map(|p| {
            p.children().filter(|c| {
                if let ScraperNode::Element(el) = c.value() {
                    el.value().name() == element.value().name()
                } else { false }
            }).position(|c| c.id() == element.id())
        }).flatten().unwrap_or(0);
        return Ok(idx + 1 == total);
    }
    
    Ok(true)
}

fn node_to_xpath(el: &Element) -> XPathNode {
    let mut attrs = HashMap::new();
    for (name, value) in el.value().attrs() {
        attrs.insert(name.to_string(), value.to_string());
    }
    
    let text = el.text().collect::<Vec<_>>().join(" ").trim().to_string();
    let html = el.inner_html();
    let tag = el.value().name();
    let attrs_str: String = el.value().attrs()
        .map(|(k, v)| format!("{}=\"{}\"", k, v))
        .collect::<Vec<_>>()
        .join(" ");
    let outer_html = if attrs_str.is_empty() {
        format!("<{}>{}</{}>", tag, html, tag)
    } else {
        format!("<{} {}>{}</{}>", tag, attrs_str, html, tag)
    };
    
    XPathNode {
        text,
        attributes: attrs,
        html,
        outer_html,
        name: tag.to_string(),
    }
}

fn merge_into(target: &mut Vec<XPathNode>, value: XPathValue) {
    match value {
        XPathValue::Nodes(nodes) => target.extend(nodes),
        _ => {}
    }
}

fn merge_values(left: XPathValue, right: XPathValue) -> Result<XPathValue, String> {
    let mut nodes = match left {
        XPathValue::Nodes(n) => n,
        _ => return Ok(right),
    };
    match right {
        XPathValue::Nodes(n) => nodes.extend(n),
        _ => {}
    }
    Ok(XPathValue::Nodes(nodes))
}

pub fn evaluate_to_json(html: &str, xpath: &str) -> String {
    match evaluate(html, xpath) {
        Ok(XPathValue::Nodes(nodes)) => {
            serde_json::json!({
                "success": true,
                "type": "nodes",
                "count": nodes.len(),
                "nodes": nodes,
            }).to_string()
        }
        Ok(XPathValue::String(s)) => {
            serde_json::json!({
                "success": true,
                "type": "string",
                "value": s,
            }).to_string()
        }
        Ok(XPathValue::Bool(b)) => {
            serde_json::json!({
                "success": true,
                "type": "bool",
                "value": b,
            }).to_string()
        }
        Ok(XPathValue::Number(n)) => {
            serde_json::json!({
                "success": true,
                "type": "number",
                "value": n,
            }).to_string()
        }
        Err(e) => {
            serde_json::json!({
                "success": false,
                "type": "error",
                "error": e,
            }).to_string()
        }
    }
}

pub fn evaluate_batch_to_json(html: &str, xpaths: HashMap<String, String>) -> String {
    let mut results = serde_json::Map::new();
    for (key, xpath) in xpaths {
        match evaluate(html, &xpath) {
            Ok(XPathValue::Nodes(nodes)) => {
                let items: Vec<serde_json::Value> = nodes.iter().map(|n| {
                    serde_json::json!({
                        "text": n.text,
                        "attributes": n.attributes,
                        "html": n.html,
                        "outer_html": n.outer_html,
                        "name": n.name,
                    })
                }).collect();
                results.insert(key.clone(), serde_json::json!({"type": "nodes", "items": items}));
            }
            Ok(XPathValue::String(s)) => {
                results.insert(key.clone(), serde_json::json!({"type": "string", "value": s}));
            }
            Ok(XPathValue::Bool(b)) => {
                results.insert(key.clone(), serde_json::json!({"type": "bool", "value": b}));
            }
            Ok(XPathValue::Number(n)) => {
                results.insert(key.clone(), serde_json::json!({"type": "number", "value": n}));
            }
            Err(e) => {
                results.insert(key.clone(), serde_json::json!({"type": "error", "error": e}));
            }
        }
    }
    serde_json::json!(results).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_tag() {
        let html = "<div><p>hello</p></div>";
        let result = evaluate(html, "//p").unwrap();
        match result {
            XPathValue::Nodes(nodes) => {
                assert_eq!(nodes.len(), 1);
                assert_eq!(nodes[0].text, "hello");
            }
            _ => panic!("Expected nodes"),
        }
    }

    #[test]
    fn test_class_predicate() {
        let html = "<div><p class='a'>one</p><p class='b'>two</p></div>";
        let result = evaluate(html, "//p[@class='a']").unwrap();
        match result {
            XPathValue::Nodes(nodes) => {
                assert_eq!(nodes.len(), 1);
                assert_eq!(nodes[0].text, "one");
            }
            _ => panic!("Expected nodes"),
        }
    }

    #[test]
    fn test_contains() {
        let html = "<div><p class='playlist-item'>x</p><p class='other'>y</p></div>";
        let result = evaluate(html, "//p[contains(@class,'playlist')]").unwrap();
        match result {
            XPathValue::Nodes(nodes) => {
                assert_eq!(nodes.len(), 1);
                assert_eq!(nodes[0].text, "x");
            }
            _ => panic!("Expected nodes"),
        }
    }

    #[test]
    fn test_wildcard() {
        let html = "<div><p>a</p><span>b</span></div>";
        let result = evaluate(html, "//*").unwrap();
        match result {
            XPathValue::Nodes(nodes) => {
                assert!(nodes.len() >= 2);
            }
            _ => panic!("Expected nodes"),
        }
    }

    #[test]
    fn test_position() {
        let html = "<ul><li>a</li><li>b</li><li>c</li></ul>";
        let result = evaluate(html, "//li[2]").unwrap();
        match result {
            XPathValue::Nodes(nodes) => {
                assert_eq!(nodes.len(), 1);
                assert_eq!(nodes[0].text, "b");
            }
            _ => panic!("Expected nodes"),
        }
    }

    #[test]
    fn test_last() {
        let html = "<ul><li>a</li><li>b</li><li>c</li></ul>";
        let result = evaluate(html, "//li[last()]").unwrap();
        match result {
            XPathValue::Nodes(nodes) => {
                assert_eq!(nodes.len(), 1);
                assert_eq!(nodes[0].text, "c");
            }
            _ => panic!("Expected nodes"),
        }
    }

    #[test]
    fn test_not_predicate() {
        let html = "<div><p class='a'>x</p><p class='b'>y</p></div>";
        let result = evaluate(html, "//p[not(@class='a')]").unwrap();
        match result {
            XPathValue::Nodes(nodes) => {
                assert_eq!(nodes.len(), 1);
                assert_eq!(nodes[0].text, "y");
            }
            _ => panic!("Expected nodes"),
        }
    }

    #[test]
    fn test_union() {
        let html = "<div><p>a</p><span>b</span></div>";
        let result = evaluate(html, "//p | //span").unwrap();
        match result {
            XPathValue::Nodes(nodes) => {
                assert_eq!(nodes.len(), 2);
            }
            _ => panic!("Expected nodes"),
        }
    }

    #[test]
    fn test_text_extraction() {
        let html = "<a href='/ep/1'>第1话</a>";
        let result = evaluate(html, "//a").unwrap();
        match result {
            XPathValue::Nodes(nodes) => {
                assert_eq!(nodes.len(), 1);
                assert_eq!(nodes[0].text, "第1话");
                assert_eq!(nodes[0].attributes.get("href").unwrap(), "/ep/1");
            }
            _ => panic!("Expected nodes"),
        }
    }

    #[test]
    fn test_href_extraction() {
        let html = "<div><a href='/ep/1'>第1话</a></div>";
        let result = evaluate(html, "//a/@href").unwrap();
        match result {
            XPathValue::String(s) => {
                assert_eq!(s, "/ep/1");
            }
            _ => panic!("Expected string"),
        }
    }

    #[test]
    fn test_contains_full() {
        let html = "<div class='playlist-container'><p>content</p></div>";
        let result = evaluate(html, "//div[contains(@class,'playlist')]").unwrap();
        match result {
            XPathValue::Nodes(nodes) => {
                assert_eq!(nodes.len(), 1);
            }
            _ => panic!("Expected nodes"),
        }
    }

    #[test]
    fn test_complex_rule() {
        let html = "<html><body><div class='list'><ul><li><a href='/1'>AA</a></li><li><a href='/2'>BB</a></li></ul></div></body></html>";
        let result = evaluate(html, "//div[@class='list']//a").unwrap();
        match result {
            XPathValue::Nodes(nodes) => {
                assert_eq!(nodes.len(), 2);
                assert_eq!(nodes[0].attributes.get("href").unwrap(), "/1");
                assert_eq!(nodes[1].attributes.get("href").unwrap(), "/2");
            }
            _ => panic!("Expected nodes"),
        }
    }
}