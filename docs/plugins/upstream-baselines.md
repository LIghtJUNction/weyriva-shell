# Upstream compatibility baselines

Noctalia is reference material only. Pins make the clean-room research
reproducible and prevent a moving `main` branch from silently changing
Weyriva's compatibility claim.

## Public documentation

Pinned docs commit:

```text
b1e6e9b5235995ba6716d1814b4b127714d8f172
```

Primary sources:

- [v5 development overview](https://github.com/noctalia-dev/noctalia-docs/blob/b1e6e9b5235995ba6716d1814b4b127714d8f172/src/content/docs/v5/plugins/development/index.mdx)
- [v5 manifest and settings](https://github.com/noctalia-dev/noctalia-docs/blob/b1e6e9b5235995ba6716d1814b4b127714d8f172/src/content/docs/v5/plugins/development/manifest.mdx)
- [v5 entry scripts](https://github.com/noctalia-dev/noctalia-docs/blob/b1e6e9b5235995ba6716d1814b4b127714d8f172/src/content/docs/v5/plugins/development/entries.mdx)
- [v5 declarative UI](https://github.com/noctalia-dev/noctalia-docs/blob/b1e6e9b5235995ba6716d1814b4b127714d8f172/src/content/docs/v5/plugins/development/declarative-ui.mdx)
- [v5 runtime API](https://github.com/noctalia-dev/noctalia-docs/blob/b1e6e9b5235995ba6716d1814b4b127714d8f172/src/content/docs/v5/plugins/development/runtime-api.mdx)
- [v5 API ledger](https://github.com/noctalia-dev/noctalia-docs/blob/b1e6e9b5235995ba6716d1814b4b127714d8f172/src/data/plugin-api.ts)
- [v5 workflow and sources](https://github.com/noctalia-dev/noctalia-docs/blob/b1e6e9b5235995ba6716d1814b4b127714d8f172/src/content/docs/v5/plugins/development/workflow.mdx)
- [v4 overview](https://github.com/noctalia-dev/noctalia-docs/blob/b1e6e9b5235995ba6716d1814b4b127714d8f172/src/content/docs/v4/development/plugins/overview.mdx)
- [v4 manifest](https://github.com/noctalia-dev/noctalia-docs/blob/b1e6e9b5235995ba6716d1814b4b127714d8f172/src/content/docs/v4/development/plugins/manifest.mdx)
- [v4 Plugin API](https://github.com/noctalia-dev/noctalia-docs/blob/b1e6e9b5235995ba6716d1814b4b127714d8f172/src/content/docs/v4/development/plugins/api.mdx)
- [v4 IPC](https://github.com/noctalia-dev/noctalia-docs/blob/b1e6e9b5235995ba6716d1814b4b127714d8f172/src/content/docs/v4/development/plugins/ipc.mdx)

## Public plugin corpora

Official v5 commit:

```text
d8616f06f707ca6ba99526fb45e0b8fae672259a
```

Representative manifests:

- [example](https://github.com/noctalia-dev/official-plugins/blob/d8616f06f707ca6ba99526fb45e0b8fae672259a/example/plugin.toml)
- [timer](https://github.com/noctalia-dev/official-plugins/blob/d8616f06f707ca6ba99526fb45e0b8fae672259a/timer/plugin.toml)
- [screen recorder](https://github.com/noctalia-dev/official-plugins/blob/d8616f06f707ca6ba99526fb45e0b8fae672259a/screen_recorder/plugin.toml)

Community v5 commit:

```text
6cee9bbcc726c29e3c1190ae52c6e6135f6819ce
```

Representative manifests:

- [Upbeat](https://github.com/noctalia-dev/community-plugins/blob/6cee9bbcc726c29e3c1190ae52c6e6135f6819ce/upbeat/plugin.toml)
- [File Search](https://github.com/noctalia-dev/community-plugins/blob/6cee9bbcc726c29e3c1190ae52c6e6135f6819ce/file-search/plugin.toml)
- [ShareDND](https://github.com/noctalia-dev/community-plugins/blob/6cee9bbcc726c29e3c1190ae52c6e6135f6819ce/sharednd/plugin.toml)
- [Battery Graph](https://github.com/noctalia-dev/community-plugins/blob/6cee9bbcc726c29e3c1190ae52c6e6135f6819ce/battery-graph/plugin.toml)
- [Drive Health](https://github.com/noctalia-dev/community-plugins/blob/6cee9bbcc726c29e3c1190ae52c6e6135f6819ce/drive-health/plugin.toml)

Legacy v4 corpus commit:

```text
ea21cb63d063075bc0acd72d8b946ce2c5eef00d
```

- [legacy v4 tree](https://github.com/noctalia-dev/noctalia-plugins/tree/ea21cb63d063075bc0acd72d8b946ce2c5eef00d)

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
