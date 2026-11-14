# Instrument Components Tektronix drivers

Independently packaged Rust (`instrument-driver-tektronix`) and .NET (`InstrumentComponents.Drivers.Tektronix`)
source-derived profiles, coordinated release target `0.3.0-alpha.1`. Check registries and the matching release for availability;
no devices in this library have physical bench validation yet.

## Install and try the preview

```sh
cargo add instrument-driver-tektronix@=0.3.0-alpha.1
dotnet add package InstrumentComponents.Drivers.Tektronix --version "[0.3.0-alpha.1]"
```

Use the [complete hardware-free Rust/C# guide](https://github.com/josh-hemphill/instrument-components/blob/v0.3.0-alpha.1/docs-site/docs/getting-started.md) before connecting equipment. It uses exact public package versions and an in-memory development connector. Select this family's factories explicitly; adding a package never probes or opens hardware. The [release source tag](https://github.com/josh-hemphill/instrument-components-tektronix/tree/driver-v0.3.0-alpha.1) selects the coordinated release once published; the latest branch contains ongoing development.


Models and aliases are passive hints carried forward from main commit
`5eb92b17f775d326e5a3542ee40855b156321745`: AFG31000, FCA3100, MDO3024, RSA306B.
See `provenance/identities.json` for every manufacturer alias and the source
resolution, and `provenance/source.json` for frozen source hashes. A hint is not
a claim that every listed model implements these commands. Suspicious inherited
identities, including Keithley's E36312A hint, are preserved as unvalidated data.

Applications explicitly register `sync_factories` / `native_factories` in Rust,
or `Drivers.Factories` / `Drivers.NativeFactories` in C#. Connector ownership,
serialization, cancellation, recovery and class views use the public SDK. No
runtime assembly loading, reflection activation, or core registry installation
is required. Multiple class views share the same opened handle. Factories have
one canonical identity for equivalent aliases; the 2602B is one DMM+PSU factory.

Supported channel IDs are inherited hints, not complete physical inventories.
Supply and generator outputs are unknown by default: metadata channel count and
checked shutdown are unavailable. Pass immutable `InventoryBinding` values keyed
by driver ID before opening a factory to declare the actual complete inventory.
Supply bindings can use sparse channel IDs. The modeled generator has one
unchanneled output; it cannot safely represent multi-output shutdown. An unknown
or multi-output setup must not claim checked shutdown from this model.

Shutdown disables every declared output and executes class-specific stop,
open-all or continuous-off actions, attempting all actions despite failures.
It never implies reset. DMM/counter/power-meter output-off is a no-op for their
modeled outputs. Temperature results default to Unknown physical units, also
through Read/Fetch; no Celsius assumption or unit configuration command is sent.
Generic remote-sense enable/disable maps Internal to OFF and External to ON,
following the legacy source documentation. N6705 sense remains unsupported.
DMM6500 ranged measurements explicitly preserve the old generic wire commands;
this source-derived behavior still requires device evidence.

`profiles.json` is the production wire plan. `cases.json` is a separately
specified golden traffic/value corpus; its generation never reads production
plans. Generated Rust/C# files contain static data and require no runtime JSON.
`provenance/operation-coverage.json` joins behavior to the shared case IDs.
`provenance/source/` is historical audit evidence, excluded from production
packages. The SDK TestKit and Tokio consumers are development-only dependencies.

The previous 0.2.0-alpha.1 SDK and driver packages are available on crates.io and NuGet. The release workflow tests fresh public-package consumers on Linux/Windows x64 and .NET 8/10.

For pre-publication development, CI also uses checksummed release-form SDK archives from
a pinned main workflow. The consumer job checks out only this repository and
resolves exact versioned SDK dependencies from fresh verified development feeds.
This is explicitly bootstrap validation, not public registry availability.
`instrument-driver verify-package` builds generated-source and packed consumers,
runs Rust default and async modes, and publishes and executes .NET Native AOT
with all four frontends on Linux/Windows x64 and net8.0/net10.0. It also requires
the intentional trimming-warning probe to fail. No hardware access occurs.

Core applications that do not select this package need none of its artifacts.
Release tags are immutable. A subsequent release needs a new version, matching SDK pins, passing native gates and council review.

## Connection-specific inventories

Use a `ConnectionBinding` containing the canonical driver ID, exact endpoint and inventory. C# accepts `connections:` in `Drivers.Factories`/`NativeFactories`. Rust exports `sync_factories_for_connections`, `native_factories_for_connections` and `combined_factories_for_connections`. All bindings are frozen and validated before IO. Two endpoints may use the same driver with different inventories. Unknown output inventory never inherits another endpoint's complete output list. Model-wide defaults remain available separately. Unknown driver IDs and normalized duplicate endpoints fail explicitly.

The committed exact wire cases now execute through endpoint bindings, including sparse channel IDs, per-operation transport failures and closed-handle checks. Source evidence remains distinct from physical evidence. All model hints in this repository are in initial scope; physical validation is pending.


## TDS1002 scalar subset in the 0.3 release target

The exact confirmed model is **TDS1002**, without A/B. The coordinated **0.3.0-alpha.1** target includes its explicit manual-derived scalar backend. The historical 0.2 distribution excludes this factory. Hardware validation remains pending. Current package execution and Council distribution review gate publication; check the matching release for public availability.

Register Rust `tds1002::Factory` / C# `Tds1002Factory` explicitly, with exact endpoint bindings and frozen module/probe/commissioning configuration. Each selected endpoint has its own probe factors. A documented **TDS2CM, TDS2CMA or TDS2CMAX** module is required for this GPIB subset. **TDS2MEM does not supply GPIB**. No direct USB/B-model adapter assumption or automatic interface discovery is made. Manufacturer TEKTRONIX and exact models TDS1002/TDS 1002 are passive matches; A/B aliases never match.

The ten supported neutral methods are Set/ReadTimebaseScale, SetChannelScale/Display/Coupling, Run/Stop/Single and fresh MeasureVpp/Frequency on channels1/2. Nominal timebase is5ns–50s/div in the model's1–2.5–5 sequence; vertical settings use the declared1/10/100/1000 voltage-probe factor and exact nominal scales. Waveform transfer, trigger source/level/slope setters, reset/shutdown/identity and math/current/B-only commands are deferred. Existing MDO3024 profiles remain unchanged and do not substitute for this factory.

Before opening, commission exclusive stable control, clean message/status state, **DESE255**, main/YT display, normal triggering, Trigger View off, matching physical voltage probes/attenuation and appropriate trigger source/level/slope/coupling. The driver preserves trigger settings. Its guards do not establish probe connection, calibrated accuracy or input protection. Normal triggering may wait indefinitely without a suitable trigger, so use a finite host deadline long enough for the chosen timebase. A50s/div capture can take hundreds of seconds.

Every operation sets HEAD OFF and VERB ON, verifies DESE255 and requires pre/post ESR0. These communication changes persist. ESR reads consume status/event history; there is no silent *CLS, DESE setter or event-queue drain. A finite measurement with a warning is rejected, and9.9E37 is an invalid-measurement Device error rather than an overload reading. An uncommissioned/masked status state cannot quietly produce Valid.

**Fresh scalar measurements stop prior acquisition, select SAMPLE/SEQUENCE, start a new acquisition, await OPC and verify stopped state/count/source/type/quoted units/value/final status. They leave the scope stopped in SAMPLE/SEQUENCE.** Reading VALUE alone is insufficient. Run and Single are start-only methods and do not wait for capture completion. Stop stops acquisition; it is not electrical safe shutdown.

Use a complete-message plain-text connector with LF/EOI framing,256-byte bounded scalar allocation and a positive finite deadline. Standard .NET VISA composition sets `endpointScheme: "scpi"`, `terminator: "\n"`, `maxResponseBytes: 256`, `allowBinaryBlocks: false`. Reject binary blocks before removing headers and preserve payload whitespace. No physical IO is performed by registration/open. Attach a genuine native connector with Rust `with_native_connector` / C# `nativeConnector`; blocking connectors use the SDK worker paths.

The [specification](spec/models/TDS1002.md) defines every command/range/parser/state change. Executed [Rust](validation/rust/src/tds_examples.rs) and [C#](validation/dotnet/TdsExamples.cs) composition examples separate synthetic TestKit setup from application construction and typed calls. Read the [procedural coverage map](validation/tds1002-procedural-coverage.md) and independent oracle/provenance before bench testing. All118 earlier cases are preserved and334 independently extracted TDS cases are added:452 active cases.

An uncertain IO/parse/Device failure or active cancellation quarantines the whole handle. Cancellation may leave an acquisition running; resynchronize/recommission externally and reopen. There is no implicit STOP/clear/reset/retry or mode restoration. Close releases owned resources once, preserves borrowed resources and never claims device shutdown. Source/packed Native AOT evidence is software conformance, separate from hardware evidence and from OpenTAP host compatibility.

## Coordinated 0.3 release

The 0.3.0-alpha.1 distribution includes the current merged software implementation and uses exact matching SDK dependencies. The 0.2 exclusions and historical proof records above describe that earlier release. New device additions remain hardware-unvalidated and require explicit commissioning/configuration. Core production packages do not bundle this driver. Release workflows run Windows/Linux x64 and .NET 8/10 package consumers; the Keysight release runs the final all-five public composition gate.
