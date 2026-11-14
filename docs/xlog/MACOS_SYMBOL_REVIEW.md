# Bundled Hamlib system imports: macOS review

This tooling-only review adds 58 strong libSystem imports used by the bundled
Hamlib library/tools to the existing allowlist. It changes no application code
or bundle bytes. The reviewed fork floor is macOS 14.0; this review is not a
claim of runtime qualification on every macOS 14 machine.

The frozen arm64 `1.24.1-xlog.1` build from
`5e56be94c7ede2099d5b8ed6d1a4d0d1ad12134b` compiled successfully in 86m30s.
Its initial bundle audit failed only because these imports were absent from
the upstream list, which originally described the main executable. That failed
receipt remains unchanged. A separate reviewed allowlist then passed the same
frozen checker against the exact built bundle. All seven Mach-O files have the
expected architecture and macOS 14 deployment target and load only bundled or
Apple libraries. The four Hamlib tools resolve their adjacent libraries through
`@loader_path`; their `--version` invocations passed without opening hardware.

## Apple evidence

[Apple's macOS 14.0 release manifest](https://github.com/apple-oss-distributions/distribution-macOS/blob/macos-140/release.json)
selects the following primary source tags:

| Project | Release tag | Reviewed uses |
| --- | --- | --- |
| [Libc](https://github.com/apple-oss-distributions/Libc/tree/Libc-1583.0.14) | `Libc-1583.0.14` | Standard C/POSIX declarations, secure string implementations, stdio aliases |
| [Libinfo](https://github.com/apple-oss-distributions/Libinfo/tree/Libinfo-583.0.1) | `Libinfo-583.0.1` | `getnameinfo` implementation |
| [libpthread](https://github.com/apple-oss-distributions/libpthread/tree/libpthread-519) | `libpthread-519` | Attribute, cancellation and exit implementations |
| [xnu](https://github.com/apple-oss-distributions/xnu/tree/xnu-10002.1.13) | `xnu-10002.1.13` | Mach clock, exact arm64 select/socketpair aliases and Darwin alias macros |
| [Libsystem](https://github.com/apple-oss-distributions/Libsystem/tree/Libsystem-1336) | `Libsystem-1336` | Required component libraries |

The checked Apple SDK declaration for `__darwin_check_fd_set_overflow` explicitly
marks availability since macOS 11.0. Historical `stdio.h` declares the exact
`fopen`/`fdopen` Darwin-extension aliases beginning with macOS 10.6; xnu's arm64
syscall map supplies the exact `select$DARWIN_EXTSN` and `socketpair` symbols.
The two secure-string symbols have implementations in the release-pinned Libc.

The [per-symbol evidence index](evidence/macos-hamlib-imports-20261114.json)
records all 58 names, source file hashes/lines, project trees/archive hashes and
release-manifest hash. Full downloaded Apple sources, the SDK declaration,
review helper and logs are retained in the Mac supplemental evidence directory.
A generated, non-executed arm64 link probe using Apple's headers passed with
`-mmacosx-version-min=14.0`, `-Werror=unguarded-availability` and
`-Werror=unguarded-availability-new`. That probe complements the historical
source review; linking against the newer SDK alone would not prove availability
on the older operating system.

The unchanged Mach-O checker test suite passed all 33 cases on the Mac using
the updated sorted allowlist. The Windows run also exposed existing POSIX-path
assertions; it is not reported as a passing cross-platform suite.

## Exact-byte baseline artifact

Evidence root on the build Mac:

`/Users/oliver/Projects/OM0RX-xNexus/target/xnexus-provenance/macos/1.24.1-xlog.1-20261114T124521Z-8160`

Original `provenance.json` SHA-256:
`234c47583949118c6ed501286ed70a2aac68932e553a359e2dbae47bc78d9870`.
Supplemental qualification is recorded separately in `supplemental/receipt.json`;
`supplemental/evidence-catalog.json` SHA-256 is
`4621fd80e4fa7f80a214841e0821017206d3937c05f4022d232e5fdbd702219f`.

The unchanged bundle was archived as
`OM0RX-xNexus_1.24.1-xlog.1_arm64_UNSIGNED.app.tar.gz`, 56,583,180 bytes,
SHA-256 `ab8fb543ba702bdf3ca5fc75673945d49a62b6d9fa7a391dab317d42a509b2af`.
Before/after bundle manifests match, and every archive entry's mode/content or
symlink target was checked against the original. Model and resource/license
hashes match the build provenance. Both actual CFBundle version fields retain
`1.24.1-xlog.1`; the built UI footer retains that version and source `5e56be94c`.

The executable has only a linker ad-hoc signature; bundle resources are unsealed.
There is no Developer ID signature or notarization. No GUI launch, installation,
public upload, macOS 14 runtime test or physical-station acceptance was performed.
The newer Homebrew build dependencies remain explicitly recorded in provenance,
so static checks do not become an unsupported broad compatibility claim.
