---
cairn: delta
change: adversarial-parser-tests
---

## ADDED Requirements

### Requirement: The parsers are robust against a hostile server
`CarillonImapWatch::resume` and `CarillonCardDavPoll::resume` parse an untrusted,
user-named server's responses in-process and SHALL treat malformed, truncated, or
oversized input as a terminating error — never a panic, unbounded allocation, or
hang. The IMAP fragmentizer SHALL cap a single message/literal at `MAX_MESSAGE_SIZE`
(1 MiB). A stable-Rust adversarial corpus driven through each `resume` under a bounded
loop guards this; coverage-guided fuzzing is a follow-up.
