# Offline preview and revision review verification

PR 5 adds searchable, revision-scoped human review assertions with retained
evidence, logical names/tags, and offline HTML galleries for selected assets or
search pages. Existing images/GIF/audio are copied without transformation.

Two integration fixtures validate escaped hostile metadata, CSP, byte-identical
WAV copies, exact GIF delays of 70/190 ms, unavailable-source reporting, no-clobber
output, immutable evidence retention and independent approval states across two
revisions. Fifteen existing library tests pass. The new default CLI smoke uses
synthetic PNG/GIF/WAV media, an old retained image and a new image revision: four
preview cards, distinct review states, search filters and metadata annotation all
pass without creating a Job/Plan store before doctor.

The preview/review smoke is wired into macOS/Windows source verification and the
actual Windows portable launcher, including workflow path triggers. Strict
all-target core/CLI Clippy passed.

Browser playback verification is pending manual confirmation: the browser tool's
URL policy blocks `file://` access. No workaround was attempted. Automated media
and HTML checks establish document structure and byte/timing preservation; they
do not establish actual browser playback. The synthetic manual-check artifact is
outside the repository at `/private/tmp/forge-library-review-browser/preview/index.html`.
