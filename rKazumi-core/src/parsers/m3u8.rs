use std::str::FromStr;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct M3U8Segment {
    pub uri: String,
    pub duration: f64,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<M3U8Key>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub map: Option<M3U8Map>,
    pub discontinuity: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct M3U8Key {
    pub method: String,
    pub uri: String,
    pub iv: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct M3U8Map {
    pub uri: String,
    pub byterange: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct M3U8Variant {
    pub uri: String,
    pub bandwidth: u64,
    pub resolution: String,
    pub codecs: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PlaylistType {
    MasterPlaylist,
    MediaPlaylist,
    Unknown,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct M3U8Playlist {
    pub playlist_type: PlaylistType,
    pub segments: Vec<M3U8Segment>,
    pub variants: Vec<M3U8Variant>,
    pub target_duration: f64,
    pub media_sequence: u64,
    pub version: u32,
    pub end_list: bool,
}

pub struct M3U8Parser;

impl M3U8Parser {
    pub fn parse(content: &str, base_url: Option<&str>) -> Result<M3U8Playlist, String> {
        let lines: Vec<&str> = content.lines().map(|l| l.trim()).collect();
        
        if lines.is_empty() || lines[0] != "#EXTM3U" {
            return Err("不是有效的 M3U8 文件".to_string());
        }
        
        let mut segments = Vec::new();
        let mut variants = Vec::new();
        let mut target_duration = 0.0;
        let mut media_sequence = 0;
        let mut version = 1;
        let mut end_list = false;
        let mut current_duration = 0.0;
        let mut current_title = String::new();
        let mut current_key: Option<M3U8Key> = None;
        let mut current_map: Option<M3U8Map> = None;
        let mut current_discontinuity = false;
        let mut has_variants = false;
        
        for line in &lines[1..] {
            if line.starts_with("#EXT-X-VERSION:") {
                version = line[15..].parse().unwrap_or(1);
            } else if line.starts_with("#EXT-X-TARGETDURATION:") {
                target_duration = line[21..].parse().unwrap_or(0.0);
            } else if line.starts_with("#EXT-X-MEDIA-SEQUENCE:") {
                media_sequence = line[21..].parse().unwrap_or(0);
            } else if line.starts_with("#EXT-X-ENDLIST") {
                end_list = true;
            } else if line.starts_with("#EXTINF:") {
                let rest = &line[8..];
                if let Some(comma) = rest.find(',') {
                    current_duration = rest[..comma].parse().unwrap_or(0.0);
                    current_title = rest[comma+1..].to_string();
                } else {
                    current_duration = rest.parse().unwrap_or(0.0);
                    current_title = String::new();
                }
            } else if line.starts_with("#EXT-X-KEY:") {
                let params = &line[11..];
                current_key = Some(Self::parse_key_params(params));
            } else if line.starts_with("#EXT-X-MAP:") {
                let params = &line[11..];
                current_map = Some(Self::parse_map_params(params));
            } else if line.starts_with("#EXT-X-DISCONTINUITY") {
                current_discontinuity = true;
            } else if line.starts_with("#EXT-X-STREAM-INF:") {
                has_variants = true;
                let params = &line[18..];
                let variant = Self::parse_variant_params(params, "");
                variants.push(variant);
            } else if !line.starts_with('#') && !line.is_empty() {
                if has_variants {
                    if let Some(last) = variants.last_mut() {
                        last.uri = Self::resolve_url(line, base_url);
                    }
                    has_variants = false;
                } else {
                    let uri = Self::resolve_url(line, base_url);
                    segments.push(M3U8Segment {
                        uri,
                        duration: current_duration,
                        title: current_title.clone(),
                        key: current_key.clone(),
                        map: current_map.clone(),
                        discontinuity: current_discontinuity,
                    });
                    current_duration = 0.0;
                    current_title = String::new();
                    current_discontinuity = false;
                }
            }
        }
        
        let playlist_type = if !variants.is_empty() {
            PlaylistType::MasterPlaylist
        } else if !segments.is_empty() {
            PlaylistType::MediaPlaylist
        } else {
            PlaylistType::Unknown
        };
        
        Ok(M3U8Playlist {
            playlist_type,
            segments,
            variants,
            target_duration,
            media_sequence,
            version,
            end_list,
        })
    }

    fn parse_key_params(params: &str) -> M3U8Key {
        let mut method = String::from("NONE");
        let mut uri = String::new();
        let mut iv = String::new();
        for part in params.split(',') {
            let kv: Vec<&str> = part.splitn(2, '=').collect();
            if kv.len() == 2 {
                let key = kv[0].trim();
                let value = kv[1].trim().trim_matches('"');
                match key {
                    "METHOD" => method = value.to_string(),
                    "URI" => uri = value.to_string(),
                    "IV" => iv = value.to_string(),
                    _ => {}
                }
            }
        }
        M3U8Key { method, uri, iv }
    }

    fn parse_map_params(params: &str) -> M3U8Map {
        let mut uri = String::new();
        let mut byterange = String::new();
        for part in params.split(',') {
            let kv: Vec<&str> = part.splitn(2, '=').collect();
            if kv.len() == 2 {
                let key = kv[0].trim();
                let value = kv[1].trim().trim_matches('"');
                match key {
                    "URI" => uri = value.to_string(),
                    "BYTERANGE" => byterange = value.to_string(),
                    _ => {}
                }
            }
        }
        M3U8Map { uri, byterange }
    }

    fn parse_variant_params(params: &str, _uri: &str) -> M3U8Variant {
        let mut bandwidth = 0;
        let mut resolution = String::new();
        let mut codecs = String::new();
        for part in params.split(',') {
            let kv: Vec<&str> = part.splitn(2, '=').collect();
            if kv.len() == 2 {
                let key = kv[0].trim();
                let value = kv[1].trim().trim_matches('"');
                match key {
                    "BANDWIDTH" => bandwidth = value.parse().unwrap_or(0),
                    "RESOLUTION" => resolution = value.to_string(),
                    "CODECS" => codecs = value.to_string(),
                    _ => {}
                }
            }
        }
        M3U8Variant { uri: String::new(), bandwidth, resolution, codecs }
    }

    fn resolve_url(url: &str, base_url: Option<&str>) -> String {
        if let Some(base) = base_url {
            if url.starts_with("http://") || url.starts_with("https://") {
                url.to_string()
            } else if url.starts_with('/') {
                let base_url_clean = base.trim_end_matches('/');
                format!("{}{}", base_url_clean, url)
            } else {
                let base_url_clean = base.trim_end_matches('/');
                if let Some(last_slash) = base_url_clean.rfind('/') {
                    format!("{}/{}", &base_url_clean[..=last_slash].trim_end_matches('/'), url)
                } else {
                    format!("{}/{}", base_url_clean, url)
                }
            }
        } else {
            url.to_string()
        }
    }

    pub fn get_best_variant(playlist: &M3U8Playlist) -> Option<&M3U8Variant> {
        playlist.variants.iter().max_by_key(|v| v.bandwidth)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_master_playlist() {
        let content = "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1280000,RESOLUTION=720x480\nlow.m3u8\n#EXT-X-STREAM-INF:BANDWIDTH=2560000,RESOLUTION=1920x1080\nhigh.m3u8";
        let result = M3U8Parser::parse(content, None).unwrap();
        assert_eq!(result.variants.len(), 2);
        assert_eq!(result.playlist_type, PlaylistType::MasterPlaylist);
    }

    #[test]
    fn test_parse_media_playlist() {
        let content = "#EXTM3U\n#EXT-X-TARGETDURATION:10\n#EXTINF:5.0,\nseg1.ts\n#EXTINF:5.0,\nseg2.ts\n#EXT-X-ENDLIST";
        let result = M3U8Parser::parse(content, None).unwrap();
        assert_eq!(result.segments.len(), 2);
        assert!(result.end_list);
    }

    #[test]
    fn test_parse_with_encryption() {
        let content = "#EXTM3U\n#EXT-X-KEY:METHOD=AES-128,URI=\"key.bin\",IV=0xabc\n#EXTINF:5.0,\nseg1.ts\n#EXT-X-ENDLIST";
        let result = M3U8Parser::parse(content, None).unwrap();
        assert_eq!(result.segments.len(), 1);
        assert!(result.segments[0].key.is_some());
        assert_eq!(result.segments[0].key.as_ref().unwrap().method, "AES-128");
    }

    #[test]
    fn test_resolve_url() {
        assert_eq!(M3U8Parser::resolve_url("seg.ts", Some("http://example.com/path/")), "http://example.com/path/seg.ts");
        assert_eq!(M3U8Parser::resolve_url("/seg.ts", Some("http://example.com")), "http://example.com/seg.ts");
        assert_eq!(M3U8Parser::resolve_url("http://other.com/seg.ts", Some("http://example.com")), "http://other.com/seg.ts");
    }

    #[test]
    fn test_parse_extinf() {
        let content = "#EXTM3U\n#EXTINF:10.5,测试标题\nseg.ts\n#EXT-X-ENDLIST";
        let result = M3U8Parser::parse(content, None).unwrap();
        assert_eq!(result.segments[0].duration, 10.5);
        assert_eq!(result.segments[0].title, "测试标题");
    }

    #[test]
    fn test_get_best_variant() {
        let content = "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1280000\nlow.m3u8\n#EXT-X-STREAM-INF:BANDWIDTH=2560000\nhigh.m3u8";
        let result = M3U8Parser::parse(content, None).unwrap();
        let best = M3U8Parser::get_best_variant(&result).unwrap();
        assert_eq!(best.bandwidth, 2560000);
    }
}