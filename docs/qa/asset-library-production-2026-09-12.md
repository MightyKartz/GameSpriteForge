# Output registration and build reuse verification

PR 3 binds local static, audio, animation, character and direct layered outputs
to an explicit V3 resource project. Existing generated output publications now
retain recovery requests alongside Packs. Recovery validates bytes and registers
without invoking execution or Provider dispatch.

Synthetic validation passed: 13 library tests, 4 audio plan tests, 14 automation
tests, 10 game-art build tests, 10 layered Pack tests, 8 static preparation tests,
and 137 core unit tests. Bound audio exercised installed FFmpeg. Failure injection
corrupted an existing immutable record: the next Job retained its valid Pack and
publication request, reported a recoverable registration error, and registration
succeeded after restoring the object, without another Job. A pending request
refused changed Pack bytes. Static, audio, both animation paths and direct layered
publication preserve source distinctions and do not choose a delivery revision.

The V3 build fixture changes A to B and returns to A, reusing the original A Packs
and dependency revisions with zero additional Provider requests and a byte-identical
catalog head. Legacy build fixtures also passed. Private projects were not modified.
