# Council A — early TDS1002 implementation source inspection

Member **A**; actual model **gpt-6.1-sol**, reasoning **high**. Stable framing: consider a new user's interpretation. Rubric **v2**. Stage **EARLY IMPLEMENTATION SOURCE INSPECTION**; no final numeric grades assigned.

Frozen manifest SHA256 **2be2e5a2c21635775afd05e010faac47830ff3af7562859c68f4933cc4030c71**. I verified this identity and all **15** bound-file hashes. The linked approved full-spec packet independently matches SHA256 **c52c37b13a460ea7e5c86916d1d51c9377fc8e9f11848347a3727581f5208a76**. Only the frozen copies govern this inspection; no peer findings or scores were read.

## Proved required correction

### A-TDS-SRC-01 — C# uses an absent SDK reply type and cannot compile

**P1; confidence high.** The frozen C# backend constructs `CatalogReply.Scalar` at:

- `dotnet/InstrumentComponents.Drivers.Tektronix/Tds1002.cs:106`, measurement VALUE reply.
- `dotnet/InstrumentComponents.Drivers.Tektronix/Tds1002.cs:116`, main-timebase reply.

The manifest-pinned immutable SDK source **1243881c6b91c46ff40cf0cb3cfd4598b8352409** defines `CatalogReply.Numeric(Quantity Value)` at `dotnet/src/InstrumentComponents.Abstractions/Catalog.cs:36`. The complete CatalogReply declaration has no `Scalar` nested record. I inspected that SDK file with `git show` at the exact pinned commit. The frozen generated expected replies also use `CatalogReply.Numeric`.

These two references therefore block compilation of the C# backend against the declared SDK, independently of runtime traffic. **Required fix:** replace both references with the actual SDK Numeric reply variant, then build and execute both quantity-producing paths and the complete source/packed C# corpus in the required frontends/frameworks. Preserve this frozen finding; a subsequent corrected snapshot and execution evidence should demonstrate resolution.

I reported this defect immediately. The facilitator subsequently reported the same compiler failure and a correction in the working source. That update is not independently verified by this frozen packet, so the defect remains recorded here. I did not run the compiler or edit the code.

## Source observations against the approved contract

Apart from the compile blocker, I found no additional demonstrated command/parser/state/admission defect in the inspected source. These statements describe code paths, not executed conformance.

**Factory/admission:** `src/tds1002.rs:76` and C# `Tds1002.cs:21` both reject non-ASCII schemes before trimming, so the earlier Keysight Unicode-scheme mismatch is not repeated here. Both constructors bound/copied inputs, derive immutable per-endpoint probe factors, reject MEM/Unknown module/probe/commissioning and duplicate routes, and perform selected-route/required-capability checks before connector acquisition. Exact TDS1002/TDS 1002 identities and ten operations match the approved subset. Native connector absence rejects before its open; generic profile registration does not silently instantiate the commissioned factory. The C# Core is immutable and safely shared as configuration rather than session health.

**Canonical settings:** Rust TIMEBASES at218 and the C# table contain the31 exact decimal-literal settings. Vertical admission multiplies integer millivolts by the frozen integer probe factor before one binary64 division, with maximum product5,000,000 safely within u32. Structural rejection is completed while preparing the plan, before HEAD/VERB/status IO. E11 formatting is invariant and exponent-normalized in each language. Probe/state checks precede settings writes as specified.

**Fresh acquisition/status:** Rust `measure` at331 and C# `Measure` at95 preserve the approved MAIN/YT/NORMAL/visible/probe guards and STOP/SAMPLE/verify/SEQUENCE/verify/RUN/OPC/stopped/count sequence. Both then select and verify immediate source/type/quoted unit and read VALUE. `prepare` appends a final ESR check to every supported operation, including scalar reads and start-only control actions. The wrappers keep a decoded quantity private until the full transaction, including final ESR, succeeds; a finite VALUE followed by warning ESR cannot escape as a successful returned quantity. No HEAD/VERB/DESE/error-queue drain or hidden channel enable beyond the declared prefix/action behavior is invented.

**Numeric/token parsing:** Rust integer/NR3 helpers at228/245 and C# helpers at65/71 enforce ASCII grammar, optional-plus integer bodies and domains, mandatory NR3 exponent with1–3 digits, whole-body consumption, nonfinite/giant rejection and nonzero-mantissa underflow rejection. Measurement positive9.9e37 receives Device before generic magnitude rejection. Actual zero Vpp remains legal; frequency zero/negative and negative Vpp are Device. Timebase giant/sentinel values do not inherit the measurement-only exception. Exact Boolean/OPC tokens stay distinct from NR1 registers, full discrete keywords and quoted units. Long leading-zero integer bodies are intentionally normalized before u32 parsing, within the shared256-byte body bound.

**Lifetime/quarantine:** Rust/C# transaction wrappers validate and prepare before IO, mark uncertainty before each host access/await, stop at the first IO/decode error and restore healthy state only after all steps complete. A post-status error preserves quarantine even after a valid numeric decode. Rust future abandonment leaves unhealthy state once host IO begins. Owned close is idempotent, borrowed resources are preserved and no device cleanup/STOP is issued. The SDK supplies whole-operation serialization; actual pending host access ending before cancellation completion is a connector/runtime obligation still requiring integration execution.

## Corpus identity and preservation

I checked the frozen452-case active corpus: **452 unique IDs**. Every one of the **334** approved TDS cases is present with identical logical JSON content. The pre-existing **118** Tek cases at baseline commit **787c4881636d18618f1f8a503a7be7273333064f** are also present unchanged. Thus the snapshot adds the new oracle without deleting or rewriting the baseline corpus. The approved334-case shard/provenance and synthetic/manual-derived evidence labels remain intact.

This identity check does not prove the generated cases execute through the new factory or that generation checks are current. The facilitator reports successful Rust452-case frontend execution; those logs/results are outside this frozen packet and receive no independent execution credit here.

## Meaningful checks required in the later frozen evidence

1. **Resolve A-TDS-SRC-01 with compilation and actual C# quantity-path execution.** Include ReadTimebaseScale, Vpp and frequency with final ESR success, plus a valid finite value followed by ESR16 yielding Device and subsequent no-traffic quarantine.
2. **Probe/grid admission:** execute all four factors on both channels, canonical millivolt-derived scales and all31 timebases; reject adjacent binary64 values, NaN/infinities, invalid channels/symbols and unsupported module/enum/endpoint values before acquisition or prefix writes. Mutating caller collections must leave the original valid endpoint/probe mapping intact. Exercise distinct endpoints with different probes.
3. **Pending/native cancellation and deadlines:** pause actual prefix writes, RUN writes, OPC and VALUE queries after a start signal. Verify finite host deadline independence, no synchronous fallback, no host access after cancellation completes, no implicit STOP/reset/drain and a different follow-up operation quarantined without traffic.
4. **Whole-operation serialization and ownership:** pause one multistep scalar operation while a second view waits; prove no second prefix/action interleaves. Exercise owned/borrowed close twice, failed/cancelled/undelivered opens, worker thread affinity and repeated lifecycle use.
5. **Parser boundaries:** cover bounded long-leading-zero NR1, plus/malformed register forms, signed zero, genuine zero versus nonzero-underflow, measurement-only sentinel precedence, Unicode/control/binary/header/CSV rejection, malformed discarded state/unit responses and real host framing/allocation bounds. Keep source/packed and generated/golden case identity joined.
6. **Final source/packed gate:** inspect the corrected current files, compiled examples, full T-PROC execution and452-case corpus in all required Rust/C# frontends, actual Linux/Windows x64 NativeAOT .NET8/10 binaries, warning-probe failure, package boundaries and exact producer/consumer checksums. Successful SDK producer cells alone do not qualify this vendor backend.

These are required integration checks already within the approved acceptance scope, not newly invented feature requests. This early packet does not contain their final implementations/results and cannot establish the final implementation gate.

No live instrument, physical trigger/probe truth, calibration, publication or merge is asserted. Only this review artifact was written; all frozen input hashes remain unchanged. The final complete nine-item54-category implementation review remains separate.
