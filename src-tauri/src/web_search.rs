use crate::models::WebSearchResult;
use regex::Regex;
use reqwest::{Client, Url};
use serde_json::Value;
use std::time::Duration;

pub async fn search(
    query: &str,
    max_results: usize,
    bypass_proxy: bool,
) -> Result<Vec<WebSearchResult>, String> {
    let limit = max_results.clamp(1, 8);
    let mut builder = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Locastra/0.1")
        .connect_timeout(Duration::from_secs(12))
        .timeout(Duration::from_secs(25));
    if bypass_proxy {
        builder = builder.no_proxy();
    }
    let client = builder.build().map_err(|e| e.to_string())?;

    match search_bing_rss(&client, query, limit).await {
        Ok(results) if !results.is_empty() => Ok(results),
        bing_result => match search_duckduckgo(&client, query, limit).await {
            Ok(results) if !results.is_empty() => Ok(results),
            duck_result => Err(format!(
                "联网搜索暂时不可用。Bing：{}；DuckDuckGo：{}",
                error_summary(bing_result),
                error_summary(duck_result)
            )),
        },
    }
}

async fn search_bing_rss(
    client: &Client,
    query: &str,
    limit: usize,
) -> Result<Vec<WebSearchResult>, String> {
    let mut url = Url::parse("https://www.bing.com/search").map_err(|e| e.to_string())?;
    url.query_pairs_mut()
        .append_pair("q", query)
        .append_pair("format", "rss")
        .append_pair("setlang", "zh-hans")
        .append_pair("mkt", "zh-CN")
        .append_pair("cc", "CN");
    let body = client
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;
    Ok(rank_relevant(
        parse_bing_rss(&body, limit * 3),
        query,
        limit,
    ))
}

fn parse_bing_rss(body: &str, limit: usize) -> Vec<WebSearchResult> {
    let item_re = Regex::new(r"(?s)<item>(.*?)</item>").expect("valid item regex");
    let title_re = Regex::new(r"(?s)<title>(.*?)</title>").expect("valid title regex");
    let link_re = Regex::new(r"(?s)<link>(.*?)</link>").expect("valid link regex");
    let description_re =
        Regex::new(r"(?s)<description>(.*?)</description>").expect("valid description regex");
    item_re
        .captures_iter(body)
        .filter_map(|item| {
            let xml = item.get(1)?.as_str();
            let title = capture_text(&title_re, xml)?;
            let url = capture_text(&link_re, xml)?;
            if !matches!(Url::parse(&url), Ok(parsed) if matches!(parsed.scheme(), "http" | "https")) {
                return None;
            }
            let snippet = capture_text(&description_re, xml).unwrap_or_default();
            Some(WebSearchResult {
                title: truncate(&title, 180),
                url,
                snippet: truncate(&strip_tags(&snippet), 480),
                engine: "bing".into(),
            })
        })
        .take(limit)
        .collect()
}

async fn search_duckduckgo(
    client: &Client,
    query: &str,
    limit: usize,
) -> Result<Vec<WebSearchResult>, String> {
    let mut url = Url::parse("https://api.duckduckgo.com/").map_err(|e| e.to_string())?;
    url.query_pairs_mut()
        .append_pair("q", query)
        .append_pair("format", "json")
        .append_pair("no_html", "1")
        .append_pair("skip_disambig", "1")
        .append_pair("no_redirect", "1");
    let value: Value = client
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    let mut results = Vec::new();
    if let (Some(title), Some(url)) = (
        value.get("Heading").and_then(Value::as_str),
        value.get("AbstractURL").and_then(Value::as_str),
    ) {
        let snippet = value
            .get("AbstractText")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !title.is_empty() && !url.is_empty() {
            results.push(WebSearchResult {
                title: truncate(title, 180),
                url: url.into(),
                snippet: truncate(snippet, 480),
                engine: "duckduckgo".into(),
            });
        }
    }
    collect_related(value.get("RelatedTopics"), &mut results, limit);
    Ok(rank_relevant(results, query, limit))
}

fn rank_relevant(results: Vec<WebSearchResult>, query: &str, limit: usize) -> Vec<WebSearchResult> {
    let terms = relevance_terms(query);
    if terms.is_empty() {
        return results.into_iter().take(limit).collect();
    }
    let mut scored = results
        .into_iter()
        .filter_map(|result| {
            let title = result.title.to_lowercase();
            let corpus = format!(
                "{title} {} {}",
                result.url.to_lowercase(),
                result.snippet.to_lowercase()
            );
            let score = terms.iter().fold(0usize, |score, term| {
                score
                    + if title.contains(term) { 4 } else { 0 }
                    + if corpus.contains(term) { 1 } else { 0 }
            });
            (score > 0).then_some((score, result))
        })
        .collect::<Vec<_>>();
    scored.sort_by(|left, right| right.0.cmp(&left.0));
    scored
        .into_iter()
        .take(limit)
        .map(|(_, result)| result)
        .collect()
}

fn relevance_terms(query: &str) -> Vec<String> {
    let lower = query.to_lowercase();
    let stop_words = [
        "model", "official", "features", "site", "com", "html", "website",
    ];
    let mut terms = lower
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '.')
        .filter(|term| term.len() >= 3 && !stop_words.contains(term))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let chinese = lower
        .chars()
        .filter(|character| matches!(*character as u32, 0x3400..=0x9fff))
        .collect::<Vec<_>>();
    for pair in chinese.windows(2) {
        terms.push(pair.iter().collect());
    }
    terms.sort();
    terms.dedup();
    terms
}

fn collect_related(value: Option<&Value>, output: &mut Vec<WebSearchResult>, limit: usize) {
    let Some(items) = value.and_then(Value::as_array) else {
        return;
    };
    for item in items {
        if output.len() >= limit {
            break;
        }
        if let Some(nested) = item.get("Topics") {
            collect_related(Some(nested), output, limit);
            continue;
        }
        let Some(text) = item.get("Text").and_then(Value::as_str) else {
            continue;
        };
        let Some(url) = item.get("FirstURL").and_then(Value::as_str) else {
            continue;
        };
        output.push(WebSearchResult {
            title: truncate(text.split(" - ").next().unwrap_or(text), 180),
            url: url.into(),
            snippet: truncate(text, 480),
            engine: "duckduckgo".into(),
        });
    }
}

fn capture_text(regex: &Regex, xml: &str) -> Option<String> {
    regex
        .captures(xml)
        .and_then(|capture| capture.get(1))
        .map(|value| xml_unescape(value.as_str().trim()))
        .filter(|value| !value.is_empty())
}

fn xml_unescape(value: &str) -> String {
    value
        .replace("<![CDATA[", "")
        .replace("]]>", "")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

fn strip_tags(value: &str) -> String {
    Regex::new(r"<[^>]+>")
        .expect("valid tag regex")
        .replace_all(value, " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn truncate(value: &str, max: usize) -> String {
    let mut chars = value.chars();
    let head: String = chars.by_ref().take(max).collect();
    if chars.next().is_some() {
        format!("{head}…")
    } else {
        head
    }
}

fn error_summary<T>(result: Result<Vec<T>, String>) -> String {
    match result {
        Ok(_) => "无结果".into(),
        Err(error) => error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bing_rss_and_rejects_non_http_links() {
        let xml = r#"<rss><channel><item><title>Qwen &amp; Locastra</title><link>https://example.com/qwen</link><description><![CDATA[<b>本地</b> 推理说明]]></description></item><item><title>坏链接</title><link>file:///c:/secret</link><description>skip</description></item></channel></rss>"#;
        let results = parse_bing_rss(xml, 5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Qwen & Locastra");
        assert_eq!(results[0].snippet, "本地 推理说明");
    }

    #[test]
    fn removes_irrelevant_search_results_and_prefers_title_matches() {
        let results = vec![
            WebSearchResult {
                title: "Microsoft Outlook support".into(),
                url: "https://microsoft.com/outlook".into(),
                snippet: "Email help".into(),
                engine: "bing".into(),
            },
            WebSearchResult {
                title: "Qwen3.8-27B model card".into(),
                url: "https://huggingface.co/Qwen/Qwen3.8-27B".into(),
                snippet: "Official parameters and capabilities".into(),
                engine: "bing".into(),
            },
        ];
        let ranked = rank_relevant(results, "Qwen3.8-27B 模型 官方 参数 特性", 5);
        assert_eq!(ranked.len(), 1);
        assert!(ranked[0].title.contains("Qwen3.8"));
    }
}
