# Upstream compatibility baselines

Noctalia is reference material only. Pins make the clean-room research
reproducible and prevent a moving `main` branch from silently changing
Weyriva's compatibility claim.

## Public documentation

Pinned docs commit:

```text
a0fcbcafc709836f46e1c23b18ade6947d442e26
```

Primary sources:

- [v5 development overview](https://github.com/noctalia-dev/noctalia-docs/blob/a0fcbcafc709836f46e1c23b18ade6947d442e26/src/content/docs/v5/plugins/development/index.mdx)
- [v5 manifest and settings](https://github.com/noctalia-dev/noctalia-docs/blob/a0fcbcafc709836f46e1c23b18ade6947d442e26/src/content/docs/v5/plugins/development/manifest.mdx)
- [v5 entry scripts](https://github.com/noctalia-dev/noctalia-docs/blob/a0fcbcafc709836f46e1c23b18ade6947d442e26/src/content/docs/v5/plugins/development/entries.mdx)
- [v5 declarative UI](https://github.com/noctalia-dev/noctalia-docs/blob/a0fcbcafc709836f46e1c23b18ade6947d442e26/src/content/docs/v5/plugins/development/declarative-ui.mdx)
- [v5 runtime API](https://github.com/noctalia-dev/noctalia-docs/blob/a0fcbcafc709836f46e1c23b18ade6947d442e26/src/content/docs/v5/plugins/development/runtime-api.mdx)
- [v5 API ledger](https://github.com/noctalia-dev/noctalia-docs/blob/a0fcbcafc709836f46e1c23b18ade6947d442e26/src/data/plugin-api.ts)
- [v5 workflow and sources](https://github.com/noctalia-dev/noctalia-docs/blob/a0fcbcafc709836f46e1c23b18ade6947d442e26/src/content/docs/v5/plugins/development/workflow.mdx)
- [v4 overview](https://github.com/noctalia-dev/noctalia-docs/blob/a0fcbcafc709836f46e1c23b18ade6947d442e26/src/content/docs/v4/development/plugins/overview.mdx)
- [v4 manifest](https://github.com/noctalia-dev/noctalia-docs/blob/a0fcbcafc709836f46e1c23b18ade6947d442e26/src/content/docs/v4/development/plugins/manifest.mdx)
- [v4 Plugin API](https://github.com/noctalia-dev/noctalia-docs/blob/a0fcbcafc709836f46e1c23b18ade6947d442e26/src/content/docs/v4/development/plugins/api.mdx)
- [v4 IPC](https://github.com/noctalia-dev/noctalia-docs/blob/a0fcbcafc709836f46e1c23b18ade6947d442e26/src/content/docs/v4/development/plugins/ipc.mdx)

The corresponding shell runtime pins are:

```text
main: 2edf8c003cec37b5622a8f6bb9d511b6cfa5cf49 (plugin API 3..19)
v5.0.0-beta.6: d24fe45e9a798317072547fa5d56950607750e68 (plugin API 3..16)
```

The runtime API authority is
[`src/scripting/plugin_api.h`](https://github.com/noctalia-dev/noctalia/blob/2edf8c003cec37b5622a8f6bb9d511b6cfa5cf49/src/scripting/plugin_api.h).

## Public plugin corpora

Official v5 commit:

```text
4b03f0a5e3b701c5a3ade87d35ed62c1699f93c6
```

This snapshot contains 11 catalog plugins. Representative manifests:

- [Kaomoji](https://github.com/noctalia-dev/official-plugins/blob/4b03f0a5e3b701c5a3ade87d35ed62c1699f93c6/kaomoji/plugin.toml)
- [example](https://github.com/noctalia-dev/official-plugins/blob/4b03f0a5e3b701c5a3ade87d35ed62c1699f93c6/example/plugin.toml)
- [timer](https://github.com/noctalia-dev/official-plugins/blob/4b03f0a5e3b701c5a3ade87d35ed62c1699f93c6/timer/plugin.toml)
- [screen recorder](https://github.com/noctalia-dev/official-plugins/blob/4b03f0a5e3b701c5a3ade87d35ed62c1699f93c6/screen_recorder/plugin.toml)

Community v5 commit:

```text
35afaa444de6389164360b1ecadb87c972b32912
```

This snapshot contains 51 catalog plugins. Representative manifests:

- [Upbeat](https://github.com/noctalia-dev/community-plugins/blob/35afaa444de6389164360b1ecadb87c972b32912/upbeat/plugin.toml)
- [File Search](https://github.com/noctalia-dev/community-plugins/blob/35afaa444de6389164360b1ecadb87c972b32912/file-search/plugin.toml)
- [ShareDND](https://github.com/noctalia-dev/community-plugins/blob/35afaa444de6389164360b1ecadb87c972b32912/sharednd/plugin.toml)
- [Battery Graph](https://github.com/noctalia-dev/community-plugins/blob/35afaa444de6389164360b1ecadb87c972b32912/battery-graph/plugin.toml)
- [Drive Health](https://github.com/noctalia-dev/community-plugins/blob/35afaa444de6389164360b1ecadb87c972b32912/drive-health/plugin.toml)

Legacy v4 corpus commit:

```text
ea21cb63d063075bc0acd72d8b946ce2c5eef00d
```

- [legacy v4 tree](https://github.com/noctalia-dev/legacy-v4-plugins/tree/ea21cb63d063075bc0acd72d8b946ce2c5eef00d)

This snapshot contains 132 manifests.

## Installed-tool observation

During the 2026-07-29 research pass, the available public CLI reported:

```text
noctalia v5.0.0 (v5.0.0-beta.6-45-g83cac1e01c85)
```

Its offline `plugins lint` command documented checks for undeclared
`getConfig()` reads, obsolete aliases, unused settings, and missing entry
files. This observation helps design fixtures; it is not a Weyriva runtime
dependency and does not widen the supported API range.

## Updating a baseline

A baseline update requires:

1. pin the new commit;
2. diff public ABI documents and manifests;
3. classify facts versus implementation details;
4. update capability and unknown-gap ledgers;
5. add or update self-authored fixtures;
6. pass independent review before changing compatibility claims.
