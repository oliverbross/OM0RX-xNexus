// SPDX-License-Identifier: GPL-3.0-only
// Inert finite-lived parent/child fixture. Never an installer or application UI.
using System;
using System.Diagnostics;
using System.IO;
using System.Threading;
class ObserverFixture {
    static int Main(string[] args) {
        if((args.Length!=2 && args.Length!=3) || (args[0]!="parent" && args[0]!="child") || !Path.IsPathRooted(args[1])) return 2;
        int childExit=args.Length==3 && args[2]=="1" ? 1 : 0;
        string root=args[1];
        if(!File.Exists(Path.Combine(root,"inert-observer-fixture.marker"))) return 3;
        if(args[0]=="child") {
            File.WriteAllText(Path.Combine(root,"child.pid"),Process.GetCurrentProcess().Id.ToString());
            Thread.Sleep(6000);
            File.WriteAllText(Path.Combine(root,"child-exited.txt"),"ordinary child completion");
            return childExit;
        }
        var child=Process.Start(new ProcessStartInfo(Process.GetCurrentProcess().MainModule.FileName,"child \""+root+"\" "+childExit) {UseShellExecute=false,CreateNoWindow=true});
        if(child==null) return 4;
        Thread.Sleep(1200);
        File.WriteAllText(Path.Combine(root,"parent-exited.txt"),"ordinary parent completion before child");
        return 0;
    }
}
