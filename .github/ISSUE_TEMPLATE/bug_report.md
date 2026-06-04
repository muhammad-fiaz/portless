---
name: Bug report
about: Create a report to help us improve
title: "[bug] "
labels: ["bug", "triage"]
assignees: []
---

## Bug Description

A clear and concise description of what the bug is.

## Reproduction Steps

Steps to reproduce the behavior:

1. `portless ...`
2. `...`
3. See error

## Expected Behavior

A clear and concise description of what you expected to happen.

## Actual Behavior

What actually happened. Include any error output, stack traces, or screenshots.

```
$ portless ...
error: ...
```

## Environment

- **OS**: [e.g. macOS 14.4, Ubuntu 24.04, Windows 11]
- **Rust version** (`rustc --version`): [e.g. rustc 1.85.0]
- **Portless version** (`portless --version`): [e.g. portless 0.0.0]
- **Portless install method**: [e.g. `cargo install`, source build, pre-built binary]
- **Framework / dev server** (if applicable): [e.g. Next.js 14.3, Vite 5.0]

## Configuration

<details>
<summary>portless.json (if any)</summary>

```json
{
  "name": "..."
}
```
</details>

<details>
<summary>package.json scripts (if relevant)</summary>

```json
{
  "scripts": {
    "dev": "..."
  }
}
```
</details>

## Logs

<details>
<summary>Proxy log (`~/.portless/proxy.log`)</summary>

```
paste here
```
</details>

<details>
<summary>Child log (`~/.portless/logs/<hostname>.log`)</summary>

```
paste here
```
</details>

## Additional Context

Add any other context about the problem here - links, screenshots, related issues.

## Possible Solution

If you have a suggestion for how to fix the bug, please describe it here.

## Willingness to Contribute

- [ ] I am willing to submit a PR to fix this bug
- [ ] I would like to discuss the approach first
