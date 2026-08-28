//! 预先结构的多信号匹配：全名、别名、标题词核、设定关键词、上一段 dwell。
//! 不用模型，不做正文抽取。

use novel_domain::StoryEntry;

#[derive(Debug, Clone, PartialEq)]
pub struct EntryMatch {
    pub score: f32,
    pub reason: String,
}

const STOPWORDS: &[&str] = &[
    "然后",
    "于是",
    "只是",
    "可是",
    "但是",
    "这时",
    "此时",
    "忽然",
    "突然",
    "终于",
    "其实",
    "因为",
    "所以",
    "如果",
    "虽然",
    "他们",
    "她们",
    "我们",
    "自己",
    "这个",
    "那个",
    "什么",
    "没有",
    "不是",
    "已经",
    "还是",
    "或者",
    "以及",
    "里面",
    "还有",
    "一个",
    "没有",
    "现在",
    "后来",
    "开始",
    "继续",
    "出现",
    "时候",
    "地方",
    "东西",
    "补充",
    "说明",
    "预先",
    "设定",
    "人物",
    "伏笔",
    "条目",
    "终年",
    "夜晚",
    "海面",
    "进来",
    "出去",
    "看见",
    "听到",
    "知道",
    "觉得",
    "走过",
    "走进",
    "来到",
    "抵达",
    "到了",
    "快到",
    "快到了",
];

const SPLITTERS: &[char] = &[
    '，', '。', '；', '、', ',', '.', '!', '?', '！', '？', '：', ':', '\n', '\t', ' ', '「', '」',
    '“', '”', '\'', '（', '）', '(', ')', '《', '》', '—', '…', '·', '的', '地', '得',
];

pub fn match_story_entry(current: &str, lookback: &str, entry: &StoryEntry) -> Option<EntryMatch> {
    let raw_title = entry.title.trim();
    let (canonical, extra) = novel_domain::split_title_and_aliases(raw_title);
    let title_owned = if canonical.chars().count() >= 2 {
        canonical
    } else {
        raw_title.to_string()
    };
    let title = title_owned.as_str();
    if title.chars().count() < 2 {
        return None;
    }
    let mut aliases: Vec<String> = entry
        .aliases
        .iter()
        .map(|alias| alias.trim().to_string())
        .filter(|alias| alias.chars().count() >= 2 && alias.as_str() != title)
        .collect();
    for alias in extra {
        if alias.chars().count() >= 2 && alias != title && !aliases.contains(&alias) {
            aliases.push(alias);
        }
    }
    let current_l = normalize(current);
    let lookback_l = normalize(lookback);

    let mut best: Option<EntryMatch> = None;
    let mut extra = 0u32;

    if let Some(index) = find_term(&current_l, title) {
        consider(
            &mut best,
            &mut extra,
            EntryMatch {
                score: 0.98 * position_boost(&current_l, index),
                reason: format!("出现名称「{title}」"),
            },
        );
    }
    for alias in &aliases {
        if let Some(index) = find_term(&current_l, alias) {
            consider(
                &mut best,
                &mut extra,
                EntryMatch {
                    score: 0.90 * position_boost(&current_l, index),
                    reason: format!("出现别名「{alias}」"),
                },
            );
        }
    }
    for core in title_cores(title) {
        if let Some(index) = find_term(&current_l, &core) {
            consider(
                &mut best,
                &mut extra,
                EntryMatch {
                    score: 0.72 * position_boost(&current_l, index),
                    reason: format!("提到「{core}」"),
                },
            );
        }
    }
    let mut keyword_hits = 0u32;
    let mut keyword_best: Option<EntryMatch> = None;
    for keyword in summary_keywords(&entry.summary, title, &aliases) {
        if let Some(index) = find_term(&current_l, &keyword) {
            keyword_hits += 1;
            let candidate = EntryMatch {
                score: 0.58 * position_boost(&current_l, index),
                reason: format!("设定里提到「{keyword}」"),
            };
            if keyword_best
                .as_ref()
                .map(|hit| candidate.score > hit.score)
                .unwrap_or(true)
            {
                keyword_best = Some(candidate);
            }
        }
    }
    if let Some(mut hit) = keyword_best {
        hit.score = (hit.score + 0.06 * (keyword_hits.saturating_sub(1) as f32)).min(0.76);
        consider(&mut best, &mut extra, hit);
    }

    if best.is_none() {
        if let Some(index) = find_term(&lookback_l, title) {
            return Some(EntryMatch {
                score: 0.60 * position_boost(&lookback_l, index),
                reason: format!("上一段出现「{title}」"),
            });
        }
        for alias in &aliases {
            if find_term(&lookback_l, alias).is_some() {
                return Some(EntryMatch {
                    score: 0.56,
                    reason: format!("上一段出现别名「{alias}」"),
                });
            }
        }
        return retrieve_story_entry(current, entry);
    }

    if let Some(hit) = &mut best {
        if extra > 0 {
            hit.score = (hit.score + 0.04 * extra as f32).min(1.0);
        }
    }
    best
}

/// 第二级预算：用当前段里的词去条目标题/说明里做词汇检索。
/// 本地规则没命中时才走到这里；不用模型。
pub fn retrieve_story_entry(current: &str, entry: &StoryEntry) -> Option<EntryMatch> {
    let haystack = normalize(&format!(
        "{} {} {}",
        entry.title,
        entry.aliases.join(" "),
        entry.summary
    ));
    let mut matched: Vec<String> = query_tokens(current)
        .into_iter()
        .filter(|token| haystack.contains(&normalize(token)))
        .collect();
    if matched.is_empty() {
        return None;
    }
    matched.sort_by_key(|token| std::cmp::Reverse(token.chars().count()));
    matched.dedup();
    let term = matched[0].as_str();
    Some(EntryMatch {
        score: 0.40,
        reason: format!("检索到「{term}」"),
    })
}

fn query_tokens(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        if SPLITTERS.contains(&ch) {
            push_query_chunk(&mut tokens, &current);
            current.clear();
        } else {
            current.push(ch);
        }
    }
    push_query_chunk(&mut tokens, &current);
    tokens
}

fn push_query_chunk(tokens: &mut Vec<String>, chunk: &str) {
    let chunk = chunk.trim();
    if chunk.is_empty() {
        return;
    }
    let chars: Vec<char> = chunk.chars().collect();
    let n = chars.len();
    if n < 2 {
        return;
    }
    if n <= 12 {
        accept_query_token(tokens, chunk);
    }
    let max_len = n.min(4);
    for len in 2..=max_len {
        for start in 0..=n - len {
            let term: String = chars[start..start + len].iter().collect();
            accept_query_token(tokens, &term);
        }
    }
}

fn accept_query_token(tokens: &mut Vec<String>, token: &str) {
    if token.chars().count() < 2 || is_stop(token) {
        return;
    }
    if !tokens.iter().any(|existing| existing == token) {
        tokens.push(token.to_owned());
    }
}

fn consider(best: &mut Option<EntryMatch>, extra: &mut u32, candidate: EntryMatch) {
    match best {
        None => *best = Some(candidate),
        Some(hit) if candidate.score > hit.score => {
            *extra += 1;
            *best = Some(candidate);
        }
        Some(_) => *extra += 1,
    }
}

fn title_cores(title: &str) -> Vec<String> {
    let chars: Vec<char> = title.chars().collect();
    let n = chars.len();
    if n < 3 {
        return Vec::new();
    }
    let mut cores = Vec::new();
    let suffix2: String = chars[n - 2..].iter().collect();
    if n >= 3 && !is_stop(&suffix2) && suffix2 != title {
        cores.push(suffix2);
    }
    if n >= 4 {
        let suffix3: String = chars[n - 3..].iter().collect();
        if !is_stop(&suffix3) {
            cores.push(suffix3);
        }
    }
    cores
}

fn summary_keywords(summary: &str, title: &str, aliases: &[String]) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for ch in summary.chars() {
        if SPLITTERS.contains(&ch) {
            push_token(&mut tokens, &current, title, aliases);
            current.clear();
        } else {
            current.push(ch);
        }
    }
    push_token(&mut tokens, &current, title, aliases);
    tokens.sort_by_key(|token| std::cmp::Reverse(token.chars().count()));
    tokens.dedup();
    tokens
}

fn push_token(tokens: &mut Vec<String>, chunk: &str, title: &str, aliases: &[String]) {
    let chunk = chunk.trim();
    if chunk.is_empty() {
        return;
    }
    let chars: Vec<char> = chunk.chars().collect();
    let n = chars.len();
    if !(2..=12).contains(&n) {
        return;
    }
    accept_token(tokens, chunk, title, aliases);
    if n >= 4 {
        let first2: String = chars[..2].iter().collect();
        let last2: String = chars[n - 2..].iter().collect();
        let last3: String = chars[n - 3..].iter().collect();
        accept_token(tokens, &first2, title, aliases);
        accept_token(tokens, &last2, title, aliases);
        accept_token(tokens, &last3, title, aliases);
    } else if n == 3 {
        let first2: String = chars[..2].iter().collect();
        let last2: String = chars[n - 2..].iter().collect();
        accept_token(tokens, &first2, title, aliases);
        accept_token(tokens, &last2, title, aliases);
    }
}

fn accept_token(tokens: &mut Vec<String>, token: &str, title: &str, aliases: &[String]) {
    if token.chars().count() < 2 || is_stop(token) {
        return;
    }
    if token == title || aliases.iter().any(|alias| alias == token) {
        return;
    }
    if title.contains(token) {
        return;
    }
    if !tokens.iter().any(|existing| existing == token) {
        tokens.push(token.to_owned());
    }
}

fn is_stop(token: &str) -> bool {
    STOPWORDS.contains(&token)
}

fn normalize(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .collect::<String>()
        .replace(['　', '\u{00a0}'], " ")
}

fn find_term(haystack: &str, term: &str) -> Option<usize> {
    let needle = normalize(term);
    if needle.chars().count() < 2 {
        return None;
    }
    haystack.find(&needle)
}

fn position_boost(text: &str, index: usize) -> f32 {
    let length = text.len().max(1) as f32;
    (1.0 - (index as f32 / length) * 0.2).clamp(0.8, 1.0)
}
