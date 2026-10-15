# N02 isolated ifc-lite spike

Original MPL-2.0 evaluation code, not a production BeamCraft backend. See
[the report](../../docs/design/n02-ifc-lite-evaluation-v0.1.md) for the decision,
full inventory, provenance, limitations and proposed notices. No upstream code or fixtures copied.

From the repository root, on Windows with the existing Cargo cache:

```powershell
cargo build --offline --release --manifest-path spikes/n02-ifc-lite/Cargo.toml
python spikes/n02-ifc-lite/supervise.py
python spikes/n02-ifc-lite/checks.py
```

The executable waits for a one-byte gate. The supervisor installs a Windows Job Object
memory cap before geometry and kills a child after five seconds. Tests use original,
fixed-size analytic inputs. Results are evidence, not acceptance fixtures for an independent
replacement implementation. target/ artifacts are not deliverables. The empty workspace table
keeps the spike outside the production workspace. verification.json records final encoding,
lockfile and check-status evidence.
