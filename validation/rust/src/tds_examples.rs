//! Executed static-composition example; only the TestKit script is test setup.
use crate::generated_cases;
use instrument_api::{catalog::Oscilloscope, *};
use instrument_driver_tektronix::tds1002::{
    Binding, Commissioning, CommunicationModule, Configuration, Factory, Probe,
};
use instrument_runtime::{DriverRegistry, Selection};
use instrument_testkit::{Script, ScriptStep};
use std::{collections::BTreeSet, sync::Arc};
pub fn run() -> Result<()> {
    let ids = [
        "timebase-nominal-16",
        "channel-1-scale-100mv",
        "channel-1-display-True",
        "channel-1-coupling-DC",
        "channel-1-fresh-Vpp",
    ];
    let cases = generated_cases::cases();
    let mut steps = Vec::new();
    for id in ids {
        let case = cases
            .iter()
            .find(|c| c.id == format!("tektronix.tds1002|E|{id}"))
            .unwrap();
        steps.extend(
            case.script
                .iter()
                .filter(|s| !matches!(s, ScriptStep::Close(_)))
                .cloned(),
        );
    }
    steps.push(ScriptStep::Close(Ok(())));
    let script = Script::new(steps);
    // Application code begins here. Replace Script with an application-supplied
    // commissioned connector enforcing LF/EOI,256-byte bodies and finite deadlines.
    // CM/CMA/CMAX GPIB, main/YT/normal trigger, matching probes, clean status,
    // DESE255 and stable exclusive control are external lifetime preconditions.
    let connector = Arc::new(script.clone());
    let endpoint = Endpoint {
        scheme: "scpi".into(),
        address: "fixture".into(),
    };
    let factory = Factory::new(
        vec![Binding {
            endpoint: endpoint.clone(),
            configuration: Configuration {
                communication_module: CommunicationModule::Tds2Cma,
                channel_1_probe: Probe::X10,
                channel_2_probe: Probe::X1,
                commissioning: Commissioning::QualifiedGpibNormalModeConfirmed,
            },
        }],
        connector,
    )?;
    let registry = DriverRegistry::new(vec![Arc::new(factory) as Arc<dyn DriverFactory>])?;
    let candidate = Candidate {
        endpoint,
        identity: Identity {
            manufacturer: Some("TEKTRONIX".into()),
            model: Some("TDS1002".into()),
            ..Identity::default()
        },
    };
    let handle = registry.open(
        &candidate,
        &Selection::default(),
        &BTreeSet::new(),
        ExecutionMode::Sync,
        ConnectionOwnership::Owned,
    )?;
    let scope = Oscilloscope(&handle);
    scope.set_timebase_scale(1e-3)?;
    scope.set_channel_scale(1, 1.0)?;
    scope.set_channel_display(1, true)?;
    scope.set_channel_coupling(1, "DC".into())?;
    let vpp = scope.measure_vpp(1)?;
    assert_eq!(vpp.value, 5.16);
    // Fresh scalar replaces acquisition mode, consumes ESR/event history and
    // leaves stopped SAMPLE/SEQUENCE. HEAD OFF/VERB ON persist. No restore/STOP
    // is inferred on cancellation or close; resynchronize externally after failure.
    handle.close(false)?;
    script.assert_complete()?;
    println!("PASS compiled TDS1002 static composition example and five typed scope calls");
    Ok(())
}
