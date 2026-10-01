# Passive capture adapters

This crate is the deliberately isolated first stage of desktop packet capture.
It will provide a receive-only WinDivert adapter on Windows x86/x64 and a pcap
adapter on supported Unix platforms. Nothing in the desktop app or existing
services starts or links these adapters yet.

## Boundary

- Native libraries are loaded explicitly at runtime, never linked at startup.
- Retain the native library for every function pointer and open handle.
- Capture only inbound TCP server traffic with source port 3101; no client payloads.
- No packet injection, driver installation, permission changes, or live capture tests.
- No protocol decoding/reassembly, ingress, persistence, background service, IPC,
  installer, UI, or packaging in this change.

## Verification

Tests use synthetic packets and mock native APIs. Native live capture, permission
configuration, and installed-driver behavior must be validated separately with
explicit authorization on the target operating systems.
