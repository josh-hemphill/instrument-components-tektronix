use instrument_api::*;
use instrument_driver_tektronix::tds1002 as t;
use instrument_scpi::Connector;
use std::sync::Arc;
#[cfg(feature = "async")]
pub trait Host: Connector + instrument_scpi::AsyncConnector + Clone + 'static {}
#[cfg(feature = "async")]
impl<T: Connector + instrument_scpi::AsyncConnector + Clone + 'static> Host for T {}
#[cfg(not(feature = "async"))]
pub trait Host: Connector + Clone + 'static {}
#[cfg(not(feature = "async"))]
impl<T: Connector + Clone + 'static> Host for T {}
pub fn configuration() -> t::Configuration {
    t::Configuration {
        communication_module: t::CommunicationModule::Tds2Cma,
        channel_1_probe: t::Probe::X10,
        channel_2_probe: t::Probe::X1,
        commissioning: t::Commissioning::QualifiedGpibNormalModeConfirmed,
    }
}
pub fn factory<H: Host>(script: &H, native: bool) -> Result<Arc<dyn DriverFactory>> {
    let factory = t::Factory::new(
        vec![t::Binding {
            endpoint: Endpoint {
                scheme: "scpi".into(),
                address: "fixture".into(),
            },
            configuration: configuration(),
        }],
        Arc::new(script.clone()),
    )?;
    #[cfg(feature = "async")]
    let factory = if native {
        factory.with_native_connector(Arc::new(script.clone()))
    } else {
        factory
    };
    #[cfg(not(feature = "async"))]
    let _ = native;
    Ok(Arc::new(factory))
}
