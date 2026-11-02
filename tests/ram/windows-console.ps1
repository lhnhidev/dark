param([switch]$Helper, [string]$Exe, [string]$Scenario, [string]$Result, [string]$TargetDirectory)
$ErrorActionPreference = 'Stop'
if (-not $Helper) {
    $root = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
    if (-not $TargetDirectory) { $TargetDirectory = Join-Path $root 'target' }
    foreach ($profile in @('debug', 'release')) {
        $exePath = Join-Path $TargetDirectory "$profile/dark.exe"
        if (-not (Test-Path -LiteralPath $exePath)) { throw "Build missing: $exePath" }
        foreach ($case in @('q', 'ctrlc', 'panic', 'qhidden', 'panichidden')) {
            $resultPath = Join-Path ([IO.Path]::GetTempPath()) ("dark-console-" + [guid]::NewGuid() + '.txt')
            try {
                $argsList = @('-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-File', ('"' + $PSCommandPath + '"'), '-Helper', '-Exe', ('"' + $exePath + '"'), '-Scenario', $case, '-Result', ('"' + $resultPath + '"'))
                $helperProcess = Start-Process -FilePath (Join-Path $PSHOME 'powershell.exe') -ArgumentList $argsList -WindowStyle Hidden -PassThru
                if (-not $helperProcess.WaitForExit(90000)) { $helperProcess.Kill(); throw "Helper timed out: $profile $case" }
                if (Test-Path -LiteralPath $resultPath) { "$profile $(Get-Content -LiteralPath $resultPath -Raw)" }
                if ($helperProcess.ExitCode -ne 0) { throw "Console test failed: $profile $case (exit $($helperProcess.ExitCode))" }
            } finally { Remove-Item -LiteralPath $resultPath -ErrorAction SilentlyContinue }
        }
    }
    exit 0
}
$fixturePath = Join-Path ([IO.Path]::GetTempPath()) ("dark-console-fixture-" + [guid]::NewGuid())
try {
New-Item -ItemType Directory -Path $fixturePath | Out-Null
& git -C $fixturePath init -q
if ($LASTEXITCODE -ne 0) { throw 'Could not initialize fixture' }
& git -C $fixturePath config user.name 'Dark Test'
if ($LASTEXITCODE -ne 0) { throw 'Could not configure fixture name' }
& git -C $fixturePath config user.email 'dark-test@example.invalid'
if ($LASTEXITCODE -ne 0) { throw 'Could not configure fixture email' }
[IO.File]::WriteAllText((Join-Path $fixturePath 'sample.txt'), "before`n")
& git -C $fixturePath add sample.txt
if ($LASTEXITCODE -ne 0) { throw 'Could not stage fixture' }
& git -C $fixturePath commit -qm fixture
if ($LASTEXITCODE -ne 0) { throw 'Could not commit fixture' }
[IO.File]::WriteAllText((Join-Path $fixturePath 'sample.txt'), "after`n")
Add-Type -TypeDefinition @'
using System;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;
public static class ConsoleProbe {
 [StructLayout(LayoutKind.Sequential)] struct COORD { public short X,Y; public COORD(short x,short y){X=x;Y=y;} }
 [StructLayout(LayoutKind.Sequential)] struct RECT { public short left,top,right,bottom; }
 [StructLayout(LayoutKind.Sequential)] struct SCREENINFO { public COORD size,cursor; public ushort attributes; public RECT window; public COORD maximum; }
 [StructLayout(LayoutKind.Sequential)] struct CURSOR { public uint size; [MarshalAs(UnmanagedType.Bool)] public bool visible; }
 [StructLayout(LayoutKind.Explicit, Size=20)] struct INPUT { [FieldOffset(0)] public ushort type; [FieldOffset(4)] public int down; [FieldOffset(8)] public ushort repeat; [FieldOffset(10)] public ushort vk; [FieldOffset(12)] public ushort scan; [FieldOffset(14)] public char ch; [FieldOffset(16)] public uint control; }
 [StructLayout(LayoutKind.Sequential)] struct MEM { public uint cb, faults; public UIntPtr peak, working, qpp, qp, qnp, qn, page, peakpage, privateUsage, privateWorking, shared; }
 [DllImport("kernel32.dll", SetLastError=true)] static extern bool AllocConsole();
 [DllImport("kernel32.dll")] static extern bool FreeConsole();
 [DllImport("kernel32.dll")] static extern uint SetErrorMode(uint mode);
 [DllImport("kernel32.dll")] static extern IntPtr GetConsoleWindow();
 [DllImport("user32.dll")] static extern bool ShowWindow(IntPtr h,int n);
 [DllImport("kernel32.dll",CharSet=CharSet.Unicode,SetLastError=true)] static extern IntPtr CreateFile(string n,uint a,uint s,IntPtr sa,uint c,uint flags,IntPtr t);
 [DllImport("kernel32.dll",SetLastError=true)] static extern bool SetStdHandle(int id,IntPtr h);
 [DllImport("kernel32.dll",SetLastError=true)] static extern bool SetHandleInformation(IntPtr h,uint mask,uint flags);
 [DllImport("kernel32.dll",SetLastError=true)] static extern bool GetConsoleMode(IntPtr h,out uint m);
 [DllImport("kernel32.dll",SetLastError=true)] static extern bool SetConsoleMode(IntPtr h,uint mode);
 [DllImport("kernel32.dll",SetLastError=true)] static extern bool GetConsoleCursorInfo(IntPtr h,out CURSOR c);
 [DllImport("kernel32.dll",SetLastError=true)] static extern bool SetConsoleCursorInfo(IntPtr h,ref CURSOR c);
 [DllImport("kernel32.dll",SetLastError=true)] static extern bool GetConsoleScreenBufferInfo(IntPtr h,out SCREENINFO info);
 [DllImport("kernel32.dll",SetLastError=true)] static extern bool SetConsoleWindowInfo(IntPtr h,bool absolute,ref RECT window);
 [DllImport("kernel32.dll",SetLastError=true)] static extern bool SetConsoleScreenBufferSize(IntPtr h,COORD size);
 [DllImport("kernel32.dll",CharSet=CharSet.Unicode,SetLastError=true)] static extern bool WriteConsoleOutputCharacterW(IntPtr h,string text,uint length,COORD pos,out uint written);
 [DllImport("kernel32.dll",CharSet=CharSet.Unicode,SetLastError=true)] static extern bool WriteConsoleW(IntPtr h,string text,uint length,out uint written,IntPtr reserved);
 [DllImport("kernel32.dll",CharSet=CharSet.Unicode,SetLastError=true)] static extern bool ReadConsoleOutputCharacter(IntPtr h,StringBuilder s,uint n,COORD c,out uint read);
 [DllImport("kernel32.dll",SetLastError=true)] static extern bool WriteConsoleInputW(IntPtr h,INPUT[] input,uint n,out uint written);
 [DllImport("kernel32.dll",SetLastError=true)] static extern bool CloseHandle(IntPtr h);
 [DllImport("psapi.dll",SetLastError=true)] static extern bool GetProcessMemoryInfo(IntPtr h,ref MEM m,uint size);
 static void Check(bool ok,string what) { if(!ok) throw new Exception(what+": Win32 error "+Marshal.GetLastWin32Error()); }
 static IntPtr Open(string name) { var h=CreateFile(name,0xc0000000,3,IntPtr.Zero,3,0,IntPtr.Zero); if(h==new IntPtr(-1)) throw new Exception("open "+name+": "+Marshal.GetLastWin32Error()); return h; }
 static string Screen() { var h=Open("CONOUT$"); try { var s=new StringBuilder(161); uint n; Check(ReadConsoleOutputCharacter(h,s,120,new COORD(0,0),out n),"read screen"); return s.ToString().Substring(0,(int)n); } finally { CloseHandle(h); } }
 static string Content() { var h=Open("CONOUT$"); try { SCREENINFO info; Check(GetConsoleScreenBufferInfo(h,out info),"screen size"); int count=info.size.X*2; var s=new StringBuilder(count+1); uint n; Check(ReadConsoleOutputCharacter(h,s,(uint)count,new COORD(0,info.window.top),out n),"read content"); return s.ToString().Substring(0,(int)n); } finally { CloseHandle(h); } }
 static string Status() { var h=Open("CONOUT$"); try { SCREENINFO info; Check(GetConsoleScreenBufferInfo(h,out info),"screen size"); var s=new StringBuilder(info.size.X+1); uint n; Check(ReadConsoleOutputCharacter(h,s,(uint)info.size.X,new COORD(0,info.window.bottom),out n),"read status"); return s.ToString().Substring(0,(int)n); } finally { CloseHandle(h); } }
 static string Body() { var h=Open("CONOUT$"); try { SCREENINFO info; Check(GetConsoleScreenBufferInfo(h,out info),"screen size"); int row=info.window.top+(info.window.bottom-info.window.top+1)/2; var s=new StringBuilder(info.size.X+1); uint n; Check(ReadConsoleOutputCharacter(h,s,(uint)info.size.X,new COORD(info.window.left,(short)row),out n),"read body"); return s.ToString().Substring(0,(int)n); } finally { CloseHandle(h); } }
 static ulong Memory(Process p) { MEM m=new MEM(); m.cb=(uint)Marshal.SizeOf(typeof(MEM)); Check(GetProcessMemoryInfo(p.Handle,ref m,m.cb),"GetProcessMemoryInfo EX2"); return m.privateWorking.ToUInt64(); }
 static void Key(IntPtr input,char c,bool ctrl) { var e=new INPUT { type=1,down=1,repeat=1,vk=(ushort)char.ToUpperInvariant(c),ch=ctrl?'\x03':c,control=ctrl?8u:0u }; uint n; Check(WriteConsoleInputW(input,new[]{e},1,out n)&&n==1,"WriteConsoleInput"); }
 static void MarkHeader() { var h=Open("CONOUT$"); try { uint n; const string marker="RESIZE TEST"; Check(WriteConsoleOutputCharacterW(h,marker,(uint)marker.Length,new COORD(0,0),out n) && n==marker.Length,"mark header"); } finally { CloseHandle(h); } }
 static SCREENINFO Shrink() { var h=Open("CONOUT$"); try { SCREENINFO info; Check(GetConsoleScreenBufferInfo(h,out info),"screen size before resize"); var small=info.window; if(small.left!=0 || small.top!=0 || small.right<3 || small.bottom<3 || info.size.Y==short.MaxValue) throw new Exception("unsupported console geometry"); small.right--; small.bottom--; MarkHeader(); Check(SetConsoleWindowInfo(h,true,ref small),"shrink console window"); Check(SetConsoleScreenBufferSize(h,new COORD(info.size.X,(short)(info.size.Y+1))),"resize console buffer"); return info; } finally { CloseHandle(h); } }
 static void RestoreSize(SCREENINFO info) { var h=Open("CONOUT$"); try { MarkHeader(); Check(SetConsoleScreenBufferSize(h,info.size),"restore console buffer"); var window=info.window; Check(SetConsoleWindowInfo(h,true,ref window),"restore console window"); } finally { CloseHandle(h); } }
 static void WaitScreen(Process p,string text,ref ulong peak,bool body=false) { var until=DateTime.UtcNow.AddSeconds(30); while(DateTime.UtcNow<until) { if(p.HasExited) throw new Exception("process exited with code "+p.ExitCode+" before screen "+text+"; got: "+Content()); peak=Math.Max(peak,Memory(p)); if((body?Body():Content()).Contains(text)) return; Thread.Sleep(20); } throw new Exception("screen not seen: "+text+"; got: "+(body?Body():Content())); }
 static void WaitResize(Process p,ref ulong peak) { var until=DateTime.UtcNow.AddSeconds(30); while(DateTime.UtcNow<until) { if(p.HasExited) throw new Exception("process exited during resize"); peak=Math.Max(peak,Memory(p)); if(!Screen().Contains("RESIZE TEST")) return; Thread.Sleep(20); } throw new Exception("resize did not redraw header"); }
 static void WaitStatus(Process p,string text,ref ulong peak) { var until=DateTime.UtcNow.AddSeconds(30); while(DateTime.UtcNow<until) { if(p.HasExited) throw new Exception("process exited before status "+text); peak=Math.Max(peak,Memory(p)); if(Status().Contains(text)) return; Thread.Sleep(20); } throw new Exception("status not seen: "+text+"; got: "+Status()); }
 static string FullContent() { var h=Open("CONOUT$"); try { SCREENINFO info; Check(GetConsoleScreenBufferInfo(h,out info),"screen size"); int count=info.size.X*(info.window.bottom-info.window.top+1); var s=new StringBuilder(count+1); uint n; Check(ReadConsoleOutputCharacter(h,s,(uint)count,new COORD(0,info.window.top),out n),"read full content"); return s.ToString().Substring(0,(int)n); } finally { CloseHandle(h); } }
 static void WaitDiff(Process p,string text,ref ulong peak) { var until=DateTime.UtcNow.AddSeconds(15); while(DateTime.UtcNow<until) { if(p.HasExited) throw new Exception("process exited before auto refresh"); peak=Math.Max(peak,Memory(p)); if(FullContent().Contains(text)) return; Thread.Sleep(20); } throw new Exception("automatic diff refresh missing: "+text+"; screen: "+FullContent()); }
 public static string Run(string exe,string scenario,bool release,string fixture) {
  // The child inherits this test-only setting; avoid interactive Windows error dialogs.
  SetErrorMode(0x0001u | 0x0002u | 0x8000u);
  FreeConsole(); Check(AllocConsole(),"AllocConsole"); ShowWindow(GetConsoleWindow(),0);
  IntPtr input=Open("CONIN$"), output=Open("CONOUT$");
  SCREENINFO geometry; Check(GetConsoleScreenBufferInfo(output,out geometry),"initial geometry"); if(geometry.window.right-geometry.window.left+1<80 || geometry.window.bottom-geometry.window.top+1<24) throw new Exception("console geometry must be at least 80x24");
  Check(SetHandleInformation(input,1,1) && SetHandleInformation(output,1,1),"inherit console handles");
  Check(SetStdHandle(-10,input) && SetStdHandle(-11,output) && SetStdHandle(-12,output),"SetStdHandle");
  uint initialMode; Check(GetConsoleMode(input,out initialMode),"read initial input mode"); Check(SetConsoleMode(input,initialMode & ~8u),"clear ENABLE_WINDOW_INPUT");
  if(scenario.EndsWith("hidden")) { CURSOR initial; Check(GetConsoleCursorInfo(output,out initial),"read cursor"); initial.visible=false; Check(SetConsoleCursorInfo(output,ref initial),"hide initial cursor"); }
  uint modeBefore, outputModeBefore, written; CURSOR cursorBefore; Check(GetConsoleMode(input,out modeBefore),"initial input mode"); Check(GetConsoleMode(output,out outputModeBefore),"initial output mode"); Check(GetConsoleCursorInfo(output,out cursorBefore),"initial cursor"); const string marker="dark console test marker"; Check(WriteConsoleW(output,marker,(uint)marker.Length,out written,IntPtr.Zero) && written==marker.Length,"write marker");
  Process p=null; try {
   bool panic=scenario.StartsWith("panic"); var start=new ProcessStartInfo(exe,panic?"--self-test-panic":scenario=="ctrlc"?"--debug":""); start.UseShellExecute=false; start.WorkingDirectory=fixture; p=Process.Start(start); ulong peak=0,idle=0; int threads=0;
   WaitScreen(p,"Changes",ref peak);
   WaitScreen(p,"sample.txt",ref peak);
   uint modeRunning; Check(GetConsoleMode(input,out modeRunning),"running input mode"); if((modeRunning & 8u)==0) throw new Exception("ENABLE_WINDOW_INPUT not enabled while running");
   if(scenario=="ctrlc") WaitStatus(p,"Memory (private_working_set):",ref peak);
   if(!panic) {
    // No terminal input: the filesystem notification alone must redraw Review.
    var file=System.IO.Path.Combine(fixture,"sample.txt");
    System.IO.File.WriteAllText(file,"autorefresh_native_write\n"); WaitDiff(p,"autorefresh_native_write",ref peak);
    var temporary=System.IO.Path.Combine(fixture,"replacement.tmp");
    System.IO.File.WriteAllText(temporary,"autorefresh_atomic_save\n"); System.IO.File.Delete(file); System.IO.File.Move(temporary,file); WaitDiff(p,"autorefresh_atomic_save",ref peak);
    System.IO.File.WriteAllText(System.IO.Path.Combine(fixture,"new-file.txt"),"new file\n"); WaitDiff(p,"new-file.txt",ref peak);
    foreach(var item in new[]{Tuple.Create('2',"Edit"),Tuple.Create('3',"Vibe"),Tuple.Create('5',"Models")}) { Key(input,item.Item1,false); WaitScreen(p,item.Item2,ref peak,true); }
    Key(input,'1',false); WaitScreen(p,"Changes",ref peak);
    var originalSize=Shrink(); WaitResize(p,ref peak); RestoreSize(originalSize); WaitScreen(p,"Changes",ref peak);
    for(int i=0;i<5;i++) { Thread.Sleep(100); idle=Math.Max(idle,Memory(p)); peak=Math.Max(peak,idle); }
    p.Refresh(); threads=p.Threads.Count;
    if(release && (idle>=2000000 || peak>=10000000)) throw new Exception("RAM budget exceeded: idle="+idle+" peak_sampled="+peak);
    Key(input,scenario=="ctrlc"?'c':'q',scenario=="ctrlc");
   }
   if(!p.WaitForExit(6000)) throw new Exception("application did not exit");
   if(panic == (p.ExitCode==0)) throw new Exception("unexpected exit code: "+p.ExitCode);
   uint modeAfter, outputModeAfter; CURSOR cursorAfter; Check(GetConsoleMode(input,out modeAfter),"final input mode"); Check(GetConsoleMode(output,out outputModeAfter),"final output mode"); Check(GetConsoleCursorInfo(output,out cursorAfter),"final cursor");
   if(modeBefore!=modeAfter) throw new Exception("console input mode not restored: "+modeBefore+" -> "+modeAfter);
   if(outputModeBefore!=outputModeAfter) throw new Exception("console output mode not restored: "+outputModeBefore+" -> "+outputModeAfter);
   if(cursorBefore.visible!=cursorAfter.visible || cursorBefore.size!=cursorAfter.size) throw new Exception("cursor not restored");
   if(!Screen().StartsWith(marker)) throw new Exception("original screen marker not restored: "+Screen().Replace('\0','.'));
   return scenario+" PASS idle_max_5_samples_private_ws_bytes="+idle+" peak_sampled_private_ws_bytes="+peak+" threads="+threads+" input_mode_before="+modeBefore+" input_mode_running="+modeRunning+" input_mode_after="+modeAfter+" output_mode_before="+outputModeBefore+" output_mode_after="+outputModeAfter+" initial_cursor_visible="+cursorBefore.visible+" exit="+p.ExitCode;
  } finally { if(p!=null) { if(!p.HasExited) { p.Kill(); p.WaitForExit(); } p.Dispose(); } CloseHandle(input); CloseHandle(output); FreeConsole(); }
 }
}
'@
    $message = [ConsoleProbe]::Run($Exe, $Scenario, $Exe -match '[\\/]release[\\/]', $fixturePath)
    [IO.File]::WriteAllText($Result, $message)
    exit 0
} catch {
    [IO.File]::WriteAllText($Result, ('FAIL: ' + $_.Exception.ToString()))
    exit 1
} finally {
    if (Test-Path -LiteralPath $fixturePath) { Remove-Item -LiteralPath $fixturePath -Recurse -Force }
}
