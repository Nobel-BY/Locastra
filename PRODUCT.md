# Product

<!-- impeccable:product-schema 1 -->

## Platform

windows

## Users

Windows 10/11 x64 users in mainland China who want to run local language models without learning command-line tools, model formats, quantization, or inference flags.

## Product Purpose

Turn local AI setup into one understandable desktop workflow: inspect the computer, find a compatible GGUF model, download it reliably, load it with safe defaults, and chat in Chinese. Success means a new user can reach an offline conversation without using a terminal.

## Positioning

An offline-first Windows model workspace that combines mainland-friendly model delivery, honest hardware-fit guidance, and one-click llama.cpp runtime management in one native desktop experience.

## Operating Context

Users search ModelScope, Hugging Face, and configured mirrors; compare quantizations and hardware fit; manage large resumable downloads; import local GGUF files; load one model; then work in persistent local conversations. Long downloads, limited disk space, VPN or proxy interference, and models larger than system memory are normal operating conditions.

## Capabilities and Constraints

- Windows 10/11 x64 desktop application built with Tauri 2, React, TypeScript, and Rust.
- Public GGUF text-chat models only in the first release.
- ModelScope-first discovery with Hugging Face, hf-mirror.com, and verified cross-source fallback.
- Local SQLite settings, model metadata, and conversations; no account required.
- llama.cpp CPU, Vulkan, and CUDA runtimes; one model loaded at a time.
- Existing information architecture and working flows must remain functional during the redesign.
- Simplified Chinese is the primary interface language. English is used for the formal product brand and established technical terms.

## Brand Commitments

- The Chinese descriptor “本地智聊” remains available for continuity, but the product receives a formal English brand name.
- The product should feel trustworthy, calm, precise, and approachable, expressed as a premium local-compute instrument rather than a playful consumer chatbot.
- Privacy, local ownership, and transparent runtime state are central product promises.
- Avoid generic AI-purple gradients, emoji icons, and oversized marketing layouts inside operational screens. Translucency is reserved for the fixed shell and modal layers where it communicates depth.
- Offer two deliberately composed local themes: the existing continuous dark neural-console appearance and a Lunar White light appearance. Keep selection explicit and local; do not follow the Windows theme automatically.
- The Windows build must suppress browser context menus, browser navigation commands, printing, page saving, and developer tools.

## Evidence on Hand

- Existing working Tauri application and complete feature implementation in `src/App.tsx`.
- Existing visual implementation and component states in `src/styles.css`.
- Hardware detection, resilient download manager, model library, chat, assistant workspace, comparison, MCP tools, and settings are implemented.
- No customer testimonials, usage metrics, or commercial claims are available and none should be invented.

## Product Principles

1. Make the next action obvious without hiding important technical truth.
2. Explain model fit and failures in plain Chinese.
3. Keep local state and privacy visible, not merely promised.
4. Favor reliable progress and recovery over decorative novelty.
5. Preserve expert controls through progressive disclosure.

## Accessibility & Inclusion

Target WCAG 2.2 AA contrast, visible keyboard focus, 40px or larger primary hit areas, reduced-motion support, and layouts that remain usable at the minimum 920×620 Windows size.
