//! Commissioned non-B TDS1002 scalar operations. Fresh measurements change
//! acquisition state and consume status; resource close never restores either.
use crate::message_backend::{self as tx, error, Engine, Step};
use instrument_api::{catalog::*, *};
use instrument_scpi::Connector;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommunicationModule {
    Unknown,
    Tds2Cm,
    Tds2Cma,
    Tds2Cmax,
    Tds2Mem,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Probe {
    Unknown,
    X1,
    X10,
    X100,
    X1000,
}
impl Probe {
    fn factor(self) -> Result<u32> {
        match self {
            Self::X1 => Ok(1),
            Self::X10 => Ok(10),
            Self::X100 => Ok(100),
            Self::X1000 => Ok(1000),
            _ => Err(bad()),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Commissioning {
    Unknown,
    QualifiedGpibNormalModeConfirmed,
}
#[derive(Clone, Copy, Debug)]
pub struct Configuration {
    pub communication_module: CommunicationModule,
    pub channel_1_probe: Probe,
    pub channel_2_probe: Probe,
    pub commissioning: Commissioning,
}
#[derive(Clone, Debug)]
pub struct Binding {
    pub endpoint: Endpoint,
    pub configuration: Configuration,
}
#[derive(Clone)]
pub struct Factory {
    routes: BTreeMap<(String, String), Core>,
    connector: Arc<dyn Connector>,
    #[cfg(feature = "async")]
    native: Option<Arc<dyn instrument_scpi::AsyncConnector>>,
}
fn bad() -> DriverError {
    error(
        ErrorKind::InvalidArgument,
        "Require qualified GPIB module, probes and exact endpoint",
    )
}
fn parse() -> DriverError {
    error(ErrorKind::Parse, "Unexpected non-B TDS1002 scalar body")
}
fn device() -> DriverError {
    error(
        ErrorKind::Device,
        "Scope state or measurement violates commissioned contract",
    )
}
fn endpoint(value: &Endpoint) -> Result<()> {
    if !value.scheme.is_ascii()
        || !value.scheme.trim().eq_ignore_ascii_case("scpi")
        || value.address.trim().is_empty()
    {
        return Err(bad());
    }
    Ok(())
}
fn operations() -> BTreeSet<Operation> {
    use CatalogOperation::*;
    [
        OscilloscopeSetTimebaseScale,
        OscilloscopeReadTimebaseScale,
        OscilloscopeSetChannelScale,
        OscilloscopeSetChannelDisplay,
        OscilloscopeSetChannelCoupling,
        OscilloscopeRun,
        OscilloscopeStop,
        OscilloscopeSingle,
        OscilloscopeMeasureVpp,
        OscilloscopeMeasureFrequency,
    ]
    .into_iter()
    .map(Operation::Catalog)
    .collect()
}
impl Factory {
    pub fn new(bindings: Vec<Binding>, connector: Arc<dyn Connector>) -> Result<Self> {
        if bindings.is_empty() || bindings.len() > 4096 {
            return Err(bad());
        }
        let mut routes = BTreeMap::new();
        for binding in bindings {
            endpoint(&binding.endpoint)?;
            let c = binding.configuration;
            if !matches!(
                c.communication_module,
                CommunicationModule::Tds2Cm
                    | CommunicationModule::Tds2Cma
                    | CommunicationModule::Tds2Cmax
            ) || c.commissioning != Commissioning::QualifiedGpibNormalModeConfirmed
            {
                return Err(bad());
            }
            let core = Core {
                probes: [c.channel_1_probe.factor()?, c.channel_2_probe.factor()?],
            };
            if routes
                .insert(tx::endpoint_key(&binding.endpoint), core)
                .is_some()
            {
                return Err(bad());
            }
        }
        Ok(Self {
            routes,
            connector,
            #[cfg(feature = "async")]
            native: None,
        })
    }
    #[cfg(feature = "async")]
    pub fn with_native_connector(
        mut self,
        connector: Arc<dyn instrument_scpi::AsyncConnector>,
    ) -> Self {
        self.native = Some(connector);
        self
    }
    fn selected(&self, context: &OpenContext<'_>) -> Result<Core> {
        endpoint(&context.candidate.endpoint)?;
        let core = self
            .routes
            .get(&tx::endpoint_key(&context.candidate.endpoint))
            .ok_or_else(bad)?;
        if !context.required.is_subset(&operations()) {
            return Err(error(
                ErrorKind::UnsupportedOperation,
                "Unsupported TDS1002 operation",
            ));
        }
        Ok(core.clone())
    }
}
impl DriverFactory for Factory {
    fn descriptor(&self) -> DriverDescriptor {
        DriverDescriptor {
            id: "tektronix.tds1002".into(),
            contract_major: CONTRACT_MAJOR,
            rules: ["TDS1002", "TDS 1002"]
                .into_iter()
                .map(|model| MatchRule::ExactIdentity {
                    scheme: "scpi".into(),
                    manufacturer: "TEKTRONIX".into(),
                    model: model.into(),
                })
                .collect(),
            operations: operations(),
        }
    }
    fn execution_modes(&self) -> BTreeSet<ExecutionMode> {
        #[cfg(feature = "async")]
        let native = self.native.is_some();
        #[cfg(not(feature = "async"))]
        let native = false;
        tx::modes(native)
    }
    fn open(&self, context: &OpenContext<'_>) -> Result<Box<dyn DeviceBackend>> {
        let core = self.selected(context)?;
        Ok(Box::new(tx::Device::new(
            core,
            self.connector.open(context)?,
            context.ownership,
        )))
    }
    #[cfg(feature = "async")]
    fn open_native<'a>(
        &'a self,
        context: &'a OpenContext<'_>,
    ) -> DriverFuture<'a, Box<dyn AsyncDeviceBackend>> {
        Box::pin(async move {
            let core = self.selected(context)?;
            let native = self.native.as_ref().ok_or_else(|| {
                error(
                    ErrorKind::UnsupportedExecutionMode,
                    "Native connector not configured",
                )
            })?;
            Ok(Box::new(tx::AsyncDevice::new(
                core,
                native.open(context).await?,
                context.ownership,
            )) as Box<dyn AsyncDeviceBackend>)
        })
    }
}
#[derive(Clone)]
struct Core {
    probes: [u32; 2],
}
// Canonical decimal literals, rather than multiplication of rounded doubles.
const TIMEBASES: [f64; 31] = [
    5e-9, 1e-8, 2.5e-8, 5e-8, 1e-7, 2.5e-7, 5e-7, 1e-6, 2.5e-6, 5e-6, 1e-5, 2.5e-5, 5e-5, 1e-4,
    2.5e-4, 5e-4, 1e-3, 2.5e-3, 5e-3, 1e-2, 2.5e-2, 5e-2, 1e-1, 2.5e-1, 5e-1, 1.0, 2.5, 5.0, 10.0,
    25.0, 50.0,
];
fn real12(value: f64) -> String {
    let raw = format!("{value:.11E}");
    let (m, e) = raw.split_once('E').expect("scientific formatter");
    format!("{m}E{:+03}", e.parse::<i32>().expect("scientific exponent"))
}
fn integer(body: &str, max: u32) -> Result<u32> {
    let digits = body.strip_prefix('+').unwrap_or(body);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(parse());
    }
    // Leading zeros are valid even when the bounded record exceeds u32 length.
    let significant = digits.trim_start_matches('0');
    let value = if significant.is_empty() {
        0
    } else {
        significant.parse::<u32>().map_err(|_| parse())?
    };
    if value > max {
        return Err(parse());
    }
    Ok(value)
}
fn nr3(body: &str, measurement: bool) -> Result<f64> {
    let bytes = body.as_bytes();
    let mut i = usize::from(matches!(bytes.first(), Some(b'+') | Some(b'-')));
    let start = i;
    while bytes.get(i).is_some_and(u8::is_ascii_digit) {
        i += 1;
    }
    if i == start {
        return Err(parse());
    }
    if bytes.get(i) == Some(&b'.') {
        i += 1;
        let fraction = i;
        while bytes.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        if i == fraction {
            return Err(parse());
        }
    }
    let mantissa_end = i;
    if !matches!(bytes.get(i), Some(b'E') | Some(b'e')) {
        return Err(parse());
    }
    i += 1;
    if matches!(bytes.get(i), Some(b'+') | Some(b'-')) {
        i += 1;
    }
    let exponent = i;
    while bytes.get(i).is_some_and(u8::is_ascii_digit) {
        i += 1;
    }
    if i != bytes.len() || !(1..=3).contains(&(i - exponent)) {
        return Err(parse());
    }
    let value = body.parse::<f64>().map_err(|_| parse())?;
    if measurement && value == 9.9e37 {
        return Err(device());
    }
    if !value.is_finite()
        || value.abs() >= 1e30
        || (value == 0.0
            && bytes[start..mantissa_end]
                .iter()
                .any(|b| matches!(b, b'1'..=b'9')))
    {
        return Err(parse());
    }
    Ok(value)
}
#[derive(Clone)]
enum Check {
    Mask,
    Esr,
    Token(String, &'static [&'static str]),
    Probe(u32),
    Timebase,
    Opc,
    State,
    Count,
    Value(Unit),
}
fn token(expected: &str, known: &'static [&'static str]) -> Check {
    Check::Token(expected.into(), known)
}
fn query(message: &str, check: Check) -> Step<Check> {
    Step::Query(message.into(), check)
}
fn write(message: &str) -> Step<Check> {
    Step::Write(message.into())
}
fn prefix() -> Vec<Step<Check>> {
    vec![
        write(":HEAD OFF"),
        write(":VERB ON"),
        query(":DESE?", Check::Mask),
        query("*ESR?", Check::Esr),
    ]
}
impl Core {
    fn probe(&self, channel: u32) -> Result<u32> {
        if !(1..=2).contains(&channel) {
            return Err(bad());
        }
        Ok(self.probes[(channel - 1) as usize])
    }
    fn measure(&self, channel: u32, frequency: bool) -> Result<Vec<Step<Check>>> {
        let probe = self.probe(channel)?;
        let source = format!("CH{channel}");
        let kind = if frequency { "FREQ" } else { "PK2PK" };
        Ok(vec![
            query(":HOR:VIEW?", token("MAIN", &["MAIN", "WINDOW", "ZONE"])),
            query(":DISP:FORM?", token("YT", &["YT", "XY"])),
            query(":TRIG:MAIN:MODE?", token("NORMAL", &["NORMAL", "AUTO"])),
            query(&format!(":SEL:CH{channel}?"), token("1", &["0", "1"])),
            query(&format!(":CH{channel}:PRO?"), Check::Probe(probe)),
            write(":ACQ:STATE STOP"),
            write(":ACQ:MODE SAMPLE"),
            query(
                ":ACQ:MODE?",
                token("SAMPLE", &["SAMPLE", "PEAKDETECT", "AVERAGE"]),
            ),
            write(":ACQ:STOPA SEQ"),
            query(":ACQ:STOPA?", token("SEQUENCE", &["SEQUENCE", "RUNSTOP"])),
            write(":ACQ:STATE RUN"),
            query("*OPC?", Check::Opc),
            query(":ACQ:STATE?", Check::State),
            query(":ACQ:NUMACQ?", Check::Count),
            write(&format!(":MEASU:IMM:SOU {source}")),
            write(&format!(":MEASU:IMM:TYP {kind}")),
            query(":MEASU:IMM:SOU?", token(&source, &["CH1", "CH2"])),
            query(
                ":MEASU:IMM:TYP?",
                token(
                    if frequency { "FREQUENCY" } else { "PK2PK" },
                    &[
                        "FREQUENCY",
                        "MEAN",
                        "PERIOD",
                        "PK2PK",
                        "CRMS",
                        "MINIMUM",
                        "MAXIMUM",
                        "RISE",
                        "FALL",
                        "PWIDTH",
                        "NWIDTH",
                    ],
                ),
            ),
            query(
                ":MEASU:IMM:UNI?",
                token(
                    if frequency { "\"Hz\"" } else { "\"V\"" },
                    &["\"V\"", "\"s\"", "\"Hz\""],
                ),
            ),
            query(
                ":MEASU:IMM:VAL?",
                Check::Value(if frequency { Unit::Hertz } else { Unit::Volt }),
            ),
        ])
    }
}
impl Engine for Core {
    type Check = Check;
    fn operations(&self) -> BTreeSet<Operation> {
        operations()
    }
    fn prepare(&self, request: &CatalogRequest) -> Result<Vec<Step<Check>>> {
        use CatalogRequest::*;
        let actions = match request {
            OscilloscopeSetTimebaseScale { seconds_per_div } => {
                if !TIMEBASES.contains(seconds_per_div) {
                    return Err(bad());
                }
                vec![
                    query(":HOR:VIEW?", token("MAIN", &["MAIN", "WINDOW", "ZONE"])),
                    write(&format!(":HOR:MAIN:SCA {}", real12(*seconds_per_div))),
                ]
            }
            OscilloscopeReadTimebaseScale => vec![
                query(":HOR:VIEW?", token("MAIN", &["MAIN", "WINDOW", "ZONE"])),
                query(":HOR:MAIN:SCA?", Check::Timebase),
            ],
            OscilloscopeSetChannelScale {
                channel,
                volts_per_div,
            } => {
                let probe = self.probe(*channel)?;
                if ![2u32, 5, 10, 20, 50, 100, 200, 500, 1000, 2000, 5000]
                    .iter()
                    .any(|m| f64::from(m * probe) / 1000.0 == *volts_per_div)
                {
                    return Err(bad());
                }
                vec![
                    query(&format!(":CH{channel}:PRO?"), Check::Probe(probe)),
                    write(&format!(":CH{channel}:SCA {}", real12(*volts_per_div))),
                ]
            }
            OscilloscopeSetChannelDisplay { channel, enabled } => {
                self.probe(*channel)?;
                vec![write(&format!(
                    ":SEL:CH{channel} {}",
                    if *enabled { "ON" } else { "OFF" }
                ))]
            }
            OscilloscopeSetChannelCoupling { channel, coupling } => {
                self.probe(*channel)?;
                if !["AC", "DC", "GND"].contains(&coupling.as_str()) {
                    return Err(bad());
                }
                vec![write(&format!(":CH{channel}:COUP {coupling}"))]
            }
            OscilloscopeRun => vec![write(":ACQ:STOPA RUNSTOP"), write(":ACQ:STATE RUN")],
            OscilloscopeStop => vec![write(":ACQ:STATE STOP")],
            OscilloscopeSingle => vec![write(":ACQ:STOPA SEQ"), write(":ACQ:STATE RUN")],
            OscilloscopeMeasureVpp { channel } => self.measure(*channel, false)?,
            OscilloscopeMeasureFrequency { channel } => self.measure(*channel, true)?,
            _ => {
                return Err(error(
                    ErrorKind::UnsupportedOperation,
                    "Unsupported TDS1002 operation",
                ))
            }
        };
        let mut steps = prefix();
        steps.extend(actions);
        steps.push(query("*ESR?", Check::Esr));
        Ok(steps)
    }
    fn decode(&self, check: Check, body: &str) -> Result<Option<CatalogReply>> {
        match check {
            Check::Mask | Check::Esr => {
                let expected = if matches!(check, Check::Mask) { 255 } else { 0 };
                if integer(body, 255)? != expected {
                    return Err(device());
                }
            }
            Check::Token(expected, known) => {
                if body != expected {
                    return Err(if known.contains(&body) {
                        device()
                    } else {
                        parse()
                    });
                }
            }
            Check::Probe(expected) => {
                let value = integer(body, 1000)?;
                if ![1, 10, 100, 1000].contains(&value) {
                    return Err(parse());
                }
                if value != expected {
                    return Err(device());
                }
            }
            Check::Opc => {
                if body != "1" {
                    return Err(parse());
                }
            }
            Check::State => {
                if body != "0" {
                    return Err(if body == "1" { device() } else { parse() });
                }
            }
            Check::Count => {
                if integer(body, 2147483647)? != 1 {
                    return Err(device());
                }
            }
            Check::Timebase => {
                let value = nr3(body, false)?;
                if !TIMEBASES.contains(&value) {
                    return Err(device());
                }
                return Ok(Some(CatalogReply::Quantity(Quantity {
                    value,
                    unit: Unit::Second,
                    status: MeasurementStatus::Valid,
                })));
            }
            Check::Value(unit) => {
                let value = nr3(body, true)?;
                if value < 0.0 || (unit == Unit::Hertz && value <= 0.0) {
                    return Err(device());
                }
                return Ok(Some(CatalogReply::Quantity(Quantity {
                    value,
                    unit,
                    status: MeasurementStatus::Valid,
                })));
            }
        }
        Ok(None)
    }
}
