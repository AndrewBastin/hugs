# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in this project, please report it by emailing naqerjonfgva.x@tznvy.pbz (deciper the ROT13 cipher).

Please include the following information in your report:

- A description of the vulnerability
- Steps to reproduce the issue
- Potential impact of the vulnerability
- Any suggested fixes (optional)

## Response Timeline

You can expect an initial response within **7 days** of your report. We will work with you to understand and address the issue.

## Scope

The following areas are considered in scope for security reports:

- **Source code**: Vulnerabilities in the project's codebase (Rust, templates, build logic)
- **Dependencies**: Security issues in third-party libraries used by the project
- **Development server**: Issues affecting the `hugs dev` localhost server
- **Build process**: Vulnerabilities in the static site generation pipeline
- **CLI**: Command injection or path traversal in CLI argument handling

We accept reports of any severity level.

## Out of Scope

The following are **not** considered valid security reports:

### Intentional Design Decisions

- **Dangerous HTML in markdown**: The markdown parser intentionally allows raw HTML, `<script>` tags, and event handlers. This is by design to support advanced use cases. Users are expected to control their own content.
- **Dangerous protocols in links**: Support for `javascript:`, `data:`, and other protocols in markdown links is intentional.
- **Template injection via frontmatter**: MiniJinja template variables controlled by frontmatter are user-authored content. Issues arising from users' own template/frontmatter combinations are not vulnerabilities.

### General Exclusions

- Vulnerabilities in applications or services not maintained by this project
- Social engineering attacks
- Denial of service attacks
- Issues requiring physical access to the machine
- Vulnerabilities in the generated static site output caused by user-authored content
- Security issues in sites built with Hugs (those are the site author's responsibility)

## Disclosure

We handle disclosure on a case-by-case basis and will work with reporters to determine an appropriate timeline for public disclosure.

## Recognition

We appreciate the efforts of security researchers who help keep this project safe. With your permission, we will acknowledge your contribution when the vulnerability is disclosed.
