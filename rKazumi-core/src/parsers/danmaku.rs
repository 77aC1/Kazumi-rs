use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DanmakuItem {
    pub time_ms: f64,
    pub danmaku_type: i32,
    pub font_size: i32,
    pub color: i64,
    pub timestamp: i64,
    pub uid: i64,
    pub danmaku_id: i64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DanmakuCollection {
    pub items: Vec<DanmakuItem>,
}

pub struct DanmakuParser;

impl DanmakuParser {
    pub fn parse_bilibili_xml(xml: &str) -> Result<DanmakuCollection, String> {
        let document = match roxmltree::Document::parse(xml) {
            Ok(doc) => doc,
            Err(e) => return Err(format!("弹幕 XML 解析失败: {}", e)),
        };

        let mut items = Vec::new();
        for d_node in document.descendants() {
            if d_node.has_tag_name("d") {
                if let Some(p_attr) = d_node.attribute("p") {
                    let parts: Vec<&str> = p_attr.split(',').collect();
                    if parts.len() >= 8 {
                        let time_ms = parts[0].parse::<f64>().unwrap_or(0.0) * 1000.0;
                        let danmaku_type = parts[1].parse::<i32>().unwrap_or(1);
                        let font_size = parts[2].parse::<i32>().unwrap_or(25);
                        let color = parts[3].parse::<i64>().unwrap_or(16777215);
                        let timestamp = parts[4].parse::<i64>().unwrap_or(0);
                        let uid = parts[6].parse::<i64>().unwrap_or(0);
                        let danmaku_id = parts[7].parse::<i64>().unwrap_or(0);

                        let text = d_node.text().unwrap_or("").to_string();

                        items.push(DanmakuItem {
                            time_ms,
                            danmaku_type,
                            font_size,
                            color,
                            timestamp,
                            uid,
                            danmaku_id,
                            text,
                        });
                    }
                }
            }
        }

        Ok(DanmakuCollection { items })
    }

    pub fn parse_json(json: &str) -> Result<DanmakuCollection, String> {
        #[derive(Deserialize)]
        struct DanmakuEntry {
            #[serde(rename = "0")]
            time: f64,
            #[serde(rename = "1")]
            danmaku_type: i32,
            #[serde(rename = "2")]
            font_size: i32,
            #[serde(rename = "3")]
            color: i64,
            #[serde(rename = "4")]
            timestamp: i64,
            #[serde(rename = "5")]
            uid: i64,
            #[serde(rename = "6")]
            danmaku_id: i64,
            #[serde(rename = "7")]
            text: String,
        }

        let entries: Vec<DanmakuEntry> = serde_json::from_str(json)
            .map_err(|e| format!("弹幕 JSON 解析失败: {}", e))?;

        let items = entries.into_iter().map(|e| DanmakuItem {
            time_ms: e.time * 1000.0,
            danmaku_type: e.danmaku_type,
            font_size: e.font_size,
            color: e.color,
            timestamp: e.timestamp,
            uid: e.uid,
            danmaku_id: e.danmaku_id,
            text: e.text,
        }).collect();

        Ok(DanmakuCollection { items })
    }

    pub fn to_dplayer_json(items: &[DanmakuItem]) -> String {
        let dplayer_items: Vec<serde_json::Value> = items.iter().map(|item| {
            serde_json::json!([
                item.time_ms,
                item.danmaku_type,
                item.font_size,
                item.color,
                item.timestamp,
                item.uid,
                item.danmaku_id,
                item.text,
            ])
        }).collect();

        serde_json::json!({
            "code": 0,
            "data": dplayer_items,
        }).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bilibili_xml() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?><i><d p="0.5,1,25,16777215,1234567890,0,12345,67890">测试弹幕</d></i>"#;
        let result = DanmakuParser::parse_bilibili_xml(xml).unwrap();
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].text, "测试弹幕");
        assert_eq!(result.items[0].time_ms, 500.0);
    }

    #[test]
    fn test_to_dplayer_json() {
        let items = vec![
            DanmakuItem {
                time_ms: 1000.0, danmaku_type: 1, font_size: 25, color: 16777215,
                timestamp: 0, uid: 0, danmaku_id: 0, text: "hello".to_string(),
            },
        ];
        let json = DanmakuParser::to_dplayer_json(&items);
        assert!(json.contains("hello"));
    }
}