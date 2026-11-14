# Blobs

The little bots in the desktop app's sidebar (ADR 0037, blobs amendment).

These ten characters (Strobi, Freddy, Citrus, Nova, Grok bot, Sunee, Kirby,
Cloudee, Cubee and Onee) are from the built-in library of
[Bible Strong Avatar Lab](https://github.com/smontlouis/bible-strong-avatar-lab)
by Stéphane Montlouis (`src/features/editor/defaultLibrary.json`, commit
`b1cf283`), licensed under the **GNU Affero General Public License v3.0**:
see [LICENSE](LICENSE). This folder, and only this folder, is under that
licence. The rest of Astrid is MIT (see the repository's LICENSE).

Changes from the original: each character was split into its own file and
converted from the lab's editor format to the character format that the MIT
runtime `@bible-strong/avatar-web` reads. Each keeps its name, body, parts and
colours. Its eyes use the standard build, at the original height. Editor-only
ids were removed.

The app draws these with `@bible-strong/avatar-web` (MIT). Bots you export
yourself from avatars.bible-strong.app go in
`~/Library/Application Support/Astrid/Blobs`, and the app shows them too.
