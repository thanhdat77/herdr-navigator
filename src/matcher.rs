use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use nucleo_matcher::{
    pattern::{CaseMatching, Normalization, Pattern},
    Config as NucleoConfig, Matcher as NucleoMatcher, Utf32Str,
};

pub(crate) fn match_score(engine: &str, hay: &str, q: &str) -> Option<i64> {
    match engine {
        "skim" => SkimMatcherV2::default().fuzzy_match(hay, q),
        "simple" => simple_fuzzy_score(hay, q).map(|score| -score),
        _ => {
            let mut matcher = NucleoMatcher::new(NucleoConfig::DEFAULT.match_paths());
            let pattern = Pattern::parse(q, CaseMatching::Ignore, Normalization::Smart);
            let mut buf = Vec::new();
            pattern
                .score(Utf32Str::new(hay, &mut buf), &mut matcher)
                .map(|score| score as i64)
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
