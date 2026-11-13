using InstrumentComponents.Drivers;
using InstrumentComponents.Drivers.Tektronix;
using InstrumentComponents.Scpi;
using InstrumentComponents.TestKit;
internal static class TdsFixture
{
    internal static Tds1002Configuration Configuration=>new(Tds1002CommunicationModule.Tds2Cma,Tds1002Probe.X10,Tds1002Probe.X1,Tds1002Commissioning.QualifiedGpibNormalModeConfirmed);
    internal static IDriverFactory Factory(Script script,bool native)=>Factory(script,native?script:null);
    internal static IDriverFactory Factory(IScpiConnector connector,IAsyncScpiConnector? native)=>new Tds1002Factory([new(new("scpi","fixture"),Configuration)],connector,native);
}
