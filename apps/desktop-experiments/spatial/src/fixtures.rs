//! Replaceable presentation data source, with no universal client protocol.
#[derive(Clone)]
pub struct Session {
    pub id: usize,
    pub name: String,
    pub identity: &'static str,
    pub color: u32,
    pub status: &'static str,
    pub summary: &'static str,
    pub prompt: &'static str,
    pub response: &'static str,
    pub activities: Vec<&'static str>,
    pub artifact: &'static str,
}
pub trait SessionSource {
    fn sessions(&self) -> Vec<Session>;
}
pub struct MockSource;
impl SessionSource for MockSource {
    fn sessions(&self) -> Vec<Session> {
        vec![
            Session {
                id: 0,
                name: "Meister".into(),
                identity: "Your primary assistant",
                color: 0x00dfc4,
                status: "Ready for you",
                summary: "A little more room to think.",
                prompt: "What should I give my attention to today?",
                response: "Your scholarship shortlist is ready to review. Nora has a small Keihatsu fix waiting for your feedback. We can take them one at a time.",
                activities: vec![
                    "3 updates gathered across your workspace",
                    "Your layout stays exactly where you left it",
                ],
                artifact: "Today's priorities · Preview",
            },
            Session {
                id: 1,
                name: "Scholarship Watch".into(),
                identity: "Shelly · Research & monitoring",
                color: 0xff83d8,
                status: "Needs your review",
                summary: "Three opportunities worth a closer look.",
                prompt: "Find scholarships that fit my research interests.",
                response: "In this example shortlist, three programmes match your interests. Two need a statement of purpose; one asks for a research proposal.",
                activities: vec![
                    "Sample shortlist prepared · 3 opportunities",
                    "Example deadline · 14 November",
                ],
                artifact: "Scholarship shortlist · 3 entries",
            },
            Session {
                id: 2,
                name: "Keihatsu".into(),
                identity: "Nora · Coding Astrix",
                color: 0xff4ca5,
                status: "Feedback requested",
                summary: "A calmer reading experience.",
                prompt: "Fix the manga details fetching issue.",
                response: "The fixture change keeps cached details visible while a refresh is in progress. The patch preview is ready for a closer look.",
                activities: vec![
                    "Mock patch · MangaDetailsRepository.swift",
                    "Fixture checks · 8 examples passing",
                ],
                artifact: "Patch preview · +24 −9",
            },
            Session {
                id: 3,
                name: "Research".into(),
                identity: "Corbin · Independent Astrix",
                color: 0xf38b65,
                status: "Draft ready",
                summary: "What makes a workspace feel spatial?",
                prompt: "Explore spatial interfaces for personal work.",
                response: "Stable locations help you remember work. A useful overview preserves identity, while focus reveals detail. Motion should explain the journey, then get out of the way.",
                activities: vec![
                    "Reference notes · 4 patterns compared",
                    "Open question · How should relevance be signalled?",
                ],
                artifact: "Spatial interface notes · Draft",
            },
            Session {
                id: 4,
                name: "Daily Brief".into(),
                identity: "Meister · Across your sessions",
                color: 0xb3a0fa,
                status: "Summary ready",
                summary: "Your day, with the noise turned down.",
                prompt: "Give me the changes that matter.",
                response: "Three scholarship examples shortlisted. One Keihatsu patch awaiting review. Research notes ready. Nothing here is running in the background.",
                activities: vec![
                    "5 fixture sessions · No live connections",
                    "All activity in this workspace is illustrative",
                ],
                artifact: "Daily brief · Example digest",
            },
        ]
    }
}
pub fn search(sessions: &[Session], query: &str) -> Vec<usize> {
    let words: Vec<_> = query.split_whitespace().map(str::to_lowercase).collect();
    sessions
        .iter()
        .filter(|s| {
            let haystack =
                format!("{} {} {} {}", s.name, s.identity, s.status, s.summary).to_lowercase();
            words.iter().all(|w| haystack.contains(w))
        })
        .map(|s| s.id)
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deterministic_search_matches_identity_and_purpose() {
        let data = MockSource.sessions();
        assert_eq!(search(&data, "SCHOLAR"), vec![1]);
        assert_eq!(search(&data, "nora coding"), vec![2]);
        assert_eq!(search(&data, ""), vec![0, 1, 2, 3, 4]);
        assert_eq!(search(&data, "no such session"), Vec::<usize>::new());
        assert_eq!(search(&[], ""), Vec::<usize>::new());
    }
    #[test]
    fn ids_unique_and_names_can_be_long() {
        let mut data = MockSource.sessions();
        data[0].name = "A deliberately long session name ".repeat(10);
        assert_eq!(search(&data, "deliberately"), vec![0]);
        let mut ids: Vec<_> = data.iter().map(|s| s.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 5);
    }
}
