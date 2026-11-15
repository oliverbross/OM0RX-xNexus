// SPDX-License-Identifier: GPL-3.0-only
// Membership observation only. No limits, termination API, breakaway or kill-on-close.
// Atomic assignment: https://learn.microsoft.com/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute
// Membership scope: ordinary CreateProcess descendants, not WMI/service launches.
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text;

namespace Xnexus.InstallerObservation {
    public sealed class Observer : IDisposable {
        [StructLayout(LayoutKind.Sequential)] struct BasicLimits {
            public long ProcessTime, JobTime; public uint Flags;
            public UIntPtr MinWorkingSet, MaxWorkingSet; public uint ActiveProcessLimit;
            public UIntPtr Affinity; public uint Priority, Scheduling;
        }
        [StructLayout(LayoutKind.Sequential)] struct IoCounters {
            public ulong ReadOperations, WriteOperations, OtherOperations, ReadBytes, WriteBytes, OtherBytes;
        }
        [StructLayout(LayoutKind.Sequential)] struct ExtendedLimits {
            public BasicLimits Basic; public IoCounters Io;
            public UIntPtr ProcessMemory, JobMemory, PeakProcessMemory, PeakJobMemory;
        }
        [StructLayout(LayoutKind.Sequential)] public struct Accounting {
            public long UserTime, KernelTime, PeriodUserTime, PeriodKernelTime;
            public uint PageFaults, TotalProcesses, ActiveProcesses, TerminatedProcesses;
        }
        [StructLayout(LayoutKind.Sequential, CharSet=CharSet.Unicode)] struct StartupInfo {
            public uint Size; public string Reserved, Desktop, Title;
            public uint X, Y, Width, Height, XChars, YChars, Fill, Flags;
            public ushort Show, ReservedSize; public IntPtr ReservedPointer, Input, Output, Error;
        }
        [StructLayout(LayoutKind.Sequential)] struct StartupInfoEx { public StartupInfo Startup; public IntPtr Attributes; }
        [StructLayout(LayoutKind.Sequential)] struct ProcessInfo { public IntPtr Process, Thread; public uint Pid, Tid; }
        [DllImport("kernel32.dll",SetLastError=true)] static extern IntPtr CreateJobObjectW(IntPtr attributes,IntPtr name);
        [DllImport("kernel32.dll",SetLastError=true)] static extern bool QueryInformationJobObject(IntPtr job,int kind,IntPtr info,uint size,IntPtr returned);
        [DllImport("kernel32.dll",SetLastError=true)] static extern bool InitializeProcThreadAttributeList(IntPtr list,int count,uint flags,ref IntPtr size);
        [DllImport("kernel32.dll",SetLastError=true)] static extern bool UpdateProcThreadAttribute(IntPtr list,uint flags,IntPtr kind,IntPtr value,IntPtr size,IntPtr previous,IntPtr returned);
        [DllImport("kernel32.dll")] static extern void DeleteProcThreadAttributeList(IntPtr list);
        [DllImport("kernel32.dll",CharSet=CharSet.Unicode,SetLastError=true)] static extern bool CreateProcessW(string executable,StringBuilder command,IntPtr pa,IntPtr ta,bool inherit,uint flags,IntPtr environment,string cwd,ref StartupInfoEx startup,out ProcessInfo process);
        [DllImport("kernel32.dll",SetLastError=true)] static extern bool IsProcessInJob(IntPtr process,IntPtr job,out bool result);
        [DllImport("kernel32.dll",SetLastError=true)] static extern uint WaitForSingleObject(IntPtr handle,uint milliseconds);
        [DllImport("kernel32.dll",SetLastError=true)] static extern bool GetExitCodeProcess(IntPtr process,out uint code);
        [DllImport("kernel32.dll",SetLastError=true)] static extern IntPtr OpenProcess(uint access,bool inherit,uint pid);
        [DllImport("kernel32.dll",SetLastError=true)] static extern bool GetProcessTimes(IntPtr process,out long created,out long exited,out long kernel,out long user);
        [DllImport("kernel32.dll",CharSet=CharSet.Unicode,SetLastError=true)] static extern bool QueryFullProcessImageNameW(IntPtr process,uint flags,StringBuilder path,ref uint size);
        [DllImport("kernel32.dll",SetLastError=true)] static extern bool CloseHandle(IntPtr handle);
        [DllImport("kernel32.dll",CharSet=CharSet.Unicode,SetLastError=true)] static extern IntPtr CreateFileW(string path,uint access,uint share,IntPtr security,uint disposition,uint flags,IntPtr template);
        [DllImport("kernel32.dll",CharSet=CharSet.Unicode,SetLastError=true)] static extern uint GetFinalPathNameByHandleW(IntPtr file,StringBuilder path,uint size,uint flags);
        IntPtr job, process;
        public uint Pid { get; private set; }
        public string Executable { get; private set; }
        public string Arguments { get; private set; }
        public DateTime RootCreatedUtc { get; private set; }
        public bool AtomicMembershipVerified { get; private set; }
        static void Check(bool value) { if(!value) throw new Win32Exception(Marshal.GetLastWin32Error()); }
        T Query<T>(int kind) {
            if(job==IntPtr.Zero) throw new ObjectDisposedException("Observer");
            IntPtr memory=Marshal.AllocHGlobal(Marshal.SizeOf(typeof(T)));
            try { Check(QueryInformationJobObject(job,kind,memory,(uint)Marshal.SizeOf(typeof(T)),IntPtr.Zero)); return (T)Marshal.PtrToStructure(memory,typeof(T)); }
            finally { Marshal.FreeHGlobal(memory); }
        }
        public uint LimitFlags { get { return Query<ExtendedLimits>(9).Basic.Flags; } }
        public ulong MemoryLimit { get { return Query<ExtendedLimits>(9).JobMemory.ToUInt64(); } }
        public Accounting Counts { get { return Query<Accounting>(1); } }
        public bool RootExited {
            get { if(process==IntPtr.Zero) throw new ObjectDisposedException("Observer"); uint s=WaitForSingleObject(process,0); if(s==0xffffffff) throw new Win32Exception(Marshal.GetLastWin32Error()); return s==0; }
        }
        public uint RootExitCode { get { if(!RootExited) throw new InvalidOperationException("Root still alive"); uint code; Check(GetExitCodeProcess(process,out code)); return code; } }
        public sealed class Member : IDisposable {
            IntPtr handle;
            public uint Pid { get; private set; }
            public DateTime CreatedUtc { get; private set; }
            public string Executable { get; private set; }
            internal Member(uint pid,IntPtr job) {
                try {
                    handle=OpenProcess(0x100000|0x1000,false,pid);Check(handle!=IntPtr.Zero);
                    bool belongs;Check(IsProcessInJob(handle,job,out belongs));
                    if(!belongs) throw new InvalidOperationException("Observed PID is not an owned member");
                    long created,exited,kernel,user;Check(GetProcessTimes(handle,out created,out exited,out kernel,out user));
                    Pid=pid;CreatedUtc=DateTime.FromFileTimeUtc(created);
                    uint size=32768;StringBuilder path=new StringBuilder((int)size);
                    Check(QueryFullProcessImageNameW(handle,0,path,ref size));Executable=path.ToString();
                } catch { Dispose();throw; }
            }
            public bool Exited { get { if(handle==IntPtr.Zero) throw new ObjectDisposedException("Member");uint state=WaitForSingleObject(handle,0);if(state==0xffffffff) throw new Win32Exception(Marshal.GetLastWin32Error());return state==0; } }
            public uint ExitCode { get { if(!Exited) throw new InvalidOperationException("Member still alive");uint code;Check(GetExitCodeProcess(handle,out code));return code; } }
            public void Dispose() { if(handle!=IntPtr.Zero) { CloseHandle(handle);handle=IntPtr.Zero; } }
        }
        public Member Capture(uint pid) {
            if(job==IntPtr.Zero) throw new ObjectDisposedException("Observer");
            return new Member(pid,job);
        }
        public static string CanonicalFilePath(string path) {
            IntPtr file=CreateFileW(path,0,7,IntPtr.Zero,3,0,IntPtr.Zero);
            if(file==new IntPtr(-1)) throw new Win32Exception(Marshal.GetLastWin32Error());
            try {
                StringBuilder normalized=new StringBuilder(32768);
                uint size=GetFinalPathNameByHandleW(file,normalized,(uint)normalized.Capacity,0);
                if(size==0 || size>=normalized.Capacity) throw new Win32Exception(Marshal.GetLastWin32Error());
                string result=normalized.ToString();
                if(!result.StartsWith("\\\\?\\") || result.StartsWith("\\\\?\\UNC\\",StringComparison.OrdinalIgnoreCase)) throw new InvalidOperationException("Nonlocal observed image");
                return result.Substring(4);
            } finally { CloseHandle(file); }
        }
        public Observer(string executable,string arguments,string directory) {
            if(!System.IO.Path.IsPathRooted(executable) || executable.IndexOf('"')>=0 || arguments==null) throw new ArgumentException("Exact executable and arguments required");
            IntPtr attributes=IntPtr.Zero, value=IntPtr.Zero; bool initialized=false;
            try {
                job=CreateJobObjectW(IntPtr.Zero,IntPtr.Zero); Check(job!=IntPtr.Zero);
                if(LimitFlags!=0 || MemoryLimit!=0) throw new InvalidOperationException("Unexpected Job policy");
                IntPtr size=IntPtr.Zero;
                InitializeProcThreadAttributeList(IntPtr.Zero,1,0,ref size);
                if(size==IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error());
                attributes=Marshal.AllocHGlobal(size); Check(InitializeProcThreadAttributeList(attributes,1,0,ref size)); initialized=true;
                value=Marshal.AllocHGlobal(IntPtr.Size); Marshal.WriteIntPtr(value,job);
                // PROC_THREAD_ATTRIBUTE_JOB_LIST. Assignment and process creation are atomic.
                Check(UpdateProcThreadAttribute(attributes,0,new IntPtr(0x2000d),value,new IntPtr(IntPtr.Size),IntPtr.Zero,IntPtr.Zero));
                StartupInfoEx startup=new StartupInfoEx(); startup.Startup.Size=(uint)Marshal.SizeOf(typeof(StartupInfoEx)); startup.Attributes=attributes;
                ProcessInfo child;
                Check(CreateProcessW(executable,new StringBuilder("\""+executable+"\" "+arguments),IntPtr.Zero,IntPtr.Zero,false,
                    0x00080000|0x08000000,IntPtr.Zero,directory,ref startup,out child));
                process=child.Process; Pid=child.Pid; CloseHandle(child.Thread);
                long created,exited,kernel,user;Check(GetProcessTimes(process,out created,out exited,out kernel,out user));
                RootCreatedUtc=DateTime.FromFileTimeUtc(created);
                bool assigned; Check(IsProcessInJob(process,job,out assigned));
                if(!assigned) throw new InvalidOperationException("Atomic membership unverified; retain process "+Pid);
                AtomicMembershipVerified=true; Arguments=arguments;
                uint capacity=32768; StringBuilder path=new StringBuilder((int)capacity);
                Check(QueryFullProcessImageNameW(process,0,path,ref capacity)); Executable=path.ToString();
                if(!String.Equals(System.IO.Path.GetFullPath(executable),Executable,StringComparison.OrdinalIgnoreCase))
                    throw new InvalidOperationException("Executable identity mismatch; retain process "+Pid);
            } catch(Exception error) {
                // Closing observation handles never terminates any successfully created process.
                uint retained=Pid; Dispose();
                throw new InvalidOperationException("Observer initialization failed; created PID="+retained+"; no process was terminated",error);
            } finally {
                if(initialized) DeleteProcThreadAttributeList(attributes);
                if(attributes!=IntPtr.Zero) Marshal.FreeHGlobal(attributes);
                if(value!=IntPtr.Zero) Marshal.FreeHGlobal(value);
            }
        }
        public uint[] ActivePids() {
            const int capacity=256; int length=8+capacity*IntPtr.Size;
            IntPtr data=Marshal.AllocHGlobal(length);
            try {
                Check(QueryInformationJobObject(job,3,data,(uint)length,IntPtr.Zero));
                int assigned=Marshal.ReadInt32(data,0), count=Marshal.ReadInt32(data,4);
                if(count<0 || count>capacity || assigned>capacity) throw new InvalidOperationException("Process inventory bound exceeded");
                uint[] result=new uint[count];
                for(int i=0;i<count;i++) result[i]=checked((uint)Marshal.ReadIntPtr(data,8+i*IntPtr.Size).ToInt64());
                return result;
            } finally { Marshal.FreeHGlobal(data); }
        }
        public void Dispose() {
            if(process!=IntPtr.Zero) { CloseHandle(process);process=IntPtr.Zero; }
            if(job!=IntPtr.Zero) { CloseHandle(job);job=IntPtr.Zero; }
        }
    }
}
