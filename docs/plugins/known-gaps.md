# Plugin compatibility known gaps

These gaps are not answered by public manifests alone. They require a
self-authored behavior probe against the pinned reference and then a matching
Weyriva conformance test.

## v5 lifecycle

- complete entry load order across six kinds;
- teardown ordering across reload, disable, uninstall, and shutdown;
- exact callback time budgets;
- service crash restart and backoff;
- ordering between config updates, service restart, and state watchers;
- behavior when disable/update operations are accepted asynchronously but later
  fail;
- hot-reload state and callback preservation.

## v5 state, persistence, and IPC

- whether `state.watch` immediately receives an existing value;
- whether writing an equal value triggers watchers;
- table-copy depth and unsupported-value diagnostics;
- IPC payload encoding, result/error shape, and broadcast ordering;
- exact persistent-data migration behavior across plugin versions;
- partial-write and corrupt-data recovery.

## v5 sources and dependencies

- interrupted update recovery;
- conflicting files or dirty source directories;
- missing or invalid historic revisions;
- catalog/source removal while a plugin is enabled;
- duplicate canonical IDs with incompatible API levels;
- exact UI and log behavior for missing informational dependencies.

## v5 API boundaries

- installed beta.6 behavior for requested API 17 and 18;
- public docs marked those levels unreleased at the pinned baseline;
- behavior below oldest supported and above current supported;
- whether unsupported capabilities fail at parse, enable, load, or call time.

## v4 host

- exact creation and destruction order for all entry points;
- minimum Quickshell import versions;
- complete `qs.*` module and service façade;
- injection timing for `pluginApi` and entry-specific context;
- reload semantics and which state survives;
- QML error containment and restart behavior;
- IPC return and error semantics;
- settings write atomicity and corrupt-file recovery;
- panel focus, anchoring, multi-output behavior, and compositor differences;
- accessible semantics of reference widgets.

## Product-level gaps

- source/catalog UX has not passed accessibility acceptance;
- no compatible v5 or v4 host is yet verified without Noctalia installed;
- no representative compatibility fixture is verified on XRY;
- packaging does not yet prove the independent hosts are the installed runtime.

Unknowns remain visible here until a probe, fixture, implementation, and review
close them. They must not be converted into assumed behavior.
