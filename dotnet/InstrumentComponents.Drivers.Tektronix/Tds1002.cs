using System.Collections.Frozen;
using System.Globalization;
using InstrumentComponents.Drivers.Catalog;
using InstrumentComponents.Scpi;
namespace InstrumentComponents.Drivers.Tektronix;
public enum Tds1002CommunicationModule { Unknown,Tds2Cm,Tds2Cma,Tds2Cmax,Tds2Mem }
public enum Tds1002Probe { Unknown,X1,X10,X100,X1000 }
public enum Tds1002Commissioning { Unknown,QualifiedGpibNormalModeConfirmed }
public sealed record Tds1002Configuration(Tds1002CommunicationModule CommunicationModule,Tds1002Probe Channel1Probe,Tds1002Probe Channel2Probe,Tds1002Commissioning Commissioning);
public sealed record Tds1002Binding(Endpoint Endpoint,Tds1002Configuration Configuration);
/// <summary>Explicit non-B scalar subset. Measurements replace acquisition state
/// and consume status; close releases resources without restoring device state.</summary>
public sealed class Tds1002Factory:IDriverFactory
{
    private readonly FrozenDictionary<(string,string),Core> _routes;
    private readonly IScpiConnector _connector;
    private readonly IAsyncScpiConnector? _native;
    private static DriverException Bad()=>NativeMessages.Error(DriverErrorKind.InvalidArgument,"Require qualified GPIB module, probes and exact endpoint");
    private static DriverException Parse()=>NativeMessages.Error(DriverErrorKind.Parse,"Unexpected non-B TDS1002 scalar body");
    private static DriverException Device()=>NativeMessages.Error(DriverErrorKind.Device,"Scope state or measurement violates commissioned contract");
    private static void EndpointValid(Endpoint value)
    {
        if(value is null||value.Scheme is null||value.Scheme.Any(c=>!char.IsAscii(c))||!value.Scheme.Trim().Equals("scpi",StringComparison.OrdinalIgnoreCase)||string.IsNullOrWhiteSpace(value.Address))throw Bad();
    }
    private static readonly FrozenSet<Operation> Supported=new[]{Operation.CatalogOscilloscopeSetTimebaseScale,Operation.CatalogOscilloscopeReadTimebaseScale,
        Operation.CatalogOscilloscopeSetChannelScale,Operation.CatalogOscilloscopeSetChannelDisplay,Operation.CatalogOscilloscopeSetChannelCoupling,
        Operation.CatalogOscilloscopeRun,Operation.CatalogOscilloscopeStop,Operation.CatalogOscilloscopeSingle,
        Operation.CatalogOscilloscopeMeasureVpp,Operation.CatalogOscilloscopeMeasureFrequency}.ToFrozenSet();
    public Tds1002Factory(IEnumerable<Tds1002Binding> bindings,IScpiConnector connector,IAsyncScpiConnector? nativeConnector=null)
    {
        if(bindings is null||connector is null)throw Bad();var entries=bindings.Take(4097).ToArray();
        if(entries.Length is 0 or >4096)throw Bad();var routes=new Dictionary<(string,string),Core>();
        foreach(var binding in entries)
        {
            if(binding is null)throw Bad();EndpointValid(binding.Endpoint);var configuration=binding.Configuration;
            if(configuration is null||configuration.CommunicationModule is not(Tds1002CommunicationModule.Tds2Cm or Tds1002CommunicationModule.Tds2Cma or Tds1002CommunicationModule.Tds2Cmax)
                ||configuration.Commissioning!=Tds1002Commissioning.QualifiedGpibNormalModeConfirmed)throw Bad();
            var core=new Core(Factor(configuration.Channel1Probe),Factor(configuration.Channel2Probe));
            if(!routes.TryAdd(NativeMessages.Key(binding.Endpoint),core))throw Bad();
        }
        _routes=routes.ToFrozenDictionary();_connector=connector;_native=nativeConnector;
    }
    private static uint Factor(Tds1002Probe probe)=>probe switch{Tds1002Probe.X1=>1,Tds1002Probe.X10=>10,Tds1002Probe.X100=>100,Tds1002Probe.X1000=>1000,_=>throw Bad()};
    public DriverDescriptor Descriptor=>new("tektronix.tds1002",DriverContract.Major,new[]{"TDS1002","TDS 1002"}.Select(m=>new ExactIdentityRule("scpi","TEKTRONIX",m)),Supported);
    public IReadOnlySet<ExecutionMode> ExecutionModes=>NativeMessages.Modes(_native is not null);
    private Core Selected(DriverOpenContext context)
    {
        EndpointValid(context.Candidate.Endpoint);if(!Enum.IsDefined(context.Ownership))throw Bad();
        if(!_routes.TryGetValue(NativeMessages.Key(context.Candidate.Endpoint),out var core))throw Bad();
        if(context.RequiredOperations is{} required&&!required.IsSubsetOf(Supported))throw NativeMessages.Error(DriverErrorKind.UnsupportedOperation,"Unsupported TDS1002 operation");
        return core;
    }
    public IDeviceBackend Open(DriverOpenContext context){var core=Selected(context);return new MessageDevice(core,_connector.Open(context),context.Ownership);}
    public async ValueTask<IAsyncDeviceBackend> OpenNativeAsync(DriverOpenContext context,CancellationToken cancellationToken=default)
    {
        cancellationToken.ThrowIfCancellationRequested();var core=Selected(context);var native=_native??throw NativeMessages.Error(DriverErrorKind.UnsupportedExecutionMode,"Native connector not configured");
        return new AsyncMessageDevice(core,await native.OpenAsync(context,cancellationToken).ConfigureAwait(false),context.Ownership);
    }
    private static readonly double[] Timebases=[5e-9,1e-8,2.5e-8,5e-8,1e-7,2.5e-7,5e-7,1e-6,2.5e-6,5e-6,1e-5,2.5e-5,5e-5,1e-4,2.5e-4,5e-4,1e-3,2.5e-3,5e-3,1e-2,2.5e-2,5e-2,1e-1,2.5e-1,5e-1,1,2.5,5,10,25,50];
    private static string Real12(double value)
    {
        var text=value.ToString("E11",CultureInfo.InvariantCulture);var marker=text.IndexOf('E');var exponent=int.Parse(text.AsSpan(marker+1),CultureInfo.InvariantCulture);
        return text[..(marker+1)]+(exponent<0?"-":"+")+Math.Abs(exponent).ToString("00",CultureInfo.InvariantCulture);
    }
    private static uint Integer(string body,uint max)
    {
        var digits=body.StartsWith('+')?body[1..]:body;if(digits.Length==0||digits.Any(c=>c is <'0' or >'9'))throw Parse();
        digits=digits.TrimStart('0');var value=digits.Length==0?0:uint.TryParse(digits,NumberStyles.None,CultureInfo.InvariantCulture,out var parsed)?parsed:throw Parse();
        if(value>max)throw Parse();return value;
    }
    private static double Nr3(string body,bool measurement)
    {
        var i=0;if(body.Length>0&&body[0] is '+' or '-')i++;var start=i;
        while(i<body.Length&&char.IsAsciiDigit(body[i]))i++;if(i==start)throw Parse();
        if(i<body.Length&&body[i]=='.'){i++;var fraction=i;while(i<body.Length&&char.IsAsciiDigit(body[i]))i++;if(i==fraction)throw Parse();}
        var mantissaEnd=i;if(i==body.Length||body[i] is not('E' or 'e'))throw Parse();i++;
        if(i<body.Length&&body[i] is '+' or '-')i++;var exponent=i;while(i<body.Length&&char.IsAsciiDigit(body[i]))i++;
        if(i!=body.Length||i-exponent is <1 or >3||!double.TryParse(body,NumberStyles.Float,CultureInfo.InvariantCulture,out var value))throw Parse();
        if(measurement&&value==9.9e37)throw Device();
        if(!double.IsFinite(value)||Math.Abs(value)>=1e30||(value==0&&body[start..mantissaEnd].Any(c=>c is >='1' and <='9')))throw Parse();
        return value;
    }
    private static CatalogReply? ExpectedInteger(string body,uint max,uint expected){if(Integer(body,max)!=expected)throw Device();return null;}
    private static CatalogReply? Token(string body,string expected,params string[] known)
    {if(body!=expected)throw known.Contains(body,StringComparer.Ordinal)?Device():Parse();return null;}
    private static CatalogReply? ProbeValue(string body,uint expected)
    {var value=Integer(body,1000);if(value is not(1 or 10 or 100 or 1000))throw Parse();if(value!=expected)throw Device();return null;}
    private sealed class Core(uint channel1,uint channel2):IMessageEngine
    {
        public IReadOnlySet<Operation> Operations=>Supported;
        private uint Probe(uint channel)=>channel switch{1=>channel1,2=>channel2,_=>throw Bad()};
        private static MessageStep Query(string message,Func<string,CatalogReply?> check)=>new(message,check);
        private static MessageStep Write(string message)=>new(message);
        private static MessageStep Main()=>Query(":HOR:VIEW?",b=>Token(b,"MAIN","MAIN","WINDOW","ZONE"));
        private List<MessageStep> Measure(uint channel,bool frequency)
        {
            var probe=Probe(channel);var source=$"CH{channel}";var kind=frequency?"FREQ":"PK2PK";
            return[Main(),Query(":DISP:FORM?",b=>Token(b,"YT","YT","XY")),Query(":TRIG:MAIN:MODE?",b=>Token(b,"NORMAL","NORMAL","AUTO")),
                Query($":SEL:CH{channel}?",b=>Token(b,"1","0","1")),Query($":CH{channel}:PRO?",b=>ProbeValue(b,probe)),
                Write(":ACQ:STATE STOP"),Write(":ACQ:MODE SAMPLE"),Query(":ACQ:MODE?",b=>Token(b,"SAMPLE","SAMPLE","PEAKDETECT","AVERAGE")),
                Write(":ACQ:STOPA SEQ"),Query(":ACQ:STOPA?",b=>Token(b,"SEQUENCE","SEQUENCE","RUNSTOP")),Write(":ACQ:STATE RUN"),
                Query("*OPC?",b=>b=="1"?null:throw Parse()),Query(":ACQ:STATE?",b=>Token(b,"0","0","1")),Query(":ACQ:NUMACQ?",b=>ExpectedInteger(b,2147483647,1)),
                Write($":MEASU:IMM:SOU {source}"),Write($":MEASU:IMM:TYP {kind}"),Query(":MEASU:IMM:SOU?",b=>Token(b,source,"CH1","CH2")),
                Query(":MEASU:IMM:TYP?",b=>Token(b,frequency?"FREQUENCY":"PK2PK","FREQUENCY","MEAN","PERIOD","PK2PK","CRMS","MINIMUM","MAXIMUM","RISE","FALL","PWIDTH","NWIDTH")),
                Query(":MEASU:IMM:UNI?",b=>Token(b,frequency?"\"Hz\"":"\"V\"","\"V\"","\"s\"","\"Hz\"")),
                Query(":MEASU:IMM:VAL?",b=>{var value=Nr3(b,true);if(value<0||(frequency&&value<=0))throw Device();return new CatalogReply.Numeric(new(value,frequency?Unit.Hertz:Unit.Volt));})];
        }
        public IReadOnlyList<MessageStep> Prepare(CatalogRequest request)
        {
            List<MessageStep> actions;
            switch(request)
            {
                case OscilloscopeSetTimebaseScaleRequest p:
                    if(!Timebases.Contains(p.SecondsPerDiv))throw Bad();actions=[Main(),Write(":HOR:MAIN:SCA "+Real12(p.SecondsPerDiv))];break;
                case OscilloscopeReadTimebaseScaleRequest:
                    actions=[Main(),Query(":HOR:MAIN:SCA?",b=>{var value=Nr3(b,false);if(!Timebases.Contains(value))throw Device();return new CatalogReply.Numeric(new(value,Unit.Second));})];break;
                case OscilloscopeSetChannelScaleRequest p:
                    var probe=Probe(p.Channel);if(!new uint[]{2,5,10,20,50,100,200,500,1000,2000,5000}.Any(m=>(double)(m*probe)/1000==p.VoltsPerDiv))throw Bad();
                    actions=[Query($":CH{p.Channel}:PRO?",b=>ProbeValue(b,probe)),Write($":CH{p.Channel}:SCA "+Real12(p.VoltsPerDiv))];break;
                case OscilloscopeSetChannelDisplayRequest p:
                    Probe(p.Channel);actions=[Write($":SEL:CH{p.Channel} "+(p.Enabled?"ON":"OFF"))];break;
                case OscilloscopeSetChannelCouplingRequest p:
                    Probe(p.Channel);if(p.Coupling is not("AC" or "DC" or "GND"))throw Bad();actions=[Write($":CH{p.Channel}:COUP {p.Coupling}")];break;
                case OscilloscopeRunRequest:actions=[Write(":ACQ:STOPA RUNSTOP"),Write(":ACQ:STATE RUN")];break;
                case OscilloscopeStopRequest:actions=[Write(":ACQ:STATE STOP")];break;
                case OscilloscopeSingleRequest:actions=[Write(":ACQ:STOPA SEQ"),Write(":ACQ:STATE RUN")];break;
                case OscilloscopeMeasureVppRequest p:actions=Measure(p.Channel,false);break;
                case OscilloscopeMeasureFrequencyRequest p:actions=Measure(p.Channel,true);break;
                default:throw NativeMessages.Error(DriverErrorKind.UnsupportedOperation,"Unsupported TDS1002 operation");
            }
            List<MessageStep> steps=[Write(":HEAD OFF"),Write(":VERB ON"),Query(":DESE?",b=>ExpectedInteger(b,255,255)),Query("*ESR?",b=>ExpectedInteger(b,255,0))];
            steps.AddRange(actions);steps.Add(Query("*ESR?",b=>ExpectedInteger(b,255,0)));return steps;
        }
    }
}
