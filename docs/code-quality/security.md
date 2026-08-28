# Security and trust boundaries

> Load when: handling media or project files, filesystem paths, downloads, updates, dependencies,
> FFI, secrets, external processes, network data, or user-controlled rendered content.
> Skip when: the change does not cross a trust boundary.

## Untrusted input

- Treat media files, `.ocproj` files, model output, update metadata, filenames, and network responses
  as untrusted.
- Validate structure, ranges, sizes, counts, durations, dimensions, and enum values before allocation
  or native calls. Reject impossible or excessive input with an actionable error.
- Put parser and decoder work behind bounded memory, time, recursion, and concurrency limits where the
  underlying format can amplify small input.
- Do not render untrusted strings as markup or commands. Keep UI text and shell/process arguments as
  data through structured APIs.

## Files and paths

- Resolve the intended base directory and prevent traversal when extracting or materializing files.
- Use unique, permission-appropriate temporary paths; avoid predictable shared filenames.
- Do not overwrite user media or projects as an intermediate step. Use atomic replacement for durable
  project and update state when supported.
- Treat symlinks and aliases deliberately when a path is used for deletion, replacement, or trust
  decisions.

## Native and external boundaries

- Validate lengths, integer conversions, null pointers, buffer capacity, ownership, and callback
  lifetime before crossing FFI.
- Keep unsafe wrappers small and ensure every native resource has one clear release path.
- Never interpolate untrusted data into a shell command. Prefer library APIs or argument arrays.
- Verify downloaded installers, updates, models, and bundled binaries using authenticated transport
  plus the project's trusted signature or digest mechanism before execution or replacement.

## Secrets, logs, and diagnostics

- Do not commit credentials, tokens, private endpoints, or signing material.
- Do not put secrets, full user paths, project contents, or sensitive media metadata into routine
  logs. Redact diagnostics before telemetry or issue reports.
- Errors shown to users should be useful without exposing internal secrets or unsafe recovery steps.

## Dependencies

- Prefer existing dependencies. For additions, verify source, maintenance, license, required features,
  transitive surface, and platform support.
- Pin and verify artifacts according to the repository's packaging/update design. Do not disable
  certificate, signature, or digest verification to make a build pass.
- Keep parsers, codecs, network clients, and native libraries updated with focused compatibility
  testing because they process hostile input at complex boundaries.

## Approval blockers

- Unbounded allocation or work derived from untrusted metadata.
- Path traversal, unsafe overwrite, or deletion through unresolved user-controlled paths.
- Command construction by string interpolation.
- Missing authenticity verification before running downloaded code.
- An FFI contract whose pointer, length, lifetime, or ownership assumptions are not enforced.
- Secret or sensitive user data exposed through source, logs, UI, or telemetry.
