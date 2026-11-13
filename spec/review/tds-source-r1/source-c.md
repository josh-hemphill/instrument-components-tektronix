# Council C — early TDS1002 implementation source review

Member **C**, actual model **gpt-6.1-sol**, reasoning **high**. Rubric **v2**; stable framing: consider model, module and execution distinctions. This is **read-only early source inspection**, not numeric implementation grading.

Frozen manifest SHA256: `2be2e5a2c21635775afd05e010faac47830ff3af7562859c68f4933cc4030c71`. The manifest and all 15 bound inputs match. I inspected these frozen copies against the passed full-nine v2 specification packet, not evolving worktree code. No peer review or scores were read, and no production code was edited.

## P1 — frozen C# source cannot compile against its declared SDK producer

**TDS-C-01: nonexistent `CatalogReply.Scalar` nested type.**

Locations in this packet:

- `dotnet/InstrumentComponents.Drivers.Tektronix/Tds1002.cs:106`: immediate VALUE decoder constructs `new CatalogReply.Scalar(...)`.
- `dotnet/InstrumentComponents.Drivers.Tektronix/Tds1002.cs:116`: ReadTimebaseScale constructs the same nonexistent type.

The manifest declares core producer `1243881c6b91c46ff40cf0cb3cfd4598b8352409`. I inspected that immutable commit's `dotnet/src/InstrumentComponents.Abstractions/Catalog.cs:33`: its variants are Empty, **Numeric**, State, Count, Trace and Identity. There is no Scalar nested type. Both TDS references therefore fail to resolve against the declared SDK; normal C# compilation and every later C#/Native AOT proof are blocked on this frozen revision.

**Required fix:** use `CatalogReply.Numeric(new Quantity(...))`, or its equivalent target-typed constructor, at both return sites. Preserve Volt/Hertz/Second units and default Valid status. Do not alter the neutral SDK merely to accommodate an accidental backend type name.

**Meaningful verification:** compile the actual Tek library against the checksum-identified producer, then execute ReadTimebaseScale and both measurement methods through all C# frontends. A finite VALUE followed by nonzero final ESR must still throw Device rather than return its already-decoded quantity.

Confidence: **high, direct source/declared-producer evidence**. I notified the facilitator immediately. The facilitator subsequently reported the same compile failure and a working-source replacement with Numeric. That reported fix is not present in this frozen packet and must be verified on the next frozen revision. I did not personally run a C# compiler in this review.

## Other inspected source behavior

No additional demonstrated protocol/design defect was found in the copied TDS implementation. The observations below are source alignment, not claims that the procedural or package/platform gates have executed.

### Exact model, module and route admission

Rust `src/tds1002.rs:76` and C# `Tds1002.cs:21` require an ASCII `scpi` scheme before trimming/case normalization, avoiding the earlier Unicode-whitespace scheme mismatch. Addresses remain exact keys after rejecting blank values. Factories require 1–4096 copied bindings, reject duplicates, unknown commissioning and unsupported communication modules, and accept only CM/CMA/CMAX for GPIB. MEM is not mistaken for a GPIB or B-model USB route.

Probe factors are converted through explicit enum switches to 1/10/100/1000; unknown/undefined values reject. C# copies only immutable record fields into an immutable Core, and Rust consumes Configuration into a private map; caller mutation cannot change the stored probes. Each selected endpoint uses its own core/probe pair, with no model-wide fallback. Exact TDS1002/TDS 1002 identities and ten operations are statically declared. The Rust library exports the new explicit module without silently appending this factory to generic profile registration.

Selected capability requirements are validated before the connector opens. Native open checks availability before using a native connector; sync connectors are not wrapped and advertised as native. Actual registry matching, required-mode rejection and no-acquisition behavior still need the named procedural proofs.

### Canonical numerical domains and parsers

Rust `tds1002.rs:218` and C# `Tds1002.cs:59` use the same 31 literal timebase values. Vertical scales multiply integer millivolts by integer probe factor before one binary64 division, avoiding multiplication of a rounded fractional BNC scale. Structural argument validation finishes while preparing actions, before HEAD/VERBOSE/status IO. Coupling tokens are the exact AC/DC/GND subset and channel indexing admits only 1/2.

The E11 algorithms normalize exponent padding in the same specified form. Integer register/count/probe decoders accept optional `+`, ASCII digits and leading zeros, strip insignificant zeros before bounded u32 parsing and enforce each context's maximum. Boolean and OPC bodies retain their exact single-character grammar.

Rust `tds1002.rs:245` and C# `Tds1002.cs:71` implement the specified NR3 grammar: required exponent, at least one integer/fraction digit as applicable, 1–3 exponent digits, no trailing content or Unicode. Positive numeric 9.9e37 is Device only for measurement values, before the generic giant-value rejection. Other giants/nonfinite values and nonzero mantissa underflow-to-zero are Parse. Vpp zero, signed zero and frequency's strictly positive domain are distinguished as specified. Timebase readback must equal a canonical value. Source-equivalent behavior requires cross-language execution at underflow, exponent and near-neighbor boundaries; source inspection alone is not a binary64 parity proof.

### Prefix, status and fresh acquisition

Rust `tds1002.rs:331`/`:394` and C# `Tds1002.cs:93`/`:108` match the complete required sequences. Every supported operation prepares HEAD OFF, VERB ON, DESE255 verification and preflight ESR0, then its action and a final ESR0. No DESE setter, CLS, event drain or hidden restoration appears.

Measurement actions guard MAIN/YT/NORMAL/visible channel/frozen probe, stop previous acquisition, set and verify SAMPLE, set and verify SEQUENCE, RUN, wait through OPC1, verify stopped state and count1, set and verify source/type/quoted units, then query VALUE. Both language plans append the final ESR check after value decoding. A reply is kept locally until every step succeeds, so a warning/error in the final ESR prevents a successful result. Token tables distinguish documented wrong states from malformed/unknown values, and no hidden enable/trigger/USB/B-only channel-unit command is added.

Run and Single remain start-only actions; fresh scalar methods perform their own restart rather than accepting the existing acquisition. Stop only stops acquisition. The implementation does not promise external trigger truth, electrical protection or physical calibration.

### Parent transaction, uncertainty and resource lifetime

Both vendor-local message wrappers validate request/plan before IO, mark health uncertain before each write/query/await, and restore it only after the complete operation. Parse, status Device, transport, timeout or active cancellation therefore leaves the opened backend quarantined; later operations reject before traffic. The value decoder cannot return early past a pending final status check.

The wrappers enforce bounded ASCII complete-body checks and reject controls; host allocation/framing/deadlines remain connector obligations. Resource-only close is idempotent and honors owned versus borrowed handles. Rust Drop follows the same release path. No device STOP/reset/clear/drain is emitted on close or failure. Actual whole-operation locking is supplied by the SDK runtime, and cancellation completion depends on the native host honoring its contract; both remain execution-dependent properties requiring the complete procedural artifact.

## Exact oracle and generated-source checks

The active corpus has **452 unique IDs**, each with exactly one expected result or error. I compared it directly with the independently reviewed TDS shard: **all 334 additions match exactly**. Using immutable Tek baseline `787c4881636d18618f1f8a503a7be7273333064f`, I verified that **all 118 existing cases are unchanged**.

Both copied generated language files contain exactly the same 452 unique case IDs as the active JSON. I extracted the SDK generator sources from immutable producer `1243881c6b91c46ff40cf0cb3cfd4598b8352409` into an isolated temporary directory and regenerated the full corpus: **`generated/cases.rs` and `generated/GeneratedCases.cs` are byte-identical** to the frozen copies. This establishes generation identity, not backend execution or physical evidence.

The facilitator reports successful Rust corpus executions, but execution logs, procedural tests, runner wiring, examples, final package manifests and native artifacts are outside this early source manifest. No execution/platform completion credit is granted here.

## Checks required on the next frozen implementation packet

1. **Resolve TDS-C-01** and bind an actual C# build plus generated/source and packed execution to the corrected source and compatible producer bytes.
2. Execute all 31 timebases and both channels across all four probe factors and 11 vertical scales (**88 channel/probe/scale combinations**), including binary64 near-neighbor and nonfinite rejection followed by a healthy valid operation. Include ASCII-versus-Unicode endpoint admission and wrong-module/probe/commissioning cases before acquisition.
3. Exercise pending prefix and sequence-RUN writes, OPC waits and VALUE reads with independent deadlines and active cancellation through native-only hosts. Assert no detached host access, sibling/class-view quarantine and no automatic STOP/reset/clear cleanup. Verify worker thread affinity and complete-operation noninterleaving separately.
4. Prove the final ESR gate with a finite measurement followed by warning/error status. Preserve exact quoted units, non-B probe/source semantics, invalid-result Device classification, underflow Parse and exact register-versus-Boolean grammar. Invalid structural requests must emit no format prefix.
5. Bind runner fixture selection, all 452 corpus outcomes, T-PROC01..11, static construction examples and immutable package/feed identities to real source/packed Linux/Windows x64 .NET8/10 Native AOT binaries/execution logs. Include the intentional warning-probe failure. Hosted SDK success cannot establish this vendor backend's qualification by itself.

These are meaningful final evidence requirements, not assertions that unprovided work already failed. The only proved required source correction in this frozen packet is TDS-C-01.

No final numeric scores or implementation approval are issued. Confidence is high in the compile defect, manifest/oracle/generator identity and inspected traffic/state logic; concurrency, cancellation, ownership under all runtime failure paths, compiled examples and package/native distribution remain unverified until the full frozen evidence review.
