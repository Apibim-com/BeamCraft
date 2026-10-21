# BeamCraft JSON

Bounded JSON values: the machine-readable output of the `beamcraft` CLI, which the native UI and
the local coordinator read back. The crate performs no I/O, process launch or filesystem access.
It is desktop-native and depends on `beamcraft-model` only for `ErrorCode`.

`decode` scans complete capped UTF-8 before it constructs a tree. It checks grammar, depth,
decoded strings, tokens, a conservative scratch reservation, numeric overflow and nonzero
underflow, escapes and surrogate pairs; tree construction rejects duplicate decoded keys.
`Json::encode` revalidates its output under the same limits. This is not an RFC 8785 (JCS)
canonicalizer.

`Limits::DEFAULT` bounds every CLI reply: 1 MiB, nesting depth 16, 65,536 tokens, 4 KiB per
decoded string and 8 MiB of parser scratch. A refusal is an `Error` (`Syntax`, `Limit` or
`Shape`); `Error::code` gives its F04 code (`MALFORMED_COMMAND` or `LIMIT_EXCEEDED`).

Integers such as workspace generations are carried as decimal strings, so no reader loses
precision.
