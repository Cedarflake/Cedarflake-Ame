# Release notes

This directory owns the user-facing changes shown on GitHub Releases.

Create one UTF-8 Markdown file named exactly after its release tag, such as
`v0.1.0-alpha.2.md`. Describe changes since the previous version, how to download or upgrade,
and relevant known limits. Keep implementation diaries, CI results and audit records in
`docs/acceptance`.

Commit the notes to `main` before dispatching the release workflow. The publisher reads the
matching file from that dispatch's immutable workflow commit and uses its contents as the release
body. Missing, empty or invalid notes stop publication. Rerunning publication synchronizes the
body while retaining already verified package attachments. There is no generated-notes fallback.

- [v0.1.0-alpha.3](v0.1.0-alpha.3.md)
- [v0.1.0-alpha.2](v0.1.0-alpha.2.md)
- [v0.1.0-alpha.1](v0.1.0-alpha.1.md)
