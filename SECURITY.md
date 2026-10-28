# Security

Found a security problem in Meldo? Please don't open a public issue. Email **contact@meldo.dev** with what you found and how to reproduce it. You'll get an answer within a week.

In this repo, the most sensitive code parses data from school servers (`crates/melde-untis`): untrusted JSON that must never panic, allocate without bounds or follow attacker-chosen hosts.

Please test only against your own accounts and devices. Don't probe WebUntis servers.
