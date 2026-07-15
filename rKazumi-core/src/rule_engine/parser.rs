use std::collections::HashMap;

pub fn parse_rule(json: &str) -> Result<serde_json::Value, String> {
    let rule: serde_json::Value = serde_json::from_str(json)
        .map_err(|e| format!("规则 JSON 解析失败: {}", e))?;
    Ok(rule)
}

pub fn validate_rule(rule: &serde_json::Value) -> Result<(), String> {
    if let Some(obj) = rule.as_object() {
        if !obj.contains_key("name") {
            return Err("规则缺少 'name' 字段".to_string());
        }
        if !obj.contains_key("searchList") && !obj.contains_key("searchUrl") {
            return Err("规则缺少搜索相关字段".to_string());
        }
    }
    Ok(())
}

pub fn selector_type(rule: &serde_json::Value) -> &'static str {
    if rule.get("searchList").and_then(|v| v.as_str()).map(|s| !s.is_empty()).unwrap_or(false) {
        "xpath"
    } else if rule.get("searchUrl").and_then(|v| v.as_str()).map(|s| !s.is_empty()).unwrap_or(false) {
        "api"
    } else {
        "unknown"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rule() {
        let json = r#"{"name":"test","searchList":"//div"}"#;
        let result = parse_rule(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_rule() {
        let json = serde_json::json!({"name":"test","searchList":"//div"});
        assert!(validate_rule(&json).is_ok());
    }

    #[test]
    fn test_selector_type() {
        let json = serde_json::json!({"searchList":"//div","searchUrl":""});
        assert_eq!(selector_type(&json), "xpath");
    }
}