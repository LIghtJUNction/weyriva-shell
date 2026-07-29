# Weyriva plugin compatibility contract

Version: `1`
Status: specification implemented; runtime conformance in progress

This contract defines what Weyriva means by compatibility with public Noctalia
plugin formats. Weyriva implements the behavior in project-owned hosts. It does
not embed, launch, or delegate desktop ownership to a Noctalia shell.

## Profiles

| Profile | Package format | Execution environment | Status |
|---|---|---|---|
| `noctalia-v5-luau/1` | `plugin.toml` + `.luau` | isolated trusted Luau VMs in Weyriva | In progress |
| `noctalia-v4-qml/1` | `manifest.json` + QML | isolated Quickshell compatibility host | Planned |
| `weyriva-exec/1` | JSON + executable | bounded local JSON daemon | Implemented locally |

A host advertises an exact profile and supported API range. It must reject a
package outside that range rather than running it partially.

## Definition of compatibility

A plugin is compatible only when all applicable behavior passes:

1. package discovery and identity;
2. manifest validation;
3. entry loading and isolation;
4. lifecycle callbacks and teardown;
5. visible UI rendering and input;
6. settings defaults, scopes, mutation, and persistence;
7. cross-entry and persistent state;
8. IPC targeting, payload, result, and errors;
9. source precedence and compatible revision selection;
10. failure containment and actionable diagnostics.

The following are explicitly insufficient:

- a catalog card appears;
- the manifest parses;
- files download or copy successfully;
- entry names are listed;
- a static screenshot resembles the reference;
- a fixture runs through an installed Noctalia engine instead of Weyriva.

## Clean-room rule

Compatibility evidence may use:

- public documentation at a pinned commit;
- public plugin manifests and published packages;
- installed public CLI/tool output;
- self-authored black-box fixtures and observed behavior.

Noctalia shell implementation source must not be copied into Weyriva or treated
as the compatibility contract. Internal classes, cache layout, private call
ordering, and undocumented recovery behavior are implementation details.

## Trust and isolation

Compatible Luau, QML, and executable plugins are trusted user code, not a
security sandbox. Weyriva still isolates failure domains:

- one bad entry must not freeze the core UI;
- time and output are bounded where the public ABI permits;
- v4 QML runs outside the core shell process;
- secrets are not included in logs or catalog metadata;
- privileged operations are not granted implicitly.

The install UI presents source, author, version, requested API, declared
dependencies, and compatibility result before enablement.

## Versioning

The contract version describes Weyriva semantics. The upstream `plugin_api`
number describes capabilities required by one v5 plugin. They are separate.

A compatibility change requires:

- an updated public baseline;
- an entry in the API capability ledger;
- new or updated self-authored fixtures;
- a documented migration or explicit rejection;
- no silent widening of the advertised range.

## Status and evidence

Each profile records independently:

- parsed;
- loaded;
- rendered;
- interactive;
- lifecycle-complete;
- settings-complete;
- IPC-complete;
- failure-isolated;
- verified on XRY.

Only the last successful level may be claimed. See
[Conformance fixtures](conformance-fixtures.md) and
[Known gaps](known-gaps.md).
