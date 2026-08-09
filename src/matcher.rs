use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use nucleo_matcher::{
    pattern::{CaseMatching, Normalization, Pattern},
    Config as NucleoConfig, Matcher as NucleoMatcher, Utf32Str,
};

/// A fuzzy matcher prepared for one query.
///
/// Building the engine and parsing the pattern is not free, so it happens once
/// per query here instead of once per candidate: `apply_filter` scores every
/// entry on every keystroke.
pub(crate) enum Scorer {
    Nucleo {
        matcher: Box<NucleoMatcher>,
        pattern: Pattern,
        buf: Vec<char>,
    },
    Skim {
        matcher: Box<SkimMatcherV2>,
        query: String,
    },
    Simple {
        query: String,
    },
}

impl Scorer {
    pub(crate) fn new(engine: &str, query: &str) -> Self {
        match engine {
            "skim" => Scorer::Skim {
                matcher: Box::new(SkimMatcherV2::default()),
                query: query.to_string(),
            },
            "simple" => Scorer::Simple {
                query: query.to_string(),
            },
            _ => Scorer::Nucleo {
                matcher: Box::new(NucleoMatcher::new(NucleoConfig::DEFAULT.match_paths())),
                pattern: Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart),
                buf: Vec::new(),
            },
        }
    }

    pub(crate) fn score(&mut self, hay: &str) -> Option<i64> {
        match self {
            Scorer::Nucleo {
                matcher,
                pattern,
                buf,
            } => pattern
                .score(Utf32Str::new(hay, buf), matcher)
                .map(|score| score as i64),
            Scorer::Skim { matcher, query } => matcher.fuzzy_match(hay, query),
            Scorer::Simple { query } => simple_fuzzy_score(hay, query).map(|score| -score),
        }
    }
}

fn simple_fuzzy_score(hay: &str, q: &str) -> Option<i64> {
    let mut score = 0;
    let mut pos = 0;
    for qc in q.chars() {
        let rest = &hay[pos..];
        let found = rest.find(qc)?;
        score += found as i64;
        pos += found + qc.len_utf8();
    }
    Some(score)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_engine_matches_and_rejects() {
        for engine in ["nucleo", "skim", "simple"] {
            let mut scorer = Scorer::new(engine, "dot");
            assert!(
                scorer
                    .score("workspace dotfiles /home/u/dotfiles")
                    .is_some(),
                "{engine} should match"
            );
            assert!(scorer.score("zzz").is_none(), "{engine} should not match");
        }
    }

    #[test]
    fn scorer_is_reusable_across_candidates() {
        for engine in ["nucleo", "skim", "simple"] {
            let mut scorer = Scorer::new(engine, "dot");
            let first = scorer.score("dotfiles");
            let again = scorer.score("dotfiles");
            assert_eq!(first, again, "{engine} must be stateless across candidates");
        }
    }
}
