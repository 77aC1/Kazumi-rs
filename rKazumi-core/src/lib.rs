use std::ffi::{CStr, CString};
use std::os::raw::c_char;

mod network;
mod parsers;
mod rule_engine;
mod utils;

/// safe_cstr 辅助函数
fn safe_cstr<'a>(ptr: *const c_char) -> &'a str {
    if ptr.is_null() {
        return "";
    }
    unsafe {
        match CStr::from_ptr(ptr).to_str() {
            Ok(s) => s,
            Err(_) => "",
        }
    }
}

fn error_json(msg: &str) -> *mut c_char {
    let result = serde_json::json!({
        "success": false,
        "error": msg,
    });
    CString::new(result.to_string()).unwrap_or_default().into_raw()
}

/// 获取引擎版本
#[no_mangle]
pub extern "C" fn rk_version() -> *mut c_char {
    let version = env!("CARGO_PKG_VERSION").to_string();
    CString::new(version).unwrap_or_default().into_raw()
}

/// 释放 Rust 分配的字符串内存
#[no_mangle]
pub extern "C" fn rk_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe { let _ = CString::from_raw(s); }
    }
}

/// 解析规则 JSON
#[no_mangle]
pub extern "C" fn rk_parse_rule(rules_json: *const c_char) -> *mut c_char {
    let input = safe_cstr(rules_json);
    match rule_engine::parser::parse_rule(input) {
        Ok(rule) => {
            let result = serde_json::json!({
                "success": true,
                "rule": rule,
            });
            CString::new(result.to_string()).unwrap_or_default().into_raw()
        }
        Err(e) => error_json(&e),
    }
}

/// 执行规则提取番剧列表
#[no_mangle]
pub extern "C" fn rk_extract_bangumi(
    html: *const c_char,
    rule_name: *const c_char,
    rules_json: *const c_char,
) -> *mut c_char {
    let html = safe_cstr(html);
    let rule_name = safe_cstr(rule_name);
    let rules_json = safe_cstr(rules_json);

    match rule_engine::executor::extract_bangumi(html, rule_name, rules_json) {
        Ok(result) => {
            CString::new(result).unwrap_or_default().into_raw()
        }
        Err(e) => error_json(&e),
    }
}

/// 执行规则提取视频源
#[no_mangle]
pub extern "C" fn rk_extract_video_sources(
    html: *const c_char,
    rule_name: *const c_char,
    rules_json: *const c_char,
) -> *mut c_char {
    let html = safe_cstr(html);
    let rule_name = safe_cstr(rule_name);
    let rules_json = safe_cstr(rules_json);

    match rule_engine::executor::extract_video_sources(html, rule_name, rules_json) {
        Ok(result) => {
            CString::new(result).unwrap_or_default().into_raw()
        }
        Err(e) => error_json(&e),
    }
}

/// 解析 M3U8 播放列表
#[no_mangle]
pub extern "C" fn rk_parse_m3u8(
    content: *const c_char,
    base_url: *const c_char,
) -> *mut c_char {
    let content = safe_cstr(content);
    let base_url = safe_cstr(base_url);

    match parsers::m3u8::M3U8Parser::parse(
        &content,
        if base_url.is_empty() { None } else { Some(&base_url) },
    ) {
        Ok(playlist) => {
            let result = serde_json::json!({
                "success": true,
                "playlist_type": format!("{:?}", playlist.playlist_type),
                "segments_count": playlist.segments.len(),
                "variants_count": playlist.variants.len(),
                "segments": playlist.segments,
                "variants": playlist.variants,
            });
            CString::new(result.to_string()).unwrap_or_default().into_raw()
        }
        Err(e) => error_json(&e),
    }
}

/// 解析 Bilibili 弹幕 XML
#[no_mangle]
pub extern "C" fn rk_parse_danmaku_bilibili(xml: *const c_char) -> *mut c_char {
    let xml = safe_cstr(xml);
    match parsers::danmaku::DanmakuParser::parse_bilibili_xml(&xml) {
        Ok(collection) => {
            let result = serde_json::json!({
                "success": true,
                "items": collection.items,
            });
            CString::new(result.to_string()).unwrap_or_default().into_raw()
        }
        Err(e) => error_json(&e),
    }
}

/// 计算字符串相似度
#[no_mangle]
pub extern "C" fn rk_similarity(
    a: *const c_char,
    b: *const c_char,
) -> f64 {
    let a = safe_cstr(a);
    let b = safe_cstr(b);
    utils::similarity::combined_similarity(a, b)
}

/// 执行 XPath 查询
#[no_mangle]
pub extern "C" fn rk_extract_xpath(
    html: *const c_char,
    xpath: *const c_char,
) -> *mut c_char {
    let html = safe_cstr(html);
    let xpath = safe_cstr(xpath);
    let result = rule_engine::xpath_evaluator::evaluate_to_json(html, xpath);
    CString::new(result).unwrap_or_default().into_raw()
}

/// 批量执行 XPath 查询
#[no_mangle]
pub extern "C" fn rk_extract_xpath_batch(
    html: *const c_char,
    xpaths_json: *const c_char,
) -> *mut c_char {
    let html = safe_cstr(html);
    let xpaths_json = safe_cstr(xpaths_json);
    let xpaths: std::collections::HashMap<String, String> = match serde_json::from_str(xpaths_json) {
        Ok(m) => m,
        Err(e) => return error_json(&format!("Invalid xpaths JSON: {}", e)),
    };
    let result = rule_engine::xpath_evaluator::evaluate_batch_to_json(html, xpaths);
    CString::new(result).unwrap_or_default().into_raw()
}

/// 专用搜索提取函数
#[no_mangle]
pub extern "C" fn rk_extract_search(
    html: *const c_char,
    list_xpath: *const c_char,
    name_xpath: *const c_char,
    url_xpath: *const c_char,
) -> *mut c_char {
    let html = safe_cstr(html);
    let list_xpath = safe_cstr(list_xpath);
    let name_xpath = safe_cstr(name_xpath);
    let url_xpath = safe_cstr(url_xpath);

    match parsers::search::extract_search_results(html, list_xpath, name_xpath, url_xpath) {
        Ok(result) => {
            CString::new(result).unwrap_or_default().into_raw()
        }
        Err(e) => error_json(&e),
    }
}
