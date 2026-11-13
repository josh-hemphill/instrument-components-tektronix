using InstrumentComponents.Drivers;
using InstrumentComponents.Drivers.Catalog;
using InstrumentComponents.Drivers.Tektronix;
using InstrumentComponents.Runtime;
using InstrumentComponents.TestKit;
internal static class TdsExamples
{
    internal static void Run()
    {
        var ids=new[]{"timebase-nominal-16","channel-1-scale-100mv","channel-1-display-True","channel-1-coupling-DC","channel-1-fresh-Frequency"};
        var cases=GeneratedDrivers.GeneratedCases.Create().ToDictionary(c=>c.Id,StringComparer.Ordinal);
        var steps=ids.SelectMany(id=>cases["tektronix.tds1002|E|"+id].Script.Where(s=>s.Kind!="close")).Append(ScriptStep.Close());
        var script=new Script(steps);
        // Application code begins here. Replace Script with the commissioned
        // LF/EOI,256-byte,finite-deadline connector. Require a CM/CMA/CMAX GPIB
        // module, correct probes/main/YT/normal trigger, DESE255, clean status,
        // Trigger View off and exclusive stable control for the handle lifetime.
        var connector=script;var endpoint=new Endpoint("scpi","fixture");
        var configuration=new Tds1002Configuration(Tds1002CommunicationModule.Tds2Cma,Tds1002Probe.X10,Tds1002Probe.X1,Tds1002Commissioning.QualifiedGpibNormalModeConfirmed);
        var factory=new Tds1002Factory([new(endpoint,configuration)],connector);
        var registry=new DriverRegistry([factory]);
        using var handle=registry.Open(new(endpoint,new InstrumentComponents.Drivers.DeviceIdentity("TEKTRONIX","TDS1002")));
        var scope=new Oscilloscope(handle);scope.SetTimebaseScale(1e-3);scope.SetChannelScale(1,1);
        scope.SetChannelDisplay(1,true);scope.SetChannelCoupling(1,"DC");
        var frequency=scope.MeasureFrequency(1);if(frequency.Unit!=Unit.Hertz)throw new InvalidOperationException("Frequency unit");
        // Fresh scalar consumes ESR/event history, replaces acquisition mode and
        // ends stopped SAMPLE/SEQUENCE. HEAD OFF/VERB ON persist. Close restores
        // none of these states and is not an electrical/safe-shutdown command.
        handle.Close();script.AssertComplete();
        Console.WriteLine("PASS compiled TDS1002 static composition example and five typed scope calls");
    }
}
