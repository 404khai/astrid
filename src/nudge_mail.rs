//! `astrid nudge-mail`: the resident presence host's header-only check for
//! new important mail (ADR 0042, amending ADR 0028 for this one use). It runs
//! only while the user has turned on the important-mail nudge in Settings. No
//! model is called; it prints one JSON line with at most 20 messages' id,
//! sender and subject, and never a snippet or body.
//!
//! "Important" is Gmail's own Important label (unread, last 2 days, not
//! promotions/social), or a sender the user listed. Apple Mail has no
//! importance marker Astrid can read, so there only listed senders count.
use astrid::{cancellation::Cancellation, mail::Mail};
use serde_json::{Value, json};

pub const MAX_SENDERS: usize = 10;
const MAX_QUERY: usize = 200;

/// A sender as a search term: printable, no quotes or parentheses.
fn term(sender: &str) -> Option<String> {
    let clean: String = sender
        .chars()
        .filter(|c| !c.is_control() && !matches!(c, '"' | '(' | ')' | '{' | '}'))
        .collect::<String>()
        .trim()
        .chars()
        .take(80)
        .collect();
    (!clean.is_empty()).then_some(clean)
}

/// The Gmail search for important new mail, within mail_search's 200 characters.
pub fn gmail_query(senders: &[String]) -> String {
    let base = "is:unread newer_than:2d -category:promotions -category:social";
    let mut from: Vec<String> = vec![];
    for sender in senders.iter().filter_map(|s| term(s)).take(MAX_SENDERS) {
        let quoted = format!("\"{sender}\"");
        let next = [from.clone(), vec![quoted.clone()]].concat().join(" OR ");
        if format!("{base} (is:important OR from:({next}))").len() > MAX_QUERY {
            break;
        }
        from.push(quoted);
    }
    if from.is_empty() {
        format!("{base} is:important")
    } else {
        format!("{base} (is:important OR from:({}))", from.join(" OR "))
    }
}

/// Only id, sender and subject leave this process.
fn headers(result: &Value) -> Vec<Value> {
    result["results"]
        .as_array()
        .map(|hits| {
            hits.iter()
                .map(|h| json!({"id":h["id"],"from":h["from"],"subject":h["subject"]}))
                .collect()
        })
        .unwrap_or_default()
}

pub async fn check(mail: &Mail, source: &str, senders: &[String]) -> Value {
    let cancel = Cancellation::default();
    let queries: Vec<String> = if source == "gmail" {
        vec![gmail_query(senders)]
    } else {
        senders
            .iter()
            .filter_map(|s| term(s))
            .take(MAX_SENDERS)
            .collect()
    };
    let mut messages: Vec<Value> = vec![];
    for query in queries {
        let arguments =
            json!({"source":source,"query":query,"limit":if source == "gmail" {20} else {5}});
        let request = match mail.prepare("mail_search", &arguments.to_string()) {
            Ok(request) => request,
            Err(error) => return json!({"error":error}),
        };
        match mail.execute(&request, &cancel).await {
            Ok(result) => messages.extend(headers(&result)),
            Err(error) => return json!({"error":error}),
        }
    }
    messages.dedup_by(|a, b| a["id"] == b["id"]);
    messages.truncate(20);
    json!({"source":source,"messages":messages})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gmail_query_is_bounded_and_strips_search_syntax() {
        assert_eq!(
            gmail_query(&[]),
            "is:unread newer_than:2d -category:promotions -category:social is:important"
        );
        let q = gmail_query(&["Ada Lovelace".into(), "bob@x.com".into(), "  ".into()]);
        assert_eq!(
            q,
            "is:unread newer_than:2d -category:promotions -category:social (is:important OR from:(\"Ada Lovelace\" OR \"bob@x.com\"))"
        );
        let hostile = gmail_query(&["a\") OR (label:secret".into()]);
        assert!(!hostile.contains("\") OR (") && hostile.contains("label:secret"));
        let many: Vec<String> = (0..30).map(|i| format!("someone{i}@example.com")).collect();
        assert!(gmail_query(&many).len() <= MAX_QUERY);
    }

    #[test]
    fn only_ids_senders_and_subjects_are_kept() {
        let result = json!({"results":[{"id":"m1","from":"Ada","subject":"Hi","date":"Mon","snippet":"secret text"}]});
        assert_eq!(
            headers(&result),
            vec![json!({"id":"m1","from":"Ada","subject":"Hi"})]
        );
    }
}
