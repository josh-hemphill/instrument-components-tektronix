//! Procedural requirements outside the JSON corpus; actual commissioned factory.
use crate::{generated_cases, tds_fixture};
use instrument_api::{catalog::*, *};
use instrument_driver_tektronix::tds1002 as t;
use instrument_runtime::*;
use instrument_testkit::*;
use std::{collections::BTreeSet, sync::Arc};
fn candidate() -> Candidate {
    Candidate {
        endpoint: Endpoint {
            scheme: "scpi".into(),
            address: "fixture".into(),
        },
        identity: Identity::default(),
    }
}
fn selection() -> Selection {
    Selection {
        driver_id: Some("tektronix.tds1002".into()),
        ..Selection::default()
    }
}
fn factory(bindings: Vec<t::Binding>, script: &Script) -> Result<t::Factory> {
    t::Factory::new(bindings, Arc::new(script.clone()))
}
fn binding() -> t::Binding {
    t::Binding {
        endpoint: candidate().endpoint,
        configuration: tds_fixture::configuration(),
    }
}
fn invalid(result: Result<t::Factory>) {
    assert!(matches!(result,Err(e)if e.kind==ErrorKind::InvalidArgument));
}
fn open(factory: Arc<dyn DriverFactory>, ownership: ConnectionOwnership) -> Result<DeviceHandle> {
    DriverRegistry::new(vec![factory])?.open(
        &candidate(),
        &selection(),
        &BTreeSet::new(),
        ExecutionMode::Sync,
        ownership,
    )
}
fn invoke(handle: &DeviceHandle, case: &CatalogCase) -> Result<()> {
    for (i, request) in case.calls.iter().enumerate() {
        let reply = handle.invoke(request.clone())?;
        if i + 1 == case.calls.len() {
            assert_eq!(reply, case.expected.clone().unwrap());
        }
    }
    Ok(())
}
pub fn run() -> Result<()> {
    let script = Script::new([]);
    let base = binding();
    invalid(factory(vec![], &script));
    for module in [
        t::CommunicationModule::Unknown,
        t::CommunicationModule::Tds2Mem,
    ] {
        let mut b = base.clone();
        b.configuration.communication_module = module;
        invalid(factory(vec![b], &script));
    }
    for probe in [0, 1] {
        let mut b = base.clone();
        if probe == 0 {
            b.configuration.channel_1_probe = t::Probe::Unknown;
        } else {
            b.configuration.channel_2_probe = t::Probe::Unknown;
        }
        invalid(factory(vec![b], &script));
    }
    let mut b = base.clone();
    b.configuration.commissioning = t::Commissioning::Unknown;
    invalid(factory(vec![b], &script));
    for scheme in ["hpib", "usb", "\u{a0}scpi", "scpi\u{2003}"] {
        let mut b = base.clone();
        b.endpoint.scheme = scheme.into();
        invalid(factory(vec![b], &script));
    }
    let mut b = base.clone();
    b.endpoint.address = " \t".into();
    invalid(factory(vec![b], &script));
    let mut b = base.clone();
    b.endpoint.scheme = " \tSCPI ".into();
    invalid(factory(vec![base.clone(), b], &script));
    let valid: Vec<_> = (0..4096)
        .map(|i| t::Binding {
            endpoint: Endpoint {
                address: format!("route{i}"),
                ..base.endpoint.clone()
            },
            ..base.clone()
        })
        .collect();
    factory(valid.clone(), &script)?;
    let mut oversized = valid;
    oversized.push(base.clone());
    invalid(factory(oversized, &script));
    for module in [
        t::CommunicationModule::Tds2Cm,
        t::CommunicationModule::Tds2Cma,
        t::CommunicationModule::Tds2Cmax,
    ] {
        let mut b = base.clone();
        b.configuration.communication_module = module;
        factory(vec![b], &script)?;
    }
    let f: Arc<dyn DriverFactory> = Arc::new(factory(vec![base.clone()], &script)?);
    assert_eq!(f.descriptor().operations.len(), 10);
    let registry = DriverRegistry::new(vec![f])?;
    for model in ["TDS1002", "TDS 1002"] {
        let mut c = candidate();
        c.identity.manufacturer = Some("tektronix".into());
        c.identity.model = Some(model.into());
        assert_eq!(
            registry.select(&c, &Selection::default())?.driver_id,
            "tektronix.tds1002"
        );
    }
    for model in ["TDS1002A", "TDS1002B", "TDS2002"] {
        let mut c = candidate();
        c.identity.manufacturer = Some("TEKTRONIX".into());
        c.identity.model = Some(model.into());
        assert!(registry.select(&c, &Selection::default()).is_err());
    }
    let mut c = candidate();
    c.endpoint.address = "unbound".into();
    assert_eq!(
        registry
            .open(
                &c,
                &selection(),
                &BTreeSet::new(),
                ExecutionMode::Sync,
                ConnectionOwnership::Owned
            )
            .err()
            .unwrap()
            .kind,
        ErrorKind::InvalidArgument
    );
    assert_eq!(
        registry
            .open(
                &candidate(),
                &selection(),
                &BTreeSet::from([Operation::Reset]),
                ExecutionMode::Sync,
                ConnectionOwnership::Owned
            )
            .err()
            .unwrap()
            .kind,
        ErrorKind::UnsupportedOperation
    );
    assert_eq!(script.opens(), 0);
    assert!(script.events().is_empty());
    let cases = generated_cases::cases();
    let normal = cases
        .iter()
        .find(|c| c.id == "tektronix.tds1002|E|acquisition-Stop")
        .unwrap();
    // Every new independent error has a meaningful health follow-up.
    for case in cases
        .iter()
        .filter(|c| c.id.starts_with("tektronix.tds1002|") && c.expected_error.is_some())
    {
        let had_io = case
            .script
            .iter()
            .any(|s| !matches!(s, ScriptStep::Close(_)));
        let mut steps: Vec<_> = case
            .script
            .iter()
            .filter(|s| !matches!(s, ScriptStep::Close(_)))
            .cloned()
            .collect();
        if had_io {
            steps.push(ScriptStep::Close(Ok(())));
        } else {
            steps.extend(normal.script.clone());
        }
        let script = Script::new(steps);
        let handle = open(
            tds_fixture::factory(&script, false)?,
            ConnectionOwnership::Owned,
        )?;
        for (i, request) in case.calls.iter().enumerate() {
            let result = handle.invoke(request.clone());
            if i + 1 == case.calls.len() {
                assert_eq!(
                    result.err().unwrap().kind,
                    case.expected_error.unwrap(),
                    "{}",
                    case.id
                );
            } else {
                result?;
            }
        }
        if had_io {
            let events = script.events().len();
            assert_eq!(
                Oscilloscope(&handle).stop().err().unwrap().kind,
                ErrorKind::SessionNeedsRecovery
            );
            assert_eq!(script.events().len(), events);
        } else {
            invoke(&handle, normal)?;
        }
        handle.close(false)?;
        script.assert_complete()?;
        assert_eq!(script.closes(), 1);
    }
    let script = Script::new(normal.script.clone());
    let mut original = vec![base];
    let f = factory(original.clone(), &script)?;
    original[0].endpoint.address = "changed".into();
    original[0].configuration.channel_1_probe = t::Probe::X1000;
    original.clear();
    let handle = open(Arc::new(f), ConnectionOwnership::Owned)?;
    let before = script.events().len();
    for case in cases
        .iter()
        .filter(|c| c.id.starts_with("tektronix.tds1002|E|timebase-nominal-"))
    {
        if let CatalogRequest::OscilloscopeSetTimebaseScale { seconds_per_div } = case.calls[0] {
            for seconds in [
                f64::from_bits(seconds_per_div.to_bits() + 1),
                f64::from_bits(seconds_per_div.to_bits() - 1),
            ] {
                assert_eq!(
                    Oscilloscope(&handle)
                        .set_timebase_scale(seconds)
                        .err()
                        .unwrap()
                        .kind,
                    ErrorKind::InvalidArgument
                );
            }
        }
    }
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0, -1.0] {
        for request in [
            CatalogRequest::OscilloscopeSetTimebaseScale {
                seconds_per_div: value,
            },
            CatalogRequest::OscilloscopeSetChannelScale {
                channel: 1,
                volts_per_div: value,
            },
        ] {
            assert_eq!(
                handle.invoke(request).err().unwrap().kind,
                ErrorKind::InvalidArgument
            );
        }
    }
    for request in [
        CatalogRequest::OscilloscopeSetChannelCoupling {
            channel: 1,
            coupling: "bad".into(),
        },
        CatalogRequest::OscilloscopeMeasureVpp { channel: 0 },
        CatalogRequest::OscilloscopeMeasureFrequency { channel: 3 },
    ] {
        assert_eq!(
            handle.invoke(request).err().unwrap().kind,
            ErrorKind::InvalidArgument
        );
    }
    assert_eq!(script.events().len(), before);
    invoke(&handle, normal)?;
    handle.close(false)?;
    handle.close(false)?;
    assert_eq!(
        Oscilloscope(&handle).stop().err().unwrap().kind,
        ErrorKind::SessionClosed
    );
    script.assert_complete()?;
    let script = Script::new(
        normal
            .script
            .iter()
            .filter(|s| !matches!(s, ScriptStep::Close(_)))
            .cloned(),
    );
    let handle = open(
        tds_fixture::factory(&script, false)?,
        ConnectionOwnership::Borrowed,
    )?;
    invoke(&handle, normal)?;
    handle.close(false)?;
    handle.close(false)?;
    script.assert_complete()?;
    assert_eq!(script.closes(), 0);
    probe_grids()?;
    endpoint_probes()?;
    println!("PASS TDS1002 actual-factory admission,4096 bounds, immutable bindings, all error health follow-ups, nonfinite usable rejection, ownership and all probe grids");
    Ok(())
}
fn endpoint_probes() -> Result<()> {
    let one = Script::new([
        ScriptStep::Write(":HEAD OFF".into()),
        ScriptStep::Write(":VERB ON".into()),
        ScriptStep::Query(":DESE?".into(), Ok("255".into())),
        ScriptStep::Query("*ESR?".into(), Ok("0".into())),
        ScriptStep::Query(":CH1:PRO?".into(), Ok("10".into())),
        ScriptStep::Write(":CH1:SCA 2.00000000000E-02".into()),
        ScriptStep::Query("*ESR?".into(), Ok("0".into())),
        ScriptStep::Close(Ok(())),
        ScriptStep::Write(":HEAD OFF".into()),
        ScriptStep::Write(":VERB ON".into()),
        ScriptStep::Query(":DESE?".into(), Ok("255".into())),
        ScriptStep::Query("*ESR?".into(), Ok("0".into())),
        ScriptStep::Query(":CH1:PRO?".into(), Ok("100".into())),
        ScriptStep::Write(":CH1:SCA 2.00000000000E-01".into()),
        ScriptStep::Query("*ESR?".into(), Ok("0".into())),
        ScriptStep::Close(Ok(())),
    ]);
    let first = binding();
    let mut second = first.clone();
    second.endpoint.address = "other".into();
    second.configuration.channel_1_probe = t::Probe::X100;
    let f: Arc<dyn DriverFactory> = Arc::new(factory(vec![first, second], &one)?);
    let registry = DriverRegistry::new(vec![f])?;
    let handle = registry.open(
        &candidate(),
        &selection(),
        &BTreeSet::new(),
        ExecutionMode::Sync,
        ConnectionOwnership::Owned,
    )?;
    Oscilloscope(&handle).set_channel_scale(1, 0.02)?;
    handle.close(false)?;
    let mut other = candidate();
    other.endpoint.address = "other".into();
    let handle = registry.open(
        &other,
        &selection(),
        &BTreeSet::new(),
        ExecutionMode::Sync,
        ConnectionOwnership::Owned,
    )?;
    let before = one.events().len();
    assert_eq!(
        Oscilloscope(&handle)
            .set_channel_scale(1, 0.02)
            .err()
            .unwrap()
            .kind,
        ErrorKind::InvalidArgument
    );
    assert_eq!(one.events().len(), before);
    Oscilloscope(&handle).set_channel_scale(1, 0.2)?;
    handle.close(false)?;
    one.assert_complete()?;
    assert_eq!(one.opens(), 2);
    assert_eq!(one.closes(), 2);
    Ok(())
}
fn probe_grids() -> Result<()> {
    for (probe, factor) in [
        (t::Probe::X1, 1u32),
        (t::Probe::X10, 10),
        (t::Probe::X100, 100),
        (t::Probe::X1000, 1000),
    ] {
        for channel in [1, 2] {
            let mut steps = Vec::new();
            for mv in [2u32, 5, 10, 20, 50, 100, 200, 500, 1000, 2000, 5000] {
                let value = f64::from(mv * factor) / 1000.0;
                let raw = format!("{value:.11E}");
                let (m, e) = raw.split_once('E').unwrap();
                steps.extend([
                    ScriptStep::Write(":HEAD OFF".into()),
                    ScriptStep::Write(":VERB ON".into()),
                    ScriptStep::Query(":DESE?".into(), Ok("255".into())),
                    ScriptStep::Query("*ESR?".into(), Ok("0".into())),
                    ScriptStep::Query(format!(":CH{channel}:PRO?"), Ok(factor.to_string())),
                    ScriptStep::Write(format!(
                        ":CH{channel}:SCA {m}E{:+03}",
                        e.parse::<i32>().unwrap()
                    )),
                    ScriptStep::Query("*ESR?".into(), Ok("0".into())),
                ]);
            }
            steps.push(ScriptStep::Close(Ok(())));
            let script = Script::new(steps);
            let mut b = binding();
            if channel == 1 {
                b.configuration.channel_1_probe = probe;
            } else {
                b.configuration.channel_2_probe = probe;
            }
            let handle = open(
                Arc::new(factory(vec![b], &script)?),
                ConnectionOwnership::Owned,
            )?;
            for mv in [2u32, 5, 10, 20, 50, 100, 200, 500, 1000, 2000, 5000] {
                let value = f64::from(mv * factor) / 1000.0;
                let before = script.events().len();
                for neighbor in [
                    f64::from_bits(value.to_bits() + 1),
                    f64::from_bits(value.to_bits() - 1),
                ] {
                    assert_eq!(
                        Oscilloscope(&handle)
                            .set_channel_scale(channel, neighbor)
                            .err()
                            .unwrap()
                            .kind,
                        ErrorKind::InvalidArgument
                    );
                }
                assert_eq!(script.events().len(), before);
                Oscilloscope(&handle).set_channel_scale(channel, value)?;
            }
            handle.close(false)?;
            script.assert_complete()?;
        }
    }
    Ok(())
}
#[cfg(feature = "async")]
pub async fn admission_async() -> Result<()> {
    let script = Script::new([]);
    let registry = DriverRegistry::new(vec![tds_fixture::factory(&script, false)?])?;
    assert_eq!(
        registry
            .open_native(
                &candidate(),
                &selection(),
                &BTreeSet::new(),
                ConnectionOwnership::Owned
            )
            .await
            .err()
            .unwrap()
            .kind,
        ErrorKind::UnsupportedExecutionMode
    );
    let registry = DriverRegistry::new(vec![tds_fixture::factory(&script, true)?])?;
    let required = BTreeSet::from([Operation::Reset]);
    assert_eq!(
        registry
            .open_native(
                &candidate(),
                &selection(),
                &required,
                ConnectionOwnership::Owned
            )
            .await
            .err()
            .unwrap()
            .kind,
        ErrorKind::UnsupportedOperation
    );
    assert_eq!(
        registry
            .open_worker(
                &candidate(),
                &selection(),
                &required,
                ConnectionOwnership::Owned
            )
            .await
            .err()
            .unwrap()
            .kind,
        ErrorKind::UnsupportedOperation
    );
    assert_eq!(script.opens(), 0);
    Ok(())
}
