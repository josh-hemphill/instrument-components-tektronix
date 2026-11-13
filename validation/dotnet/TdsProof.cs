using System.Globalization;
using InstrumentComponents.Drivers;
using InstrumentComponents.Drivers.Catalog;
using InstrumentComponents.Drivers.Tektronix;
using InstrumentComponents.Runtime;
using InstrumentComponents.TestKit;
internal static class TdsProof
{
    private static DeviceCandidate Candidate=>new(new("scpi","fixture"),new());
    private static DriverSelection Selection=>new(DriverId:"tektronix.tds1002");
    private static Tds1002Binding Binding=>new(Candidate.Endpoint,TdsFixture.Configuration);
    private static void Require(bool value,string text){if(!value)throw new InvalidOperationException(text);}
    private static void Error(DriverErrorKind kind,Action action){try{action();throw new InvalidOperationException("Missing "+kind);}catch(DriverException e){Require(e.Kind==kind,"Expected "+kind+", got "+e.Kind);}}
    private static void Invoke(DeviceHandle handle,CatalogCase test){for(var i=0;i<test.Calls.Count;i++){var reply=handle.Invoke(test.Calls[i]);if(i+1==test.Calls.Count)Require(Equals(reply,test.Expected),test.Id);}}
    internal static void Run()
    {
        var previous=CultureInfo.CurrentCulture;CultureInfo.CurrentCulture=CultureInfo.GetCultureInfo("fr-FR");
        try{Synchronous();}finally{CultureInfo.CurrentCulture=previous;}
    }
    private static void Synchronous()
    {
        var script=new Script([]);
        Tds1002Factory Build(IEnumerable<Tds1002Binding> bindings)=>new(bindings,script);
        Error(DriverErrorKind.InvalidArgument,()=>Build([]));
        foreach(var module in new[]{Tds1002CommunicationModule.Unknown,Tds1002CommunicationModule.Tds2Mem,(Tds1002CommunicationModule)99})Error(DriverErrorKind.InvalidArgument,()=>Build([Binding with{Configuration=TdsFixture.Configuration with{CommunicationModule=module}}]));
        foreach(var probe in new[]{Tds1002Probe.Unknown,(Tds1002Probe)99})foreach(var channel in new[]{1,2})Error(DriverErrorKind.InvalidArgument,()=>Build([Binding with{Configuration=channel==1?TdsFixture.Configuration with{Channel1Probe=probe}:TdsFixture.Configuration with{Channel2Probe=probe}}]));
        foreach(var commissioning in new[]{Tds1002Commissioning.Unknown,(Tds1002Commissioning)99})Error(DriverErrorKind.InvalidArgument,()=>Build([Binding with{Configuration=TdsFixture.Configuration with{Commissioning=commissioning}}]));
        foreach(var scheme in new[]{"hpib","usb","\u00a0scpi","scpi\u2003"})Error(DriverErrorKind.InvalidArgument,()=>Build([Binding with{Endpoint=new(scheme,"fixture")} ]));
        Error(DriverErrorKind.InvalidArgument,()=>Build([Binding with{Endpoint=new("scpi"," \t")} ]));
        Error(DriverErrorKind.InvalidArgument,()=>Build([Binding,Binding with{Endpoint=new(" \tSCPI ","fixture")} ]));
        var valid=Enumerable.Range(0,4096).Select(i=>Binding with{Endpoint=new("scpi","route"+i)}).ToArray();Build(valid);Error(DriverErrorKind.InvalidArgument,()=>Build(valid.Append(Binding)));
        foreach(var module in new[]{Tds1002CommunicationModule.Tds2Cm,Tds1002CommunicationModule.Tds2Cma,Tds1002CommunicationModule.Tds2Cmax})Build([Binding with{Configuration=TdsFixture.Configuration with{CommunicationModule=module}}]);
        var factory=Build([Binding]);Require(factory.Descriptor.Operations.Count==10,"Exact operation subset");var registry=new DriverRegistry([factory]);
        foreach(var model in new[]{"TDS1002","TDS 1002"})Require(registry.Select(Candidate with{Identity=new("tektronix",model)}).DriverId=="tektronix.tds1002","Passive identity");
        foreach(var model in new[]{"TDS1002A","TDS1002B","TDS2002"})Error(DriverErrorKind.DriverNotFound,()=>registry.Select(Candidate with{Identity=new("TEKTRONIX",model)}));
        Error(DriverErrorKind.InvalidArgument,()=>registry.Open(Candidate with{Endpoint=new("scpi","unbound")},Selection));
        Error(DriverErrorKind.UnsupportedOperation,()=>registry.Open(Candidate,Selection,[Operation.Reset]));
        Error(DriverErrorKind.InvalidArgument,()=>factory.Open(new(Candidate,(ConnectionOwnership)99)));
        Require(script.Opens==0&&script.Events.Count==0,"Admission accessed IO");
        var cases=GeneratedDrivers.GeneratedCases.Create();var normal=cases.Single(c=>c.Id=="tektronix.tds1002|E|acquisition-Stop");
        foreach(var test in cases.Where(c=>c.Id.StartsWith("tektronix.tds1002|",StringComparison.Ordinal)&&c.ExpectedError is not null))
        {
            var hadIo=test.Script.Any(s=>s.Kind!="close");var steps=test.Script.Where(s=>s.Kind!="close").Concat(hadIo?[ScriptStep.Close()]:normal.Script);var host=new Script(steps);
            var handle=new DriverRegistry([TdsFixture.Factory(host,false)]).Open(Candidate,Selection);
            for(var i=0;i<test.Calls.Count;i++){if(i+1==test.Calls.Count)Error(test.ExpectedError!.Value,()=>handle.Invoke(test.Calls[i]));else handle.Invoke(test.Calls[i]);}
            if(hadIo){var events=host.Events.Count;Error(DriverErrorKind.SessionNeedsRecovery,()=>new Oscilloscope(handle).Stop());Require(host.Events.Count==events,"Failure retried IO");}else Invoke(handle,normal);
            handle.Close();host.AssertComplete();Require(host.Closes==1,"Owned failure lifetime");
        }
        script=new(normal.Script);var mutable=new List<Tds1002Binding>{Binding};factory=new(mutable,script);mutable[0]=Binding with{Endpoint=new("scpi","injected"),Configuration=TdsFixture.Configuration with{Channel1Probe=Tds1002Probe.X1000}};mutable.Clear();
        var h=new DriverRegistry([factory]).Open(Candidate,Selection);var before=script.Events.Count;
        foreach(var test in cases.Where(c=>c.Id.StartsWith("tektronix.tds1002|E|timebase-nominal-",StringComparison.Ordinal)))
        {var value=((OscilloscopeSetTimebaseScaleRequest)test.Calls[0]).SecondsPerDiv;foreach(var neighbor in new[]{Math.BitIncrement(value),Math.BitDecrement(value)})Error(DriverErrorKind.InvalidArgument,()=>new Oscilloscope(h).SetTimebaseScale(neighbor));}
        foreach(var value in new[]{double.NaN,double.PositiveInfinity,double.NegativeInfinity,0,-1})foreach(var request in new CatalogRequest[]{new OscilloscopeSetTimebaseScaleRequest(value),new OscilloscopeSetChannelScaleRequest(1,value)})Error(DriverErrorKind.InvalidArgument,()=>h.Invoke(request));
        foreach(var request in new CatalogRequest[]{new OscilloscopeSetChannelCouplingRequest(1,"bad"),new OscilloscopeMeasureVppRequest(0),new OscilloscopeMeasureFrequencyRequest(3)})Error(DriverErrorKind.InvalidArgument,()=>h.Invoke(request));
        Require(script.Events.Count==before,"Structural rejection emitted prefix");Invoke(h,normal);h.Close();h.Close();Error(DriverErrorKind.SessionClosed,()=>new Oscilloscope(h).Stop());script.AssertComplete();
        script=new(normal.Script.Where(s=>s.Kind!="close"));h=new DriverRegistry([TdsFixture.Factory(script,false)]).Open(Candidate,Selection,ownership:ConnectionOwnership.Borrowed);Invoke(h,normal);h.Close();h.Close();script.AssertComplete();Require(script.Closes==0,"Borrowed host disposed");
        ProbeGrids();EndpointProbes();
        Console.WriteLine("PASS TDS1002 actual-factory admission,4096 bounds, immutable bindings, all error health follow-ups, nonfinite usable rejection, ownership, invariant culture and all probe grids");
    }
    private static void EndpointProbes()
    {
        var steps=new List<ScriptStep>();
        foreach(var(factor,wire)in new[]{("10","2.00000000000E-02"),("100","2.00000000000E-01")})steps.AddRange([ScriptStep.Write(":HEAD OFF"),ScriptStep.Write(":VERB ON"),ScriptStep.Query(":DESE?","255"),ScriptStep.Query("*ESR?","0"),ScriptStep.Query(":CH1:PRO?",factor),ScriptStep.Write(":CH1:SCA "+wire),ScriptStep.Query("*ESR?","0"),ScriptStep.Close()]);
        var script=new Script(steps);var factory=new Tds1002Factory([Binding,new(new("scpi","other"),TdsFixture.Configuration with{Channel1Probe=Tds1002Probe.X100})],script);
        var registry=new DriverRegistry([factory]);var first=registry.Open(Candidate,Selection);new Oscilloscope(first).SetChannelScale(1,.02);first.Close();
        var second=registry.Open(Candidate with{Endpoint=new("scpi","other")},Selection);var before=script.Events.Count;
        Error(DriverErrorKind.InvalidArgument,()=>new Oscilloscope(second).SetChannelScale(1,.02));Require(script.Events.Count==before,"Endpoint inherited other probe");
        new Oscilloscope(second).SetChannelScale(1,.2);second.Close();script.AssertComplete();Require(script.Opens==2&&script.Closes==2,"Endpoint lifetime");
    }
    private static void ProbeGrids()
    {
        foreach(var(probe,factor)in new[]{(Tds1002Probe.X1,1u),(Tds1002Probe.X10,10u),(Tds1002Probe.X100,100u),(Tds1002Probe.X1000,1000u)})foreach(uint channel in new uint[]{1,2})
        {
            var values=new uint[]{2,5,10,20,50,100,200,500,1000,2000,5000}.Select(m=>(double)(m*factor)/1000).ToArray();var steps=new List<ScriptStep>();
            foreach(var value in values)
            {
                var raw=value.ToString("E11",CultureInfo.InvariantCulture);var marker=raw.IndexOf('E');var exponent=int.Parse(raw.AsSpan(marker+1),CultureInfo.InvariantCulture);var text=raw[..(marker+1)]+(exponent<0?"-":"+")+Math.Abs(exponent).ToString("00",CultureInfo.InvariantCulture);
                steps.AddRange([ScriptStep.Write(":HEAD OFF"),ScriptStep.Write(":VERB ON"),ScriptStep.Query(":DESE?","255"),ScriptStep.Query("*ESR?","0"),ScriptStep.Query($":CH{channel}:PRO?",factor.ToString(CultureInfo.InvariantCulture)),ScriptStep.Write($":CH{channel}:SCA {text}"),ScriptStep.Query("*ESR?","0")]);
            }
            steps.Add(ScriptStep.Close());var script=new Script(steps);var config=channel==1?TdsFixture.Configuration with{Channel1Probe=probe}:TdsFixture.Configuration with{Channel2Probe=probe};
            var mutable=new List<Tds1002Binding>{new(Candidate.Endpoint,config)};var factory=new Tds1002Factory(mutable,script);mutable[0]=Binding with{Configuration=TdsFixture.Configuration with{Channel1Probe=Tds1002Probe.Unknown}};mutable.Clear();
            var handle=new DriverRegistry([factory]).Open(Candidate,Selection);var scope=new Oscilloscope(handle);
            foreach(var value in values){var before=script.Events.Count;foreach(var neighbor in new[]{Math.BitIncrement(value),Math.BitDecrement(value)})Error(DriverErrorKind.InvalidArgument,()=>scope.SetChannelScale(channel,neighbor));Require(script.Events.Count==before,"Neighbor emitted IO");scope.SetChannelScale(channel,value);}
            handle.Close();script.AssertComplete();
        }
    }
    internal static async Task AdmissionAsync()
    {
        var script=new Script([]);var registry=new DriverRegistry([TdsFixture.Factory(script,false)]);
        try{await registry.OpenNativeAsync(Candidate,Selection);throw new InvalidOperationException("Absent native accepted");}catch(DriverException e){Require(e.Kind==DriverErrorKind.UnsupportedExecutionMode,"Native absence kind");}
        registry=new([TdsFixture.Factory(script,true)]);
        foreach(var native in new[]{false,true})try{if(native)await registry.OpenNativeAsync(Candidate,Selection,[Operation.Reset]);else await registry.OpenWorkerAsync(Candidate,Selection,[Operation.Reset]);throw new InvalidOperationException("Unsupported required accepted");}catch(DriverException e){Require(e.Kind==DriverErrorKind.UnsupportedOperation,"Unsupported required kind");}
        Require(script.Opens==0&&script.Events.Count==0,"Mode/required acquisition");
    }
}
