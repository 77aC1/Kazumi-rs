use std::collections::HashMap;

use crate::rule_engine::parser::*;
use crate::rule_engine::xpath_evaluator::*;

pub fn extract_bangumi(html: &str, rule_name: &str, rules_json: &str) -> Result<String, String> {
    let rules: Vec<serde_json::Value> = serde_json::from_str(rules_json)
        .map_err(|e| format!("规则 JSON 解析失败: {}", e))?;
    
    let rule = rules.iter()
        .find(|r| r.get("name").and_then(|v| v.as_str()) == Some(rule_name))
        .ok_or_else(|| format!("规则 '{}' 未找到", rule_name))?;
    
    let search_list = rule.get("searchList").and_then(|v| v.as_str()).unwrap_or("");
    let search_name = rule.get("searchName").and_then(|v| v.as_str()).unwrap_or("");
    let search_result = rule.get("searchResult").and_then(|v| v.as_str()).unwrap_or("");
    
    let doc = Html::parse_document(html);
    let root = doc.root_element();
    
    let items_result = evaluate(html, search_list)?;
    let mut results = Vec::new();
    
    if let XPathValue::Nodes(nodes) = items_result {
        for node in &nodes {
            let name = if !search_name.is_empty() {
                evaluate(&node.outer_html, search_name).ok()
                    .and_then(|v| match v { XPathValue::String(s) => Some(s), XPathValue::Nodes(n) => n.first().map(|n| n.text.clone()), _ => None })
                    .unwrap_or_default()
            } else { String::new() };
            
            let src = if !search_result.is_empty() {
                evaluate(&node.outer_html, search_result).ok()
                    .and_then(|v| match v {
                        XPathValue::String(s) => Some(s),
                        XPathValue::Nodes(n) => n.first().and_then(|n| n.attributes.get("href")).cloned(),
                        _ => None
                    })
                    .unwrap_or_default()
            } else { String::new() };
            
            results.push(serde_json::json!({
                "name": name,
                "src": src,
                "html": node.html,
            }));
        }
    }
    
    Ok(serde_json::json!({"items": results}).to_string())
}

pub fn extract_video_sources(html: &str, rule_name: &str, rules_json: &str) -> Result<String, String> {
    let rules: Vec<serde_json::Value> = serde_json::from_str(rules_json)
        .map_err(|e| format!("规则 JSON 解析失败: {}", e))?;
    
    let rule = rules.iter()
        .find(|r| r.get("name").and_then(|v| v.as_str()) == Some(rule_name))
        .ok_or_else(|| format!("规则 '{}' 未找到", rule_name))?;
    
    let chapter_roads = rule.get("chapterRoads").and_then(|v| v.as_str()).unwrap_or("");
    let chapter_result = rule.get("chapterResult").and_then(|v| v.as_str()).unwrap_or("");
    
    let mut roads = Vec::new();
    
    if !chapter_roads.is_empty() {
        let road_nodes = evaluate(html, chapter_roads)?;
        if let XPathValue::Nodes(road_list) = road_nodes {
            for road_node in &road_list {
                if !chapter_result.is_empty() {
                    let eps = evaluate(&road_node.outer_html, chapter_result)?;
                    if let XPathValue::Nodes(ep_list) = eps {
                        let mut data = Vec::new();
                        let mut names = Vec::new();
                        for ep in &ep_list {
                            let url = ep.attributes.get("href").cloned().unwrap_or_default();
                            if !url.is_empty() {
                                data.push(url);
                                names.push(ep.text.clone());
                            }
                        }
                        if !data.is_empty() {
                            roads.push(serde_json::json!({
                                "name": "播放线路",
                                "data": data,
                                "names": names,
                            }));
                        }
                    }
                }
            }
        }
    }
    
    Ok(serde_json::json!({"roads": roads}).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_not_found() {
        let result = extract_bangumi("<div></div>", "nonexistent", "[]");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_bangumi_list() {
        let html = r#"<div class="list"><a href="/1">AA</a></div>"#;
        let rules = r#"[{"name":"test","searchList":"//div[@class='list']","searchName":".","searchResult":".//a"}]"#;
        let result = extract_bangumi(html, "test", rules);
        assert!(result.is_ok());
    }
}