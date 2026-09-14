use crate::command::{CompletionCandidate, CompletionRequest};

const TARGETS: &[&str] = &["path", "subtree", "value"];

/// Completes arguments using character offsets relative to the argument string.
pub(crate) fn complete(arguments: &str, cursor: usize) -> Option<CompletionRequest> {
    let characters = arguments.chars().collect::<Vec<_>>();
    let prefix = characters.get(..cursor)?.iter().collect::<String>();
    let token_end = cursor
        + characters[cursor..]
            .iter()
            .take_while(|character| !character.is_whitespace())
            .count();
    let raw_arguments = prefix.as_str();
    let arguments = raw_arguments.trim_start();
    if arguments.chars().any(char::is_whitespace) {
        return None;
    }
    let target_start = raw_arguments.chars().count() - arguments.chars().count();
    let candidates = TARGETS
        .iter()
        .filter(|target| target.starts_with(arguments) && **target != arguments)
        .map(|target| CompletionCandidate {
            display: (*target).to_owned(),
            replacement: (*target).to_owned(),
        })
        .collect::<Vec<_>>();
    (!candidates.is_empty()).then_some(CompletionRequest {
        replacement: target_start..token_end,
        candidates,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completes_targets() {
        let request = complete("sub", 3).unwrap();
        assert_eq!(request.replacement, 0..3);
        assert_eq!(request.candidates[0].replacement, "subtree");
        assert!(complete("subtree", 7).is_none());
    }
}
