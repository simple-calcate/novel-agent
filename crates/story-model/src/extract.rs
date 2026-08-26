//! 从正文抽取正史候选。首版只用确定性启发式，不调用模型。
//!
//! 规则（网文常见写法）：
//! - `林晚说道` / `沈雾问道` → 人物
//! - `《潮汐秘录》` → 物/典籍
//! - `走进雾港码头` / `来到青州` → 地点

use novel_domain::{EntityKind, ExtractedMention};
use std::collections::HashSet;

const SPEAKER_SUFFIXES: &[&str] = &[
    "冷冷道",
    "淡淡道",
    "低声道",
    "缓缓道",
    "说道",
    "问道",
    "笑道",
    "喝道",
    "叹道",
    "答道",
    "怒道",
    "叫道",
];

const LOCATION_PREFIXES: &[&str] = &["来到了", "走进了", "抵达了", "来到", "走进", "抵达"];

const NAME_STOPWORDS: &[&str] = &[
    "然后", "于是", "只是", "可是", "但是", "这时", "此时", "忽然", "突然", "终于", "其实", "因为",
    "所以", "如果", "虽然", "他们", "她们", "我们", "自己", "这个", "那个", "什么", "没有", "不是",
    "已经", "还是", "或者", "以及",
];

pub fn extract_mentions(text: &str) -> Vec<ExtractedMention> {
    let mut mentions = Vec::new();
    extract_speakers(text, &mut mentions);
    extract_titles(text, &mut mentions);
    extract_locations(text, &mut mentions);
    dedupe(mentions)
}

fn extract_speakers(text: &str, out: &mut Vec<ExtractedMention>) {
    for suffix in SPEAKER_SUFFIXES {
        let mut search = 0;
        while let Some(rel) = text[search..].find(suffix) {
            let at = search + rel;
            if let Some(name) = preceding_cjk_name(text, at) {
                if is_plausible_name(name) {
                    let end = at + suffix.len();
                    out.push(ExtractedMention {
                        entity_name: name.to_owned(),
                        entity_kind: EntityKind::Character,
                        predicate: "appearsAsSpeaker".into(),
                        object: name.to_owned(),
                        quote: quote_around(text, at.saturating_sub(name.len()), end),
                        confidence: 0.82,
                    });
                }
            }
            search = at + suffix.len();
        }
    }
}

fn extract_titles(text: &str, out: &mut Vec<ExtractedMention>) {
    let mut search = 0;
    while let Some(rel) = text[search..].find('《') {
        let start = search + rel;
        let after = start + '《'.len_utf8();
        if let Some(rel_end) = text[after..].find('》') {
            let title = &text[after..after + rel_end];
            if (2..=20).contains(&title.chars().count()) && !title.contains('\n') {
                let end = after + rel_end + '》'.len_utf8();
                out.push(ExtractedMention {
                    entity_name: title.to_owned(),
                    entity_kind: EntityKind::Item,
                    predicate: "titledWork".into(),
                    object: title.to_owned(),
                    quote: quote_around(text, start, end),
                    confidence: 0.78,
                });
            }
            search = after + rel_end + '》'.len_utf8();
        } else {
            break;
        }
    }
}

fn extract_locations(text: &str, out: &mut Vec<ExtractedMention>) {
    for prefix in LOCATION_PREFIXES {
        let mut search = 0;
        while let Some(rel) = text[search..].find(prefix) {
            let at = search + rel;
            let after = at + prefix.len();
            if let Some(name) = following_cjk_name(text, after, 2, 12) {
                if is_plausible_name(name) {
                    let end = after + name.len();
                    out.push(ExtractedMention {
                        entity_name: name.to_owned(),
                        entity_kind: EntityKind::Location,
                        predicate: "mentionedLocation".into(),
                        object: name.to_owned(),
                        quote: quote_around(text, at, end),
                        confidence: 0.64,
                    });
                }
            }
            search = after;
        }
    }
}

fn preceding_cjk_name(text: &str, suffix_start: usize) -> Option<&str> {
    let mut end = suffix_start;
    while end > 0 {
        let ch = text[..end].chars().next_back()?;
        if ch.is_whitespace() {
            end -= ch.len_utf8();
        } else {
            break;
        }
    }
    let mut start = end;
    let mut count = 0usize;
    while start > 0 && count < 8 {
        let ch = text[..start].chars().next_back()?;
        if is_cjk(ch) {
            start -= ch.len_utf8();
            count += 1;
        } else {
            break;
        }
    }
    if count < 2 {
        return None;
    }
    Some(&text[start..end])
}

fn following_cjk_name(text: &str, from: usize, min: usize, max: usize) -> Option<&str> {
    let mut start = from;
    while start < text.len() {
        let ch = text[start..].chars().next()?;
        if ch.is_whitespace() {
            start += ch.len_utf8();
        } else {
            break;
        }
    }
    let mut end = start;
    let mut count = 0usize;
    while end < text.len() && count < max {
        let ch = text[end..].chars().next()?;
        if is_cjk(ch) {
            end += ch.len_utf8();
            count += 1;
        } else {
            break;
        }
    }
    if count < min {
        return None;
    }
    Some(&text[start..end])
}

fn is_cjk(c: char) -> bool {
    ('\u{4e00}'..='\u{9fff}').contains(&c)
}

fn is_plausible_name(name: &str) -> bool {
    !NAME_STOPWORDS.contains(&name)
}

fn quote_around(text: &str, start: usize, end: usize) -> String {
    let start = floor_char(text, start.saturating_sub(12));
    let end = ceil_char(text, (end + 12).min(text.len()));
    text[start..end].replace('\n', " ").trim().to_owned()
}

fn floor_char(text: &str, mut index: usize) -> usize {
    index = index.min(text.len());
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn ceil_char(text: &str, mut index: usize) -> usize {
    index = index.min(text.len());
    while index < text.len() && !text.is_char_boundary(index) {
        index += 1;
    }
    index
}

fn dedupe(mentions: Vec<ExtractedMention>) -> Vec<ExtractedMention> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for mention in mentions {
        let key = (
            mention.entity_kind.clone(),
            mention.entity_name.clone(),
            mention.predicate.clone(),
        );
        if seen.insert(key) {
            out.push(mention);
        }
    }
    out
}
