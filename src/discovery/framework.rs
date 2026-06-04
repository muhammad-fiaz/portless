//! Framework and language ecosystem detection.
//!
//! Portless supports any language or runtime. This module provides
//! best-effort detection so the CLI can surface helpful suggestions and apply
//! framework-specific port/host flags automatically.
//!
//! Supported ecosystems:
//! - **JavaScript / TypeScript (Node.js)**: Next.js, Vite, Astro, Nuxt,
//!   SvelteKit, Remix, React Router, Solid, Angular, Express, Hono, Expo,
//!   React Native, Rsbuild, VitePlus
//! - **Deno**: detected via `deno.json` / `deno.lock`
//! - **Rust**: detected via `Cargo.toml`
//! - **Go**: detected via `go.mod`
//! - **Python**: detected via `pyproject.toml`, `setup.py`, `setup.cfg`,
//!   `requirements.txt`, or `Pipfile`
//! - **Ruby**: detected via `Gemfile`
//! - **PHP**: detected via `composer.json`
//! - **Java / Kotlin**: detected via `pom.xml`, `build.gradle`, or `gradlew`
//! - **Elixir**: detected via `mix.exs`
//! - **Dart / Flutter**: detected via `pubspec.yaml`
//! - **C# / .NET**: detected via `*.csproj` or `*.sln`
//! - **Zig**: detected via `build.zig`

use serde::{Deserialize, Serialize};
use std::path::Path;

/// A detected framework or language ecosystem.
///
/// Framework detection is used to:
/// 1. Display helpful framework names in `portless list`.
/// 2. Auto-inject `--port` / `--host` CLI flags for frameworks that require
///    them (e.g. Vite needs `--port <n> --host 127.0.0.1`).
/// 3. Set the `PORT` environment variable for frameworks that read it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Framework {
    /// Next.js — the React full-stack framework.
    Next,
    /// Vite — lightning-fast frontend build tool.
    Vite,
    /// VitePlus (`vp`) — extended Vite wrapper.
    VitePlus,
    /// Astro — content-focused static/SSR site builder.
    Astro,
    /// Nuxt — Vue full-stack framework.
    Nuxt,
    /// SvelteKit — Svelte full-stack framework.
    SvelteKit,
    /// Remix — full-stack web framework.
    Remix,
    /// React Router — data-driven routing with SSR.
    ReactRouter,
    /// SolidJS / SolidStart.
    Solid,
    /// Angular — Google's TypeScript platform.
    Angular,
    /// Express — minimal Node.js HTTP server.
    Express,
    /// Hono — fast, lightweight web framework.
    Hono,
    /// Expo — React Native development platform.
    Expo,
    /// React Native CLI.
    ReactNative,
    /// Rsbuild — Rspack-powered build tool.
    Rsbuild,
    /// Deno — secure TypeScript/JavaScript runtime.
    Deno,
    /// Rust / Cargo project.
    Cargo,
    /// Go module project.
    Go,
    /// Python project (any build system).
    Python,
    /// Ruby / Bundler project.
    Ruby,
    /// PHP / Composer project.
    Php,
    /// Java or Kotlin — Maven or Gradle.
    Jvm,
    /// Elixir / Mix project.
    Elixir,
    /// Dart / Flutter.
    Flutter,
    /// C# / .NET.
    DotNet,
    /// Zig.
    Zig,
    /// A project detected by `package.json` whose framework is not
    /// specifically recognised.
    Unknown,
}

impl Framework {
    /// Detect the framework or language ecosystem for the given project
    /// directory.
    ///
    /// Returns `None` only when no recognisable project files are found at
    /// all (e.g. an empty directory). Returns [`Framework::Unknown`] when a
    /// `package.json` is present but no known framework dependency was found.
    pub async fn detect(dir: &Path) -> Option<Self> {
        async fn exists(p: &Path) -> bool {
            tokio::fs::try_exists(p).await.unwrap_or(false)
        }

        if exists(&dir.join("next.config.js")).await
            || exists(&dir.join("next.config.mjs")).await
            || exists(&dir.join("next.config.ts")).await
        {
            return Some(Self::Next);
        }
        if exists(&dir.join("vite.config.ts")).await
            || exists(&dir.join("vite.config.js")).await
            || exists(&dir.join("vite.config.mjs")).await
        {
            return Some(Self::Vite);
        }
        if exists(&dir.join("astro.config.mjs")).await
            || exists(&dir.join("astro.config.js")).await
            || exists(&dir.join("astro.config.ts")).await
        {
            return Some(Self::Astro);
        }
        if exists(&dir.join("nuxt.config.ts")).await || exists(&dir.join("nuxt.config.js")).await {
            return Some(Self::Nuxt);
        }
        if exists(&dir.join("svelte.config.js")).await {
            return Some(Self::SvelteKit);
        }
        if exists(&dir.join("remix.config.js")).await {
            return Some(Self::Remix);
        }
        if exists(&dir.join("react-router.config.ts")).await
            || exists(&dir.join("react-router.config.js")).await
        {
            return Some(Self::ReactRouter);
        }
        if exists(&dir.join("angular.json")).await {
            return Some(Self::Angular);
        }

        if exists(&dir.join("deno.json")).await
            || exists(&dir.join("deno.jsonc")).await
            || exists(&dir.join("deno.lock")).await
        {
            return Some(Self::Deno);
        }

        if exists(&dir.join("Cargo.toml")).await {
            return Some(Self::Cargo);
        }

        if exists(&dir.join("go.mod")).await {
            return Some(Self::Go);
        }

        if exists(&dir.join("pyproject.toml")).await
            || exists(&dir.join("setup.py")).await
            || exists(&dir.join("setup.cfg")).await
            || exists(&dir.join("requirements.txt")).await
            || exists(&dir.join("Pipfile")).await
            || exists(&dir.join("uv.lock")).await
            || exists(&dir.join("poetry.lock")).await
        {
            return Some(Self::Python);
        }

        if exists(&dir.join("Gemfile")).await {
            return Some(Self::Ruby);
        }

        if exists(&dir.join("composer.json")).await {
            return Some(Self::Php);
        }

        if exists(&dir.join("pom.xml")).await
            || exists(&dir.join("build.gradle")).await
            || exists(&dir.join("build.gradle.kts")).await
            || exists(&dir.join("gradlew")).await
        {
            return Some(Self::Jvm);
        }

        if exists(&dir.join("mix.exs")).await {
            return Some(Self::Elixir);
        }

        if exists(&dir.join("pubspec.yaml")).await {
            return Some(Self::Flutter);
        }

        // Checking for any .csproj or .sln file requires a directory scan.
        if let Ok(mut rd) = tokio::fs::read_dir(dir).await {
            while let Ok(Some(entry)) = rd.next_entry().await {
                let name = entry.file_name();
                let s = name.to_string_lossy();
                if s.ends_with(".csproj") || s.ends_with(".sln") || s.ends_with(".fsproj") {
                    return Some(Self::DotNet);
                }
            }
        }

        if exists(&dir.join("build.zig")).await {
            return Some(Self::Zig);
        }

        if let Ok(bytes) = tokio::fs::read(dir.join("package.json")).await
            && let Ok(parsed) = serde_json::from_slice::<serde_json::Value>(&bytes)
        {
            let deps = parsed
                .get("dependencies")
                .and_then(|v| v.as_object())
                .cloned()
                .unwrap_or_default();
            let dev = parsed
                .get("devDependencies")
                .and_then(|v| v.as_object())
                .cloned()
                .unwrap_or_default();
            let has = |name: &str| deps.contains_key(name) || dev.contains_key(name);

            if has("next") {
                return Some(Self::Next);
            }
            if has("vite-plus") {
                return Some(Self::VitePlus);
            }
            if has("vite") {
                return Some(Self::Vite);
            }
            if has("astro") {
                return Some(Self::Astro);
            }
            if has("nuxt") {
                return Some(Self::Nuxt);
            }
            if has("@sveltejs/kit") {
                return Some(Self::SvelteKit);
            }
            if has("@remix-run/react") {
                return Some(Self::Remix);
            }
            if has("@react-router/dev") {
                return Some(Self::ReactRouter);
            }
            if has("solid-js") {
                return Some(Self::Solid);
            }
            if has("@angular/core") {
                return Some(Self::Angular);
            }
            if has("express") {
                return Some(Self::Express);
            }
            if has("hono") {
                return Some(Self::Hono);
            }
            if has("expo") {
                return Some(Self::Expo);
            }
            if has("react-native") {
                return Some(Self::ReactNative);
            }
            if has("@rsbuild/core") {
                return Some(Self::Rsbuild);
            }
            // Any other package.json project.
            return Some(Self::Unknown);
        }

        None
    }

    /// Returns `true` when this ecosystem's dev server automatically reads
    /// the `PORT` environment variable and binds to it.
    ///
    /// When `false`, portless injects an explicit `--port <n>` flag (see
    /// [`Self::port_flags`]).
    pub fn respects_port_env(self) -> bool {
        matches!(
            self,
            Self::Next
                | Self::Nuxt
                | Self::SvelteKit
                | Self::Remix
                | Self::ReactRouter
                | Self::Solid
                | Self::Express
                | Self::Hono
                | Self::Unknown
                | Self::Go
                | Self::Python
                | Self::Ruby
                | Self::Php
                | Self::Jvm
                | Self::Elixir
                | Self::Cargo
                | Self::Deno
                | Self::Flutter
                | Self::DotNet
                | Self::Zig
        )
    }

    /// Extra CLI flags to inject so the dev server binds to the right port.
    ///
    /// Portless appends these flags *before* the child process is spawned
    /// when it allocates a dynamic port. The `PORT` environment variable is
    /// always set regardless.
    pub fn port_flags(self) -> &'static [&'static str] {
        match self {
            Self::Vite
            | Self::VitePlus
            | Self::ReactRouter
            | Self::Rsbuild
            | Self::Astro
            | Self::Angular
            | Self::Expo
            | Self::ReactNative => &["--port"],
            _ => &[],
        }
    }

    /// Extra CLI flags to inject for the host-binding address.
    ///
    /// Some frameworks default to binding only `localhost`; these flags make
    /// them listen on `127.0.0.1` so the portless proxy can reach them.
    pub fn host_flags(self) -> &'static [&'static str] {
        match self {
            Self::Vite
            | Self::VitePlus
            | Self::Astro
            | Self::ReactRouter
            | Self::Rsbuild
            | Self::ReactNative
            | Self::Angular
            | Self::Expo => &["--host"],
            _ => &[],
        }
    }

    /// Whether this framework's dev server supports `--strictPort`.
    pub fn strict_port(self) -> bool {
        matches!(self, Self::Vite | Self::VitePlus | Self::ReactRouter)
    }

    /// Returns the variant matching a given CLI command basename, if any.
    pub fn from_basename(name: &str) -> Option<Self> {
        match name {
            "next" => Some(Self::Next),
            "vite" => Some(Self::Vite),
            "vp" => Some(Self::VitePlus),
            "astro" => Some(Self::Astro),
            "nuxt" => Some(Self::Nuxt),
            "svelte-kit" | "sveltekit" => Some(Self::SvelteKit),
            "remix" => Some(Self::Remix),
            "react-router" => Some(Self::ReactRouter),
            "solid" | "solid-start" | "solidstart" => Some(Self::Solid),
            "ng" => Some(Self::Angular),
            "express" => Some(Self::Express),
            "hono" => Some(Self::Hono),
            "expo" => Some(Self::Expo),
            "react-native" => Some(Self::ReactNative),
            "rsbuild" => Some(Self::Rsbuild),
            _ => None,
        }
    }

    /// Human-readable display name for use in CLI output and the dashboard.
    pub fn name(self) -> &'static str {
        match self {
            Self::Next => "Next.js",
            Self::Vite => "Vite",
            Self::VitePlus => "VitePlus",
            Self::Astro => "Astro",
            Self::Nuxt => "Nuxt",
            Self::SvelteKit => "SvelteKit",
            Self::Remix => "Remix",
            Self::ReactRouter => "React Router",
            Self::Solid => "Solid",
            Self::Angular => "Angular",
            Self::Express => "Express",
            Self::Hono => "Hono",
            Self::Expo => "Expo",
            Self::ReactNative => "React Native",
            Self::Rsbuild => "Rsbuild",
            Self::Deno => "Deno",
            Self::Cargo => "Rust/Cargo",
            Self::Go => "Go",
            Self::Python => "Python",
            Self::Ruby => "Ruby",
            Self::Php => "PHP",
            Self::Jvm => "Java/Kotlin",
            Self::Elixir => "Elixir",
            Self::Flutter => "Dart/Flutter",
            Self::DotNet => ".NET",
            Self::Zig => "Zig",
            Self::Unknown => "Node.js",
        }
    }

    /// Whether this ecosystem is JavaScript/TypeScript based.
    pub fn is_js(self) -> bool {
        matches!(
            self,
            Self::Next
                | Self::Vite
                | Self::VitePlus
                | Self::Astro
                | Self::Nuxt
                | Self::SvelteKit
                | Self::Remix
                | Self::ReactRouter
                | Self::Solid
                | Self::Angular
                | Self::Express
                | Self::Hono
                | Self::Expo
                | Self::ReactNative
                | Self::Rsbuild
                | Self::Deno
                | Self::Unknown
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn detect_via_config() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("next.config.js"), "module.exports = {}").unwrap();
        let f = Framework::detect(dir.path()).await;
        assert_eq!(f, Some(Framework::Next));
    }

    #[tokio::test]
    async fn detect_via_dep() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"dependencies": {"vite": "^5"}}"#,
        )
        .unwrap();
        let f = Framework::detect(dir.path()).await;
        assert_eq!(f, Some(Framework::Vite));
    }

    #[tokio::test]
    async fn detect_cargo() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        let f = Framework::detect(dir.path()).await;
        assert_eq!(f, Some(Framework::Cargo));
    }

    #[tokio::test]
    async fn detect_go() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module example.com/myapp\n\ngo 1.22\n",
        )
        .unwrap();
        let f = Framework::detect(dir.path()).await;
        assert_eq!(f, Some(Framework::Go));
    }

    #[tokio::test]
    async fn detect_python_pyproject() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("pyproject.toml"),
            "[project]\nname = \"myapp\"\n",
        )
        .unwrap();
        let f = Framework::detect(dir.path()).await;
        assert_eq!(f, Some(Framework::Python));
    }

    #[tokio::test]
    async fn detect_python_requirements() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("requirements.txt"), "flask\n").unwrap();
        let f = Framework::detect(dir.path()).await;
        assert_eq!(f, Some(Framework::Python));
    }

    #[tokio::test]
    async fn detect_deno() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("deno.json"), "{}").unwrap();
        let f = Framework::detect(dir.path()).await;
        assert_eq!(f, Some(Framework::Deno));
    }

    #[tokio::test]
    async fn detect_ruby() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("Gemfile"),
            "source 'https://rubygems.org'\n",
        )
        .unwrap();
        let f = Framework::detect(dir.path()).await;
        assert_eq!(f, Some(Framework::Ruby));
    }

    #[tokio::test]
    async fn empty_dir_returns_none() {
        let dir = TempDir::new().unwrap();
        let f = Framework::detect(dir.path()).await;
        assert_eq!(f, None);
    }
}
