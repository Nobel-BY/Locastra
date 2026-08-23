# Security policy

## Supported version

Security fixes currently target the latest public Locastra release.

## Reporting a vulnerability

Please do not publish exploitable details in a public issue. Use GitHub's **Security → Report a vulnerability** flow for the repository. Include the affected version, reproduction steps, expected impact, and any suggested mitigation.

## Security boundary

- Locastra treats downloaded model repositories as untrusted data and only loads validated GGUF files.
- Repository scripts and dynamic libraries are never executed.
- `llama-server` listens on `127.0.0.1`; the optional developer API can be protected with a token.
- Diagnostic exports exclude chat content by default.
- Models themselves can still produce unsafe or incorrect output. Review generated commands and files before using them.

Unsigned public-test installers can trigger Windows SmartScreen. Verify the SHA-256 shown on the GitHub Release page. Production update packages require a valid Authenticode signature.
